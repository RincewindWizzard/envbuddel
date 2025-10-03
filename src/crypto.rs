use crate::crypto::KeySource::{Env, File};
use crate::filepacker::EnvironmentPack;
use base64::Engine;
use rand::RngCore;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Key {
    bytes: [u8; 32],
}

pub enum KeySource {
    File(PathBuf),
    Env,
}

impl Key {
    /// Generate a new random 32-byte key
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        Self { bytes }
    }

    pub fn load_key(key: &Option<String>, keyfile: &Path) -> Result<(Key, KeySource), String> {
        if let Some(key) = key {
            Ok((Key::from_base64(&key)?, Env))
        } else {
            // Try to read the keyfile
            match fs::read_to_string(keyfile) {
                Ok(content) => {
                    let trimmed = content.trim();
                    if trimmed.is_empty() {
                        Err(format!("Error: Keyfile {:?} is empty", keyfile))
                    } else {
                        Ok((Key::from_base64(&trimmed)?, File(keyfile.to_path_buf())))
                    }
                }
                Err(e) => Err(format!(
                    "Error: Failed to read keyfile {:?}: {}",
                    keyfile, e
                )),
            }
        }
    }

    /// Load key from raw bytes (must be 32 bytes)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != 32 {
            return Err(format!(
                "Invalid key length: expected 32 bytes, got {}",
                bytes.len()
            ));
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(bytes);
        Ok(Self { bytes: array })
    }

    /// Load key from standard Base64
    pub fn from_base64(encoded: &str) -> Result<Self, String> {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|e| format!("Failed to decode Base64 key: {}", e))?;
        Self::from_bytes(&bytes)
    }

    /// Get key as raw bytes
    #[allow(dead_code)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Encode key as standard Base64
    pub fn to_base64(&self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&self.bytes)
    }

    /// Encrypt a string and return ciphertext with prepended nonce
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        use aes_gcm::aead::{rand_core::RngCore, Aead, OsRng};
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce};

        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&self.bytes);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        cipher
            .encrypt(nonce, plaintext)
            .map(|mut ct| {
                let mut result = nonce_bytes.to_vec();
                result.append(&mut ct);
                result
            })
            .map_err(|e| format!("Encryption failed: {:?}", e))
    }

    pub fn encrypt_base64(&self, pack: &EnvironmentPack) -> Result<String, String> {
        use base64::{engine::general_purpose, Engine as _};

        let ciphertext = self.encrypt(pack.to_bytes()?.as_slice())?;
        let b64 = general_purpose::STANDARD.encode(&ciphertext);

        // Wrap lines manually at 64 chars
        let wrapped: String = b64
            .chars()
            .collect::<Vec<_>>()
            .chunks(64)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(wrapped)
    }

    /// Decrypt a ciphertext (with prepended nonce) back to a string
    pub fn decrypt(&self, ciphertext_with_nonce: &[u8]) -> Result<EnvironmentPack, String> {
        use aes_gcm::aead::Aead;
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
        if ciphertext_with_nonce.len() < 12 {
            return Err("Ciphertext too short: missing nonce".to_string());
        }

        let (nonce_bytes, ciphertext) = ciphertext_with_nonce.split_at(12);
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&self.bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(nonce_bytes);

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| {
                "Decryption failed. Possible causes: wrong key, wrong nonce, or corrupted data."
                    .to_string()
            })
            .and_then(|bytes| {
                EnvironmentPack::from_bytes(&bytes).map_err(|e| format!("UTF-8 error: {}", e))
            })
    }

    pub fn decrypt_base64(&self, ciphertext_with_nonce: &str) -> Result<EnvironmentPack, String> {
        // Remove any line breaks (LF or CRLF) before decoding
        let cleaned = ciphertext_with_nonce.replace(&['\n', '\r'][..], "");

        // Decode Base64
        let ciphertext_bytes = base64::engine::general_purpose::STANDARD
            .decode(cleaned)
            .map_err(|e| format!("Failed to decode Base64 ciphertext: {}", e))?;

        // Decrypt
        self.decrypt(&ciphertext_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::Key;
}
