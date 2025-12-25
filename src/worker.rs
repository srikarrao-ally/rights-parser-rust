// src/worker.rs - Background worker for processing PDF jobs
use crate::AppState;
use sqlx::PgPool;
use tracing::{error, info, warn};
use uuid::Uuid;

pub async fn start_worker(state: AppState) {
    info!("🔧 Background worker started");

    loop {
        // Process pending jobs
        if let Err(e) = process_pending_jobs(&state).await {
            error!("Worker error: {}", e);
        }

        // Sleep for 5 seconds before next poll
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn process_pending_jobs(state: &AppState) -> anyhow::Result<()> {
    // Fetch pending jobs
    let pending_jobs = sqlx::query!(
        r#"
        SELECT id, file_path, webhook_url
        FROM jobs
        WHERE status = 'pending'
        ORDER BY created_at ASC
        LIMIT 5
        "#
    )
    .fetch_all(&state.db)
    .await?;

    for job in pending_jobs {
        info!("🔄 Processing job: {}", job.id);
        
        // Mark as processing
        sqlx::query!(
            "UPDATE jobs SET status = 'processing', started_at = NOW() WHERE id = $1",
            job.id
        )
        .execute(&state.db)
        .await?;

        // Process the job
        match process_job(state, job.id, &job.file_path).await {
            Ok((ipfs_cid, encryption_key, parsed_json)) => {
                // Update job as completed
                let processing_time = sqlx::query_scalar!(
                    "SELECT EXTRACT(epoch FROM (NOW() - started_at))::bigint * 1000 FROM jobs WHERE id = $1",
                    job.id
                )
                .fetch_one(&state.db)
                .await
                .unwrap_or(0);

                sqlx::query!(
                    r#"
                    UPDATE jobs
                    SET status = 'completed',
                        completed_at = NOW(),
                        processing_time_ms = $2,
                        ipfs_cid = $3,
                        encryption_key = $4,
                        parsed_json = $5
                    WHERE id = $1
                    "#,
                    job.id,
                    processing_time,
                    ipfs_cid,
                    encryption_key,
                    parsed_json
                )
                .execute(&state.db)
                .await?;

                info!("✅ Job completed: {} ({}ms)", job.id, processing_time);

                // Send webhook if configured
                if let Some(webhook_url) = job.webhook_url {
                    tokio::spawn(async move {
                        send_webhook(&webhook_url, job.id, &ipfs_cid, &encryption_key).await;
                    });
                }
            }
            Err(e) => {
                error!("❌ Job failed: {} - {}", job.id, e);
                
                // Mark as failed
                sqlx::query!(
                    r#"
                    UPDATE jobs
                    SET status = 'failed',
                        completed_at = NOW(),
                        error_message = $2,
                        retry_count = retry_count + 1
                    WHERE id = $1
                    "#,
                    job.id,
                    e.to_string()
                )
                .execute(&state.db)
                .await?;
            }
        }
    }

    Ok(())
}

async fn process_job(
    state: &AppState,
    job_id: Uuid,
    file_path: &str,
) -> anyhow::Result<(String, String, serde_json::Value)> {
    // Read PDF file
    let pdf_bytes = tokio::fs::read(file_path).await?;
    
    // Extract text
    info!("🔍 Extracting text from PDF");
    let pdf_text = state.pdf_extractor.extract_text(&pdf_bytes).await?;
    
    if pdf_text.len() < 100 {
        anyhow::bail!("Extracted text too short: {} chars", pdf_text.len());
    }
    
    info!("✅ Extracted {} characters", pdf_text.len());

    // Parse with LLM
    info!("🤖 Calling LLM for parsing");
    let json_string = state.llm_service.parse_agreement(&pdf_text).await?;
    
    info!("✅ Got JSON from LLM ({} bytes)", json_string.len());

    // Parse to validate JSON
    let parsed_json: serde_json::Value = serde_json::from_str(&json_string)?;

    // Encrypt JSON
    info!("🔐 Encrypting JSON");
    let (encrypted_data, encryption_key) = state.encryption_service.encrypt(&json_string)?;

    // Upload to IPFS
    info!("📤 Uploading to IPFS");
    let ipfs_cid = state.ipfs_client.upload(&encrypted_data).await?;

    info!("✅ Uploaded to IPFS: {}", ipfs_cid);

    Ok((ipfs_cid, encryption_key, parsed_json))
}

async fn send_webhook(url: &str, job_id: Uuid, ipfs_cid: &str, encryption_key: &str) {
    let client = reqwest::Client::new();
    
    let payload = serde_json::json!({
        "job_id": job_id.to_string(),
        "status": "completed",
        "ipfs_cid": ipfs_cid,
        "encryption_key": encryption_key,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    match client
        .post(url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) => {
            info!("✅ Webhook sent to {} (status: {})", url, resp.status());
        }
        Err(e) => {
            warn!("⚠️  Webhook failed: {}", e);
        }
    }
}