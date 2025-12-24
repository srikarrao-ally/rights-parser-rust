// src/llm_service.rs - LLM Service with Health Check
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};

#[derive(Clone)]
pub struct LLMService {
    ollama_url: String,
    model_name: String,
    client: Client,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    format: String,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: usize,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

impl LLMService {
    pub fn new(ollama_url: String, model_name: String) -> Self {
        info!("Initializing LLM service");
        info!("  Ollama URL: {}", ollama_url);
        info!("  Model: {}", model_name);

        Self {
            ollama_url,
            model_name,
            client: Client::new(),
        }
    }

    /// Parse agreement text and return JSON string
    pub async fn parse_agreement(&self, text: &str) -> Result<String> {
        info!("Parsing agreement with LLM ({} chars)", text.len());

        // Truncate text if needed (70B can handle more, but be safe)
        let text_to_use = if text.len() > 100000 {
            warn!("Text too long, truncating to 50K chars");
            &text[..100000]
        } else {
            text
        };

        // Simple prompt - Modelfile has all the instructions
        let prompt = format!(
            r#"CONTRACT TEXT:
{}

Extract all information into JSON format."#,
            text_to_use
        );

        // Call Ollama
        let request = OllamaRequest {
            model: self.model_name.clone(),
            prompt,
            stream: false,
            format: "json".to_string(),
            options: OllamaOptions {
                temperature: 0.0,
                num_predict: 8192,
            },
        };

        info!("Calling Ollama API...");
        let response = self
            .client
            .post(format!("{}/api/generate", self.ollama_url))
            .json(&request)
            .timeout(std::time::Duration::from_secs(300)) // 5 min timeout for 70B
            .send()
            .await
            .context("Failed to call Ollama API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API error: {} - {}", status, error_text);
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        let json_response = ollama_response.response.trim();

        info!("✅ LLM returned {} chars", json_response.len());

        // Clean up any markdown code blocks if present
        let cleaned = self.clean_json_response(json_response);

        // Validate it's valid JSON
        serde_json::from_str::<serde_json::Value>(&cleaned)
            .context("LLM did not return valid JSON")?;

        Ok(cleaned)
    }

    /// Clean JSON response (remove markdown, extra text)
    fn clean_json_response(&self, response: &str) -> String {
        let mut cleaned = response.trim();

        // Remove markdown code blocks
        if cleaned.starts_with("```json") {
            cleaned = cleaned.strip_prefix("```json").unwrap().trim();
        }
        if cleaned.starts_with("```") {
            cleaned = cleaned.strip_prefix("```").unwrap().trim();
        }
        if cleaned.ends_with("```") {
            cleaned = cleaned.strip_suffix("```").unwrap().trim();
        }

        // Find JSON object boundaries
        if let Some(start) = cleaned.find('{') {
            if let Some(end) = cleaned.rfind('}') {
                cleaned = &cleaned[start..=end];
            }
        }

        cleaned.to_string()
    }

    /// Health check for Ollama service
    pub async fn health_check(&self) -> Result<bool> {
        match self
            .client
            .get(format!("{}/api/tags", self.ollama_url))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_json_response() {
        let service = LLMService::new(
            "http://localhost:11434".to_string(),
            "test".to_string(),
        );

        // Test with markdown
        let input = r#"```json
{"title": "Test"}
```"#;
        let cleaned = service.clean_json_response(input);
        assert_eq!(cleaned, r#"{"title": "Test"}"#);

        // Test with extra text before
        let input = r#"Here is the JSON:
{"title": "Test"}"#;
        let cleaned = service.clean_json_response(input);
        assert_eq!(cleaned, r#"{"title": "Test"}"#);

        // Test with already clean JSON
        let input = r#"{"title": "Test"}"#;
        let cleaned = service.clean_json_response(input);
        assert_eq!(cleaned, r#"{"title": "Test"}"#);
    }
}