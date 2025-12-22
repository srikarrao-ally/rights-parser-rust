// src/pdf_extractor.rs
use anyhow::{Context, Result};
use pdf_extract::extract_text_from_mem;
use tracing::{info, warn};

pub struct PDFExtractor;

impl PDFExtractor {
    pub fn new() -> Self {
        Self
    }

    pub async fn extract_text(&self, pdf_data: &[u8]) -> Result<String> {
        info!("📖 Extracting text from PDF ({} bytes)", pdf_data.len());

        // Extract text from PDF
        let text = extract_text_from_mem(pdf_data)
            .context("Failed to extract text from PDF")?;

        // Clean and normalize text
        let cleaned_text = self.clean_text(&text);

        info!("✅ Extracted {} characters", cleaned_text.len());

        Ok(cleaned_text)
    }

    fn clean_text(&self, text: &str) -> String {
        text
            // Remove excessive whitespace
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
            // Remove control characters
            .chars()
            .filter(|c| !c.is_control() || *c == '\n')
            .collect::<String>()
            // Normalize line breaks
            .replace("\n\n\n", "\n\n")
            .trim()
            .to_string()
    }

    pub fn extract_sections(&self, text: &str) -> Vec<(String, String)> {
        // Extract key sections from agreement
        let mut sections = Vec::new();

        // Common section headers in rights agreements
        let headers = vec![
            "PARTIES",
            "TERRITORY",
            "MEDIA RIGHTS",
            "TERM",
            "FINANCIAL TERMS",
            "PAYMENT",
            "DELIVERABLES",
            "WARRANTIES",
            "INDEMNIFICATION",
            "GOVERNING LAW",
        ];

        for header in headers {
            if let Some(section_text) = self.find_section(text, header) {
                sections.push((header.to_string(), section_text));
            }
        }

        sections
    }

    fn find_section(&self, text: &str, header: &str) -> Option<String> {
        // Simple section extraction logic
        // In production, use more sophisticated parsing
        let lines: Vec<&str> = text.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            if line.to_uppercase().contains(header) {
                // Get next 10 lines as section content
                let end = (i + 10).min(lines.len());
                let section_content = lines[i+1..end].join("\n");
                return Some(section_content);
            }
        }

        None
    }
}