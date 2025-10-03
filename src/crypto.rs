use aes_gcm::Aes256Gcm;
use base64::Engine;
use rand::RngCore;
use std::fs;

pub struct Key {
    bytes: [u8; 32],
}

impl Key {
    /// Generate a new random 32-byte key
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        Self { bytes }
    }

    pub fn load_key(key: &Option<String>, keyfile: &str) -> Result<Key, String> {
        if let Some(key) = key {
            Key::from_base64(&key)
        } else {
            // Try to read the keyfile
            match fs::read_to_string(keyfile) {
                Ok(content) => {
                    let trimmed = content.trim();
                    if trimmed.is_empty() {
                        Err(format!("Error: Keyfile '{}' is empty", keyfile))
                    } else {
                        Key::from_base64(trimmed)
                    }
                }
                Err(e) => Err(format!(
                    "Error: Failed to read keyfile '{}': {}",
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
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Encode key as standard Base64
    pub fn to_base64(&self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&self.bytes)
    }

    /// Encrypt a string and return ciphertext with prepended nonce
    pub fn encrypt_string(&self, plaintext: &str) -> Result<Vec<u8>, String> {
        use aes_gcm::aead::{rand_core::RngCore, Aead, OsRng};
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce};

        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&self.bytes);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map(|mut ct| {
                let mut result = nonce_bytes.to_vec();
                result.append(&mut ct);
                result
            })
            .map_err(|e| format!("Encryption failed: {:?}", e))
    }

    pub fn encrypt_string_base64(&self, plaintext: &str) -> Result<String, String> {
        use base64::{engine::general_purpose, Engine as _};

        let ciphertext = self.encrypt_string(plaintext)?;
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
    pub fn decrypt_string(&self, ciphertext_with_nonce: &[u8]) -> Result<String, String> {
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
            .map_err(|e| format!("Decryption failed: {:?}", e))
            .and_then(|bytes| String::from_utf8(bytes).map_err(|e| format!("UTF-8 error: {}", e)))
    }

    pub fn decrypt_string_base64(&self, ciphertext_with_nonce: &str) -> Result<String, String> {
        // Remove any line breaks (LF or CRLF) before decoding
        let cleaned = ciphertext_with_nonce.replace(&['\n', '\r'][..], "");

        // Decode Base64
        let ciphertext_bytes = base64::engine::general_purpose::STANDARD
            .decode(cleaned)
            .map_err(|e| format!("Failed to decode Base64 ciphertext: {}", e))?;

        // Decrypt
        self.decrypt_string(&ciphertext_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::Key;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        // Generate a random key
        let key = Key::generate();

        // Original plaintext
        let plaintext = "This is a secret string for envcaja!";

        // Encrypt the plaintext
        let ciphertext = key
            .encrypt_string_base64(plaintext)
            .expect("Encryption failed");

        // Decrypt the ciphertext
        let decrypted = key
            .decrypt_string_base64(&ciphertext)
            .expect("Decryption failed");

        // Check that the decrypted string matches the original
        assert_eq!(plaintext, decrypted);
    }
}
