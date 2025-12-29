// src/main.rs - Production API with IPFS-only result
mod models;
mod pdf_extractor;
mod llm_service;
mod encryption;
mod ipfs_client;
mod worker;

use axum::{
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::fs;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use crate::pdf_extractor::PDFExtractor;
use crate::llm_service::LLMService;
use crate::encryption::EncryptionService;
use crate::ipfs_client::IPFSClient;

#[derive(Serialize)]
struct UploadResponse {
    success: bool,
    job_id: String,
    message: String,
    status_url: String,
}

#[derive(Serialize)]
struct StatusResponse {
    job_id: String,
    status: String,
    file_name: String,
    created_at: String,
    completed_at: Option<String>,
    processing_time_ms: Option<i64>,
    ipfs_cid: Option<String>,
    error_message: Option<String>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: String,
    database: bool,
    pending_jobs: i64,
}

#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
    message: String,
    timestamp: String,
}

#[derive(Deserialize)]
struct DecryptQuery {
    key: String,
}

#[derive(Clone)]
struct AppState {
    db: PgPool,
    pdf_extractor: Arc<PDFExtractor>,
    llm_service: Arc<LLMService>,
    encryption_service: Arc<EncryptionService>,
    ipfs_client: Arc<IPFSClient>,
    upload_dir: String,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rights_agreement_parser=info,tower_http=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🚀 Starting Rights Parser API Server (Production)");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let ollama_url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let ollama_model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "rights-parser".to_string());
    let ipfs_url = std::env::var("IPFS_URL").unwrap_or_else(|_| "http://localhost:5001".to_string());
    let pinata_jwt = std::env::var("PINATA_JWT").ok();
    let server_port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse::<u16>().unwrap_or(8080);
    let upload_dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "/workspace/uploads".to_string());

    tokio::fs::create_dir_all(&upload_dir).await.expect("Failed to create upload directory");

    info!("⚙️  Configuration:");
    info!("   Database: Connected");
    info!("   Ollama: {}", ollama_url);
    info!("   IPFS: {}", if pinata_jwt.is_some() { "Pinata" } else { "Local" });
    info!("   Upload Dir: {}", upload_dir);
    info!("   Port: {}", server_port);

    info!("📊 Connecting to database...");
    let db = PgPool::connect(&database_url).await.expect("Failed to connect to database");
    info!("✅ Database ready");

    let pdf_extractor = Arc::new(PDFExtractor::new());
    let llm_service = Arc::new(LLMService::new(ollama_url, ollama_model));
    let encryption_service = Arc::new(EncryptionService::new());
    let ipfs_client = Arc::new(IPFSClient::new(ipfs_url, pinata_jwt));

    let state = AppState {
        db: db.clone(),
        pdf_extractor: pdf_extractor.clone(),
        llm_service: llm_service.clone(),
        encryption_service: encryption_service.clone(),
        ipfs_client: ipfs_client.clone(),
        upload_dir,
    };

    info!("🔧 Starting background worker...");
    tokio::spawn(async move {
        worker::start_worker(db, pdf_extractor, llm_service, encryption_service, ipfs_client).await;
    });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/upload", post(upload_handler))
        .route("/api/v1/status/:job_id", get(status_handler))
        .route("/api/v1/result/:job_id", get(result_handler))
        .route("/api/v1/decrypt/:cid", get(decrypt_handler))
        .with_state(state)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http());

    let addr = format!("0.0.0.0:{}", server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("Failed to bind");

    info!("✅ Server listening on http://{}", addr);
    info!("📖 API Endpoints:");
    info!("   POST /api/v1/upload - Upload PDF (requires API key)");
    info!("   GET  /api/v1/status/:job_id - Check status");
    info!("   GET  /api/v1/result/:job_id - Get IPFS CID and key");
    info!("   GET  /api/v1/decrypt/:cid?key=<key> - Decrypt from IPFS");
    info!("   GET  /health - Health check");
    info!("🔑 Authentication: X-API-Key header required for upload");

    axum::serve(listener, app).await.expect("Server failed");
}

async fn validate_api_key(db: &PgPool, headers: &HeaderMap) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let api_key = headers.get("X-API-Key").and_then(|v| v.to_str().ok())
        .ok_or_else(|| error_response(StatusCode::UNAUTHORIZED, "Missing X-API-Key header"))?;

    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let key_hash = hex::encode(hasher.finalize());

    let record = sqlx::query!("SELECT user_id, is_active FROM api_keys WHERE key_hash = $1 AND is_active = TRUE", key_hash)
        .fetch_optional(db).await.map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
        .ok_or_else(|| error_response(StatusCode::UNAUTHORIZED, "Invalid API key"))?;

    let _ = sqlx::query!("UPDATE api_keys SET last_used_at = NOW(), requests_count = requests_count + 1 WHERE key_hash = $1", key_hash)
        .execute(db).await;

    Ok(record.user_id.unwrap_or_default())
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_healthy = sqlx::query("SELECT 1").fetch_one(&state.db).await.is_ok();
    let pending_jobs: Option<i64> = sqlx::query_scalar!("SELECT COUNT(*) FROM jobs WHERE status = 'pending'")
        .fetch_one(&state.db).await.ok().flatten();

    Json(HealthResponse {
        status: if db_healthy { "healthy" } else { "degraded" }.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        database: db_healthy,
        pending_jobs: pending_jobs.unwrap_or(0),
    })
}

