// src/main.rs
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tracing::{info, error};
use tower_http::cors::CorsLayer;

mod pdf_extractor;
mod llm_service;
mod json_builder;
mod models;

use pdf_extractor::PDFExtractor;
use llm_service::LLMService;
use json_builder::JSONBuilder;
use models::*;

#[derive(Clone)]
struct AppState {
    llm_service: Arc<LLMService>,
    pdf_extractor: Arc<PDFExtractor>,
    json_builder: Arc<JSONBuilder>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting Rights Agreement Parser Service");

    // Initialize services
    let llm_service = Arc::new(LLMService::new(
        "http://localhost:11434".to_string(),
        "llama3".to_string(),
    ));
    let pdf_extractor = Arc::new(PDFExtractor::new());
    let json_builder = Arc::new(JSONBuilder::new());

    let state = AppState {
        llm_service,
        pdf_extractor,
        json_builder,
    };

    // Build router
    let app = Router::new()
        .route("/", get(health_check))
        .route("/api/parse", post(parse_pdf))
        .route("/api/health", get(health_check))
        .route("/api/debug-pdf", post(debug_pdf_size))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();
    
    info!("✅ Server running on http://0.0.0.0:8080");
    
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "rights-agreement-parser",
        "version": "0.1.0"
    }))
}

async fn parse_pdf(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<RightsAgreementJSON>, (StatusCode, String)> {
    info!("📄 Received PDF parsing request");

    // Extract PDF file from multipart
    let mut pdf_data: Vec<u8> = Vec::new();
    
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!("Failed to read multipart field: {}", e);
        (StatusCode::BAD_REQUEST, e.to_string())
    })? {
        if field.name() == Some("pdf") {
            pdf_data = field.bytes().await.map_err(|e| {
                error!("Failed to read PDF bytes: {}", e);
                (StatusCode::BAD_REQUEST, e.to_string())
            })?.to_vec();
        }
    }

    if pdf_data.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No PDF file provided".to_string()));
    }

    // Step 1: Extract text from PDF
    info!("📖 Extracting text from PDF...");
    let extracted_text = state.pdf_extractor
        .extract_text(&pdf_data)
        .await
        .map_err(|e| {
            error!("PDF extraction failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    info!("✅ Extracted {} characters from PDF", extracted_text.len());

    // Step 2: Parse with LLM
    info!("🤖 Parsing with LLM...");
    let parsed_data = state.llm_service
        .parse_agreement(&extracted_text)
        .await
        .map_err(|e| {
            error!("LLM parsing failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    info!("✅ LLM parsing complete");

    // Step 3: Build JSON structure
    info!("🔨 Building JSON...");
    let json_output = state.json_builder
        .build_agreement(&parsed_data)
        .await
        .map_err(|e| {
            error!("JSON building failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    info!("✅ JSON generation complete");

    Ok(Json(json_output))
}



async fn debug_pdf_size(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut pdf_data: Vec<u8> = Vec::new();
    
    // Fix 1: Proper error conversion
    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart error: {}", e)))? 
    {
        if field.name() == Some("pdf") {
            pdf_data = field.bytes().await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Field bytes error: {}", e)))?
                .to_vec();
        }
    }
    
    Ok(Json(serde_json::json!({
        "pdf_size_bytes": pdf_data.len(),
        "first_100_bytes": std::str::from_utf8(&pdf_data[..100.min(pdf_data.len())]).unwrap_or("non-utf8"),
        "is_pdf_header": pdf_data.starts_with(b"%PDF"),
        "success": true
    })))
}