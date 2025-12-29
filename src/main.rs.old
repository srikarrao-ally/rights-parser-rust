// src/main.rs - Fixed version without await in closures
mod models;
mod pdf_extractor;
mod llm_service;
mod json_builder;
mod encryption;
mod ipfs_client;

use axum::{
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::pdf_extractor::PDFExtractor;
use crate::llm_service::LLMService;
use crate::json_builder::JSONBuilder;
use crate::encryption::EncryptionService;
use crate::ipfs_client::IPFSClient;

// Response structures
#[derive(Serialize, Deserialize)]
struct ParseResponse {
    ipfs_cid: String,
    ipfs_url: String,
    encryption_key: String,
    ipfs_gateway_url: String,
    metadata: FileMetadata,
}

#[derive(Serialize, Deserialize)]
struct FileMetadata {
    file_name: String,
    file_size: u64,
    processed_at: String,
    model_used: String,
    processing_time_ms: u64,
}

#[derive(Deserialize)]
struct DecryptQuery {
    key: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: String,
    services: ServiceHealth,
}

#[derive(Serialize)]
struct ServiceHealth {
    ollama: bool,
    ipfs: bool,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
    timestamp: String,
}

// Shared application state
#[derive(Clone)]
struct AppState {
    pdf_extractor: Arc<PDFExtractor>,
    llm_service: Arc<LLMService>,
    json_builder: Arc<JSONBuilder>,
    encryption_service: Arc<EncryptionService>,
    ipfs_client: Arc<IPFSClient>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rights_agreement_parser=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🚀 Starting Rights Parser API Server");

    // Load configuration from environment
    let ollama_url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    let ollama_model = std::env::var("OLLAMA_MODEL")
        .unwrap_or_else(|_| "rights-parser".to_string());
    let ipfs_url = std::env::var("IPFS_URL")
        .unwrap_or_else(|_| "http://localhost:5001".to_string());
    let pinata_jwt = std::env::var("PINATA_JWT").ok();
    let server_port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    info!("⚙️  Configuration:");
    info!("   Ollama URL: {}", ollama_url);
    info!("   Ollama Model: {}", ollama_model);
    info!("   IPFS URL: {}", ipfs_url);
    info!("   Pinata: {}", if pinata_jwt.is_some() { "Enabled" } else { "Disabled" });
    info!("   Port: {}", server_port);

    // Initialize services
    let pdf_extractor = Arc::new(PDFExtractor::new());
    let llm_service = Arc::new(LLMService::new(ollama_url.clone(), ollama_model.clone()));
    let json_builder = Arc::new(JSONBuilder::new());
    let encryption_service = Arc::new(EncryptionService::new());
    let ipfs_client = Arc::new(IPFSClient::new(ipfs_url, pinata_jwt));

    let state = AppState {
        pdf_extractor,
        llm_service,
        json_builder,
        encryption_service,
        ipfs_client,
    };

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/parse", post(parse_pdf_handler))
        .route("/api/decrypt/:cid", get(decrypt_handler))
        .route("/api/status/:cid", get(status_handler))
        .with_state(state)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = format!("0.0.0.0:{}", server_port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    info!("✅ Server listening on http://{}", addr);
    info!("📖 API Documentation:");
    info!("   POST /api/parse - Upload and parse PDF");
    info!("   GET  /api/decrypt/:cid?key=... - Decrypt and view result");
    info!("   GET  /api/status/:cid - Check IPFS status");
    info!("   GET  /health - Health check");

    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    info!("Health check requested");

    // Check Ollama
    let ollama_healthy = state.llm_service.health_check().await.unwrap_or(false);

    // Check IPFS
    let ipfs_healthy = state.ipfs_client.health_check().await.unwrap_or(false);

    let status = if ollama_healthy && ipfs_healthy {
        "healthy"
    } else {
        "degraded"
    };

    Json(HealthResponse {
        status: status.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        services: ServiceHealth {
            ollama: ollama_healthy,
            ipfs: ipfs_healthy,
        },
    })
}

async fn parse_pdf_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ParseResponse>, (StatusCode, Json<ErrorResponse>)> {
    let start_time = std::time::Instant::now();
    
    info!("📄 Received PDF parsing request");

    // Extract PDF from multipart
    let mut pdf_bytes: Option<Bytes> = None;
    let mut file_name = String::from("document.pdf");

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!("Failed to read multipart field: {}", e);
        error_response(StatusCode::BAD_REQUEST, "Invalid multipart data")
    })? {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            file_name = field
                .file_name()
                .unwrap_or("document.pdf")
                .to_string();
            
            pdf_bytes = Some(field.bytes().await.map_err(|e| {
                error!("Failed to read file bytes: {}", e);
                error_response(StatusCode::BAD_REQUEST, "Failed to read file")
            })?);
        }
    }

    let pdf_bytes = pdf_bytes.ok_or_else(|| {
        error!("No file provided in request");
        error_response(StatusCode::BAD_REQUEST, "No file provided")
    })?;

    let file_size = pdf_bytes.len() as u64;
    info!("📖 Processing PDF: {} ({} bytes)", file_name, file_size);

    // Save to temporary file
    let temp_path = format!("/tmp/{}-{}", 
        chrono::Utc::now().timestamp(), 
        file_name
    );
    
    fs::write(&temp_path, &pdf_bytes)
        .await
        .map_err(|e| {
            error!("Failed to write temp file: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to save file")
        })?;

    // Extract text from PDF
    info!("🔍 Extracting text from PDF");
    let pdf_text = match state.pdf_extractor.extract_text(&pdf_bytes).await {
        Ok(text) => text,
        Err(e) => {
            error!("PDF extraction failed: {}", e);
            let _ = fs::remove_file(&temp_path).await; // Cleanup without await in map_err
            return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to extract text from PDF"));
        }
    };

    if pdf_text.len() < 100 {
        warn!("Extracted text too short: {} chars", pdf_text.len());
        let _ = fs::remove_file(&temp_path).await;
        return Err(error_response(StatusCode::BAD_REQUEST, "Could not extract sufficient text from PDF"));
    }

    info!("✅ Extracted {} characters from PDF", pdf_text.len());

    // Parse with LLM
    info!("🤖 Calling LLM for parsing");
    let json_string = match state.llm_service.parse_agreement(&pdf_text).await {
        Ok(json) => json,
        Err(e) => {
            error!("LLM parsing failed: {}", e);
            let _ = fs::remove_file(&temp_path).await;
            return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("LLM parsing failed: {}", e)));
        }
    };
    
    // LLM already returns JSON - use it directly!
    info!("✅ Got JSON from LLM ({} bytes)", json_string.len());

    // Encrypt JSON
    info!("🔐 Encrypting JSON");
    let (encrypted_data, encryption_key) = match state.encryption_service.encrypt(&json_string) {
        Ok(result) => result,
        Err(e) => {
            error!("Encryption failed: {}", e);
            let _ = fs::remove_file(&temp_path).await;
            return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, "Encryption failed"));
        }
    };

    // Upload to IPFS
    info!("📤 Uploading to IPFS");
    let ipfs_cid = match state.ipfs_client.upload(&encrypted_data).await {
        Ok(cid) => cid,
        Err(e) => {
            error!("IPFS upload failed: {}", e);
            let _ = fs::remove_file(&temp_path).await;
            return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("IPFS upload failed: {}", e)));
        }
    };

    // Cleanup
    let _ = fs::remove_file(&temp_path).await;

    let processing_time = start_time.elapsed().as_millis() as u64;
    
    info!("✅ Successfully processed PDF in {}ms", processing_time);
    info!("📍 IPFS CID: {}", ipfs_cid);

    Ok(Json(ParseResponse {
        ipfs_cid: ipfs_cid.clone(),
        ipfs_url: format!("ipfs://{}", ipfs_cid),
        ipfs_gateway_url: format!("https://ipfs.io/ipfs/{}", ipfs_cid),
        encryption_key,
        metadata: FileMetadata {
            file_name,
            file_size,
            processed_at: chrono::Utc::now().to_rfc3339(),
            model_used: "llama3.3:70b-instruct-q4_K_M".to_string(),
            processing_time_ms: processing_time,
        },
    }))
}