async fn upload_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _user_id = validate_api_key(&state.db, &headers).await?;
    
    info!("📤 Upload request authenticated");

    let mut pdf_bytes: Option<Bytes> = None;
    let mut file_name = String::from("document.pdf");

    while let Some(field) = multipart.next_field().await.map_err(|_| error_response(StatusCode::BAD_REQUEST, "Invalid multipart"))? {
        if field.name() == Some("file") {
            file_name = field.file_name().unwrap_or("document.pdf").to_string();
            pdf_bytes = Some(field.bytes().await.map_err(|_| error_response(StatusCode::BAD_REQUEST, "Failed to read file"))?);
        }
    }

    let pdf_bytes = pdf_bytes.ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "No file provided. Use 'file' field."))?;
    let file_size = pdf_bytes.len() as i64;
    
    info!("📄 Received: {} ({} bytes)", file_name, file_size);

    let job_id = Uuid::new_v4();
    let file_path = format!("{}/{}.pdf", state.upload_dir, job_id);

    fs::write(&file_path, &pdf_bytes).await.map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to save file"))?;

    let api_key = headers.get("X-API-Key").and_then(|v| v.to_str().ok()).unwrap_or("");
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    sqlx::query!(
        "INSERT INTO jobs (id, file_name, file_path, file_size, api_key_hash, status) VALUES ($1, $2, $3, $4, $5, 'pending')",
        job_id, file_name, file_path, file_size, api_key_hash
    ).execute(&state.db).await.map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create job"))?;

    info!("✅ Job created: {}", job_id);

    Ok(Json(UploadResponse {
        success: true,
        job_id: job_id.to_string(),
        message: "PDF uploaded successfully. Processing will begin shortly.".to_string(),
        status_url: format!("/api/v1/status/{}", job_id),
    }))
}

async fn status_handler(State(state): State<AppState>, Path(job_id): Path<Uuid>) -> Result<Json<StatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let job = sqlx::query!("SELECT file_name, status, created_at, completed_at, processing_time_ms, ipfs_cid, error_message FROM jobs WHERE id = $1", job_id)
        .fetch_optional(&state.db).await.map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "Job not found"))?;

    Ok(Json(StatusResponse {
        job_id: job_id.to_string(),
        status: job.status,
        file_name: job.file_name,
        created_at: job.created_at.map(|t| t.to_rfc3339()).unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        completed_at: job.completed_at.map(|t| t.to_rfc3339()),
        processing_time_ms: job.processing_time_ms,
        ipfs_cid: job.ipfs_cid,
        error_message: job.error_message,
    }))
}

async fn result_handler(State(state): State<AppState>, Path(job_id): Path<Uuid>) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let job = sqlx::query!("SELECT file_name, file_size, status, completed_at, processing_time_ms, ipfs_cid, encryption_key, model_used FROM jobs WHERE id = $1", job_id)
        .fetch_optional(&state.db).await.map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "Job not found"))?;

    if job.status != "completed" {
        return Err(error_response(StatusCode::BAD_REQUEST, &format!("Job not completed. Status: {}", job.status)));
    }

    let ipfs_cid = job.ipfs_cid.ok_or_else(|| error_response(StatusCode::INTERNAL_SERVER_ERROR, "IPFS CID not found"))?;
    let encryption_key = job.encryption_key.ok_or_else(|| error_response(StatusCode::INTERNAL_SERVER_ERROR, "Encryption key not found"))?;

    Ok(Json(serde_json::json!({
        "ipfs_cid": ipfs_cid,
        "ipfs_url": format!("ipfs://{}", ipfs_cid),
        "encryption_key": encryption_key,
        "ipfs_gateway_url": format!("https://ipfs.io/ipfs/{}", ipfs_cid),
        "metadata": {
            "file_name": job.file_name,
            "file_size": job.file_size,
            "processed_at": job.completed_at.map(|t| t.to_rfc3339()).unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            "model_used": job.model_used.unwrap_or_else(|| "unknown".to_string()),
            "processing_time_ms": job.processing_time_ms.unwrap_or(0)
        }
    })))
}

async fn decrypt_handler(
    State(state): State<AppState>,
    Path(ipfs_cid): Path<String>,
    Query(params): Query<DecryptQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("🔓 Decrypt request for CID: {}", ipfs_cid);
    
    // Fetch encrypted data from IPFS
    let ipfs_url = format!("https://ipfs.io/ipfs/{}", ipfs_cid);
    let encrypted_data = reqwest::get(&ipfs_url)
        .await
        .map_err(|_| error_response(StatusCode::BAD_GATEWAY, "Failed to fetch from IPFS"))?
        .bytes()
        .await
        .map_err(|_| error_response(StatusCode::BAD_GATEWAY, "Failed to read IPFS data"))?;
    
    // Decrypt with provided key
    let decrypted_json = state.encryption_service.decrypt(&encrypted_data, &params.key)
        .map_err(|e| error_response(StatusCode::BAD_REQUEST, &format!("Decryption failed: {}", e)))?;
    
    // Parse and return JSON
    let parsed: serde_json::Value = serde_json::from_str(&decrypted_json)
        .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "Invalid JSON in decrypted data"))?;
    
    info!("✅ Successfully decrypted CID: {}", ipfs_cid);
    
    Ok(Json(serde_json::json!({
        "success": true,
        "ipfs_cid": ipfs_cid,
        "decrypted_data": parsed
    })))
}

fn error_response(status: StatusCode, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse {
        success: false,
        error: status.to_string(),
        message: message.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }))
}
