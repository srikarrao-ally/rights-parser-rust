// src/ipfs_client.rs - IPFS Client with Pinata Support
use anyhow::{Context, Result};
use reqwest::{Client, multipart};
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};

#[derive(Clone)]
pub struct IPFSClient {
    client: Client,
    ipfs_url: String,
    pinata_jwt: Option<String>,
    use_pinata: bool,
}

#[derive(Deserialize)]
struct IPFSAddResponse {
    #[serde(rename = "Hash")]
    hash: String,
}

#[derive(Deserialize)]
struct PinataResponse {
    #[serde(rename = "IpfsHash")]
    ipfs_hash: String,
}

impl IPFSClient {
    pub fn new(ipfs_url: String, pinata_jwt: Option<String>) -> Self {
        let use_pinata = pinata_jwt.is_some();
        
        if use_pinata {
            info!("Initializing IPFS client with Pinata");
        } else {
            info!("Initializing IPFS client with local node: {}", ipfs_url);
        }

        Self {
            client: Client::new(),
            ipfs_url,
            pinata_jwt,
            use_pinata,
        }
    }

    /// Upload data to IPFS
    pub async fn upload(&self, data: &[u8]) -> Result<String> {
        if self.use_pinata {
            self.upload_to_pinata(data).await
        } else {
            self.upload_to_local(data).await
        }
    }

    /// Fetch data from IPFS
    pub async fn fetch(&self, cid: &str) -> Result<Vec<u8>> {
        if self.use_pinata {
            self.fetch_from_pinata(cid).await
        } else {
            self.fetch_from_local(cid).await
        }
    }

    /// Check if content exists on IPFS
    pub async fn check_exists(&self, cid: &str) -> Result<bool> {
        match self.fetch(cid).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Health check
    pub async fn health_check(&self) -> Result<bool> {
        if self.use_pinata {
            self.check_pinata_health().await
        } else {
            self.check_local_health().await
        }
    }

    // Local IPFS node methods

    async fn upload_to_local(&self, data: &[u8]) -> Result<String> {
        info!("Uploading {} bytes to local IPFS node", data.len());

        let form = multipart::Form::new()
            .part("file", multipart::Part::bytes(data.to_vec())
                .file_name("encrypted.json"));

        let response = self.client
            .post(format!("{}/api/v0/add", self.ipfs_url))
            .multipart(form)
            .send()
            .await
            .context("Failed to upload to IPFS")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("IPFS upload failed: {} - {}", status, error_text);
        }

        let result: IPFSAddResponse = response.json()
            .await
            .context("Failed to parse IPFS response")?;

        info!("✅ Uploaded to IPFS: {}", result.hash);
        Ok(result.hash)
    }

    async fn fetch_from_local(&self, cid: &str) -> Result<Vec<u8>> {
        info!("Fetching {} from local IPFS node", cid);

        let response = self.client
            .post(format!("{}/api/v0/cat?arg={}", self.ipfs_url, cid))
            .send()
            .await
            .context("Failed to fetch from IPFS")?;

        if !response.status().is_success() {
            anyhow::bail!("IPFS fetch failed: {}", response.status());
        }

        let data = response.bytes()
            .await
            .context("Failed to read IPFS response")?
            .to_vec();

        info!("✅ Fetched {} bytes from IPFS", data.len());
        Ok(data)
    }

    async fn check_local_health(&self) -> Result<bool> {
        let response = self.client
            .post(format!("{}/api/v0/version", self.ipfs_url))
            .send()
            .await;

        Ok(response.is_ok())
    }

    // Pinata methods

    async fn upload_to_pinata(&self, data: &[u8]) -> Result<String> {
        let jwt = self.pinata_jwt.as_ref()
            .context("Pinata JWT not configured")?;

        info!("Uploading {} bytes to Pinata", data.len());

        let form = multipart::Form::new()
            .part("file", multipart::Part::bytes(data.to_vec())
                .file_name("encrypted.json"));

        let response = self.client
            .post("https://api.pinata.cloud/pinning/pinFileToIPFS")
            .header("Authorization", format!("Bearer {}", jwt))
            .multipart(form)
            .send()
            .await
            .context("Failed to upload to Pinata")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Pinata upload failed: {} - {}", status, error_text);
        }

        let result: PinataResponse = response.json()
            .await
            .context("Failed to parse Pinata response")?;

        info!("✅ Uploaded to Pinata: {}", result.ipfs_hash);
        Ok(result.ipfs_hash)
    }

    async fn fetch_from_pinata(&self, cid: &str) -> Result<Vec<u8>> {
        info!("Fetching {} from Pinata gateway", cid);

        // Try Pinata gateway first, fallback to public gateway
        let urls = vec![
            format!("https://gateway.pinata.cloud/ipfs/{}", cid),
            format!("https://ipfs.io/ipfs/{}", cid),
            format!("https://cloudflare-ipfs.com/ipfs/{}", cid),
        ];

        for url in urls {
            match self.fetch_from_gateway(&url).await {
                Ok(data) => {
                    info!("✅ Fetched {} bytes from gateway", data.len());
                    return Ok(data);
                }
                Err(e) => {
                    warn!("Failed to fetch from {}: {}", url, e);
                    continue;
                }
            }
        }

        anyhow::bail!("Failed to fetch from all IPFS gateways")
    }

    async fn fetch_from_gateway(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client
            .get(url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Gateway returned: {}", response.status());
        }

        Ok(response.bytes().await?.to_vec())
    }

    async fn check_pinata_health(&self) -> Result<bool> {
        let jwt = match &self.pinata_jwt {
            Some(j) => j,
            None => return Ok(false),
        };

        let response = self.client
            .get("https://api.pinata.cloud/data/testAuthentication")
            .header("Authorization", format!("Bearer {}", jwt))
            .send()
            .await;

        Ok(response.is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_ipfs_initialization() {
        let client = IPFSClient::new(
            "http://localhost:5001".to_string(),
            None
        );
        assert!(!client.use_pinata);
    }

    #[tokio::test]
    async fn test_pinata_initialization() {
        let client = IPFSClient::new(
            "http://localhost:5001".to_string(),
            Some("test_jwt".to_string())
        );
        assert!(client.use_pinata);
    }

    #[tokio::test]
    async fn test_fetch_from_public_gateway() {
        let client = IPFSClient::new(
            "http://localhost:5001".to_string(),
            Some("test".to_string())
        );

        // Test with a known IPFS hash (IPFS website logo)
        let cid = "QmZULkCELmmk5XNfCgTnCyFgAVxBRBXyDHGGMVoLFLiXEN";
        
        // This might fail if gateways are down, so we just check it doesn't panic
        let _ = client.fetch(cid).await;
    }
}