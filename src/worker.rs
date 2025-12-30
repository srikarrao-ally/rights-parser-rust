use sqlx::PgPool;
use tracing::{error, info};
use uuid::Uuid;
use std::sync::Arc;

pub async fn start_worker(
    db: PgPool,
    pdf_extractor: Arc<crate::pdf_extractor::PDFExtractor>,
    llm_service: Arc<crate::llm_service::LLMService>,
    encryption_service: Arc<crate::encryption::EncryptionService>,
    ipfs_client: Arc<crate::ipfs_client::IPFSClient>,
) {
    info!("🔧 Background worker started");

    loop {
        if let Err(e) = process_pending_jobs(
            &db,
            &pdf_extractor,
            &llm_service,
            &encryption_service,
            &ipfs_client,
        ).await {
            error!("Worker error: {}", e);
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn process_pending_jobs(
    db: &PgPool,
    pdf_extractor: &Arc<crate::pdf_extractor::PDFExtractor>,
    llm_service: &Arc<crate::llm_service::LLMService>,
    encryption_service: &Arc<crate::encryption::EncryptionService>,
    ipfs_client: &Arc<crate::ipfs_client::IPFSClient>,
) -> anyhow::Result<()> {
    let pending_jobs = sqlx::query!(
        "SELECT id, file_path, file_name, file_size, queue_id, callback_url 
         FROM jobs WHERE status = 'pending' ORDER BY created_at ASC LIMIT 5"
    )
    .fetch_all(db)
    .await?;

    for job in pending_jobs {
        info!("🔄 Processing job: {}", job.id);
        if let Some(ref qid) = job.queue_id {
            info!("🔢 Queue ID: {}", qid);
        }
        
        sqlx::query!("UPDATE jobs SET status = 'processing', started_at = NOW() WHERE id = $1", job.id)
            .execute(db)
            .await?;

        match process_job(job.id, &job.file_path, pdf_extractor, llm_service, encryption_service, ipfs_client).await {
            Ok((ipfs_cid, encryption_key, parsed_json)) => {
                let processing_time: Option<i64> = sqlx::query_scalar!(
                    "SELECT EXTRACT(epoch FROM (NOW() - started_at))::bigint * 1000 FROM jobs WHERE id = $1",
                    job.id
                )
                .fetch_one(db)
                .await
                .ok()
                .flatten();

                let processing_time_val = processing_time.unwrap_or(0);

                sqlx::query!(
                    "UPDATE jobs SET status = 'completed', completed_at = NOW(), processing_time_ms = $2,
                     ipfs_cid = $3, encryption_key = $4, parsed_json = $5 WHERE id = $1",
                    job.id, processing_time_val, ipfs_cid, encryption_key, parsed_json
                )
                .execute(db)
                .await?;

                info!("✅ Job completed: {} ({}ms)", job.id, processing_time_val);

                // Send "completed" webhook
                if let Some(callback_url) = job.callback_url {
                    send_webhook_callback(
                        &callback_url,
                        job.queue_id,
                        "completed",
                        Some(&ipfs_cid),
                        Some(&encryption_key),
                        &job.file_name,
                        job.file_size,
                        processing_time_val,
                        Some(&parsed_json),
                        None,
                    ).await;
                } else {
                    info!("⏭️  No callback URL provided - skipping webhook");
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                
                // Determine if it's a processing failure or system error
                let status = if error_msg.contains("extract") || 
                                error_msg.contains("too short") || 
                                error_msg.contains("parse") ||
                                error_msg.contains("LLM") ||
                                error_msg.contains("JSON") {
                    "failed"  // Processing/business logic failure
                } else {
                    "error"   // System/infrastructure error
                };
                
                error!("❌ Job {}: {} - {}", status, job.id, error_msg);
                
                sqlx::query!(
                    "UPDATE jobs SET status = $2, completed_at = NOW(), error_message = $3 WHERE id = $1",
                    job.id, status, error_msg
                )
                .execute(db)
                .await?;

                // Send "failed" or "error" webhook
                if let Some(callback_url) = job.callback_url {
                    send_webhook_callback(
                        &callback_url,
                        job.queue_id,
                        status,
                        None,
                        None,
                        &job.file_name,
                        job.file_size,
                        0,
                        None,
                        Some(&error_msg),
                    ).await;
                }
            }
        }
    }

    Ok(())
}

async fn process_job(
    _job_id: Uuid,
    file_path: &str,
    pdf_extractor: &Arc<crate::pdf_extractor::PDFExtractor>,
    llm_service: &Arc<crate::llm_service::LLMService>,
    encryption_service: &Arc<crate::encryption::EncryptionService>,
    ipfs_client: &Arc<crate::ipfs_client::IPFSClient>,
) -> anyhow::Result<(String, String, serde_json::Value)> {
    let pdf_bytes = tokio::fs::read(file_path).await?;
    
    info!("🔍 Extracting text from PDF");
    let pdf_text = pdf_extractor.extract_text(&pdf_bytes).await?;
    
    if pdf_text.len() < 100 {
        anyhow::bail!("Extracted text too short");
    }
    
    info!("✅ Extracted {} characters", pdf_text.len());

    info!("🤖 Calling LLM for parsing");
    let json_string = llm_service.parse_agreement(&pdf_text).await?;
    
    info!("✅ Got JSON from LLM ({} bytes)", json_string.len());

    let parsed_json: serde_json::Value = serde_json::from_str(&json_string)?;

    info!("🔐 Encrypting JSON");
    let (encrypted_data, encryption_key) = encryption_service.encrypt(&json_string)?;

    info!("📤 Uploading to IPFS");
    let ipfs_cid = ipfs_client.upload(&encrypted_data).await?;

    info!("✅ Uploaded to IPFS: {}", ipfs_cid);

    Ok((ipfs_cid, encryption_key, parsed_json))
}

// Handle completed, failed, and error statuses
async fn send_webhook_callback(
    callback_url: &str,
    queue_id: Option<String>,
    status: &str,  // "completed", "failed", or "error"
    ipfs_cid: Option<&str>,
    encryption_key: Option<&str>,
    file_name: &str,
    file_size: i64,
    processing_time_ms: i64,
    parsed_data: Option<&serde_json::Value>,
    error_message: Option<&str>,
) {
    let bearer_token = "VbwPP5fFpw/16Sm6GygYhc29oLyqMgcUbyAKWUiEf3c=";

    info!("📡 Sending webhook callback to: {}", callback_url);
    if let Some(ref qid) = queue_id {
        info!("🔢 Including queue_id: {}", qid);
    }
    info!("📊 Status: {}", status);

    let payload = if let Some(qid) = queue_id {
        match status {
            "completed" => {
                // Success case - include all data
                serde_json::json!({
                    "queue_id": qid,
                    "status": "completed",
                    "ipfs_cid": ipfs_cid.unwrap_or(""),
                    "ipfs_url": format!("ipfs://{}", ipfs_cid.unwrap_or("")),
                    "encryption_key": encryption_key.unwrap_or(""),
                    "ipfs_gateway_url": format!("https://ipfs.io/ipfs/{}", ipfs_cid.unwrap_or("")),
                    "decrypted_data": parsed_data.unwrap_or(&serde_json::json!({})),
                    "metadata": {
                        "file_name": file_name,
                        "file_size": file_size,
                        "processed_at": chrono::Utc::now().to_rfc3339(),
                        "model_used": "llama3.3:70b-instruct-q4_K_M",
                        "processing_time_ms": processing_time_ms
                    }
                })
            },
            "failed" => {
                // Processing failed - send error details
                serde_json::json!({
                    "queue_id": qid,
                    "status": "failed",
                    "error_message": error_message.unwrap_or("Processing failed"),
                    "metadata": {
                        "file_name": file_name,
                        "file_size": file_size,
                        "processed_at": chrono::Utc::now().to_rfc3339(),
                        "processing_time_ms": processing_time_ms
                    }
                })
            },
            "error" => {
                // System error - send error details
                serde_json::json!({
                    "queue_id": qid,
                    "status": "error",
                    "error_message": error_message.unwrap_or("System error occurred"),
                    "metadata": {
                        "file_name": file_name,
                        "file_size": file_size,
                        "processed_at": chrono::Utc::now().to_rfc3339(),
                        "processing_time_ms": processing_time_ms
                    }
                })
            },
            _ => {
                error!("⚠️  Unknown status: {}", status);
                return;
            }
        }
    } else {
        error!("⚠️  No queue_id provided for webhook - Django requires it");
        return;
    };

    match reqwest::Client::new()
        .post(callback_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                info!("✅ Webhook sent successfully to {}", callback_url);
            } else {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                error!("❌ Webhook failed with status: {} - {}", status, body);
            }
        }
        Err(e) => {
            error!("❌ Failed to send webhook: {}", e);
        }
    }
}