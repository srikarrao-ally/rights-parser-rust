// src/pdf_extractor.rs
use anyhow::{Context, Result};
use pdf_extract::extract_text_from_mem;
use tracing::{info, warn};
use std::process::Command;
use std::io::Read;

pub struct PDFExtractor;

impl PDFExtractor {
    pub fn new() -> Self {
        Self
    }

   pub async fn extract_text(&self, pdf_data: &[u8]) -> Result<String> {
    info!("📖 Extracting text from PDF ({} bytes)", pdf_data.len());
    
    // Try pdf_extract first
    match extract_text_from_mem(pdf_data) {
        Ok(text) => {
            info!("✅ pdf_extract succeeded");
            Ok(self.clean_text(&text))
        }
        Err(e) => {
            warn!("pdf_extract failed: {}. Falling back to pdftotext", e);
            self.extract_with_pdftotext(pdf_data).await
        }
    }
}

async fn extract_with_pdftotext(&self, pdf_data: &[u8]) -> Result<String> {
    let temp_path = "/tmp/temp.pdf";
    std::fs::write(temp_path, pdf_data)?;
    
    let output = Command::new("pdftotext")
        .args(&["-layout", temp_path, "-"])
        .output()
        .context("pdftotext failed")?;
    
    std::fs::remove_file(temp_path)?;
    
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(self.clean_text(&text))
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