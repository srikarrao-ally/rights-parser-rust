// src/encryption.rs - AES-256-GCM Encryption Service
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use tracing::{info, error};

pub struct EncryptionService {
    // This struct can hold configuration if needed in the future
}

impl EncryptionService {
    pub fn new() -> Self {
        info!("Initializing encryption service (AES-256-GCM)");
        Self {}
    }

    /// Encrypt data with AES-256-GCM
    /// Returns (encrypted_data, base64_encoded_key)
    pub fn encrypt(&self, plaintext: &str) -> Result<(Vec<u8>, String)> {
        // Generate random 256-bit key
        let key = Aes256Gcm::generate_key(&mut OsRng);
        let cipher = Aes256Gcm::new(&key);

        // Generate random 96-bit nonce (recommended for GCM)
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

        // Combine nonce + ciphertext
        let mut encrypted_data = nonce_bytes.to_vec();
        encrypted_data.extend_from_slice(&ciphertext);

        // Encode key as base64
        let key_b64 = general_purpose::STANDARD.encode(key.as_slice());

        info!(
            "Encrypted {} bytes → {} bytes (including nonce)",
            plaintext.len(),
            encrypted_data.len()
        );

        Ok((encrypted_data, key_b64))
    }

    /// Decrypt data with AES-256-GCM
    pub fn decrypt(&self, encrypted_data: &[u8], key_b64: &str) -> Result<String> {
        // Decode base64 key
        let key_bytes = general_purpose::STANDARD
            .decode(key_b64)
            .context("Invalid base64 key")?;

        if key_bytes.len() != 32 {
            anyhow::bail!("Invalid key length: expected 32 bytes, got {}", key_bytes.len());
        }

        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        // Extract nonce (first 12 bytes) and ciphertext
        if encrypted_data.len() < 12 {
            anyhow::bail!("Encrypted data too short");
        }

        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext_bytes = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed - invalid key or corrupted data: {:?}", e))?;

        let plaintext = String::from_utf8(plaintext_bytes)
            .context("Decrypted data is not valid UTF-8")?;

        info!(
            "Decrypted {} bytes → {} bytes",
            encrypted_data.len(),
            plaintext.len()
        );

        Ok(plaintext)
    }

    /// Generate a random encryption key (for testing/utilities)
    pub fn generate_key() -> String {
        let key = Aes256Gcm::generate_key(&mut OsRng);
        general_purpose::STANDARD.encode(key.as_slice())
    }
}

impl Default for EncryptionService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let service = EncryptionService::new();
        let plaintext = r#"{"title":"Test Agreement","licensor":"Company A"}"#;

        // Encrypt
        let (encrypted_data, key) = service.encrypt(plaintext).unwrap();
        assert!(encrypted_data.len() > plaintext.len());

        // Decrypt
        let decrypted = service.decrypt(&encrypted_data, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_with_wrong_key() {
        let service = EncryptionService::new();
        let plaintext = "Secret data";

        let (encrypted_data, _correct_key) = service.encrypt(plaintext).unwrap();
        let wrong_key = EncryptionService::generate_key();

        // Should fail with wrong key
        let result = service.decrypt(&encrypted_data, &wrong_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_corrupted_data() {
        let service = EncryptionService::new();
        let key = EncryptionService::generate_key();

        // Corrupted data
        let corrupted_data = vec![0u8; 50];

        let result = service.decrypt(&corrupted_data, &key);
        assert!(result.is_err());
    }
}