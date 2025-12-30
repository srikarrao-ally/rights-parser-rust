#!/bin/bash

# Backup current worker.rs
cp src/worker.rs src/worker.rs.backup_webhook

# Create the new send_webhook_callback function
cat > /tmp/new_webhook_func.rs << 'EOFNEW'
async fn send_webhook_callback(
    callback_url: &str,
    queue_id: Option<String>,
    ipfs_cid: &str,
    encryption_key: &str,
    file_name: &str,
    file_size: i64,
    processing_time_ms: i64,
    parsed_data: &serde_json::Value,
) {
    let bearer_token = "VbwPP5fFpw/16Sm6GygYhc29oLyqMgcUbyAKWUiEf3c=";

    info!("📡 Sending webhook callback to: {}", callback_url);
    if let Some(ref qid) = queue_id {
        info!("🔢 Including queue_id: {}", qid);
    }

    // Build the metadata object
    let metadata = serde_json::json!({
        "file_name": file_name,
        "file_size": file_size,
        "processed_at": chrono::Utc::now().to_rfc3339(),
        "model_used": "llama3.3:70b-instruct-q4_K_M",
        "processing_time_ms": processing_time_ms
    });

    // Build the data object (everything except queue_id and status)
    let data = serde_json::json!({
        "ipfs_cid": ipfs_cid,
        "ipfs_url": format!("ipfs://{}", ipfs_cid),
        "encryption_key": encryption_key,
        "ipfs_gateway_url": format!("https://ipfs.io/ipfs/{}", ipfs_cid),
        "decrypted_data": parsed_data,
        "metadata": metadata
    });

    // Build the complete payload with queue_id and status at top level
    let payload = if let Some(qid) = queue_id {
        serde_json::json!({
            "queue_id": qid,
            "status": "completed",
            "data": data
        })
    } else {
        error!("⚠️  No queue_id provided for webhook - Django requires it");
        return;
    };

    info!("📋 Sending webhook with structure: {{queue_id, status, data}}");

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
EOFNEW

echo "✅ New webhook function created"
echo "📝 Please manually replace the send_webhook_callback function in src/worker.rs"
echo "   Starting at line ~148 until the end of the function"