async fn decrypt_handler(
    State(state): State<AppState>,
    Path(cid): Path<String>,
    Query(params): Query<DecryptQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("🔓 Decrypting IPFS content: {}", cid);

    // Fetch from IPFS
    let encrypted_data = state.ipfs_client.fetch(&cid)
        .await
        .map_err(|e| {
            error!("IPFS fetch failed: {}", e);
            error_response(StatusCode::NOT_FOUND, &format!("Failed to fetch from IPFS: {}", e))
        })?;

    // Decrypt
    let json_string = state.encryption_service.decrypt(&encrypted_data, &params.key)
        .map_err(|e| {
            error!("Decryption failed: {}", e);
            error_response(StatusCode::UNAUTHORIZED, "Decryption failed - invalid key")
        })?;

    // Parse JSON
    let json_value: serde_json::Value = serde_json::from_str(&json_string)
        .map_err(|e| {
            error!("JSON parsing failed: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Invalid JSON data")
        })?;

    info!("✅ Successfully decrypted content");

    Ok(Json(json_value))
}

async fn status_handler(
    State(state): State<AppState>,
    Path(cid): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("📊 Checking IPFS status for: {}", cid);

    let exists = state.ipfs_client.check_exists(&cid)
        .await
        .unwrap_or(false);

    Ok(Json(serde_json::json!({
        "cid": cid,
        "exists": exists,
        "gateway_url": format!("https://ipfs.io/ipfs/{}", cid),
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

fn error_response(status: StatusCode, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: status.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
    )
}