use crate::crypto::KeySource::{Env, File};
use crate::filepacker::EnvironmentPack;
use base64::Engine;
use rand::RngCore;
use std::fs;
use std::path::{Path, PathBuf};

const BASE62: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

pub struct Key {
    bytes: [u8; 32],
}

#[derive(Eq, PartialEq, Debug, Clone)]
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
            Ok((Key::from_printable(&key)?, Env))
        } else {
            // Try to read the keyfile
            match fs::read_to_string(keyfile) {
                Ok(content) => {
                    let trimmed = content.trim();
                    if trimmed.is_empty() {
                        Err(format!("Error: Keyfile {:?} is empty", keyfile))
                    } else {
                        Ok((Key::from_printable(&trimmed)?, File(keyfile.to_path_buf())))
                    }
                }
                Err(e) => Err(format!(
                    "Error: Failed to read keyfile {:?}: {}",
                    keyfile, e
                )),
            }
        }
    }

    pub fn save_key(&self, keyfile: &Path) -> Result<(), String> {
        fs::write(keyfile, self.to_printable()).map_err(|e| e.to_string())
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn to_base64(&self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&self.bytes)
    }

    pub fn to_printable(&self) -> String {
        use base_x::encode;
        encode(BASE62, &self.bytes)
    }

    pub fn from_printable(encoded: &str) -> Result<Self, String> {
        use base_x::decode;
        let bytes =
            decode(BASE62, encoded).map_err(|e| format!("Failed to decode Base62: {}", e))?;
        Self::from_bytes(&bytes)
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

    /// Decrypt a ciphertext (with prepended nonce) back to a EnvironmentPack
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
    use super::*;
    use crate::filepacker::EnvironmentPack;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    // Test that key generation produces 32 bytes
    #[test]
    fn test_generate_key_length() {
        let key = Key::generate();
        assert_eq!(key.bytes.len(), 32);
    }

    // Test from_bytes and as_bytes roundtrip
    #[test]
    fn test_from_bytes_roundtrip() {
        let original_bytes = [42u8; 32];
        let key = Key::from_bytes(&original_bytes).unwrap();
        assert_eq!(key.as_bytes(), &original_bytes);
    }

    // Test invalid from_bytes length
    #[test]
    fn test_from_bytes_invalid_length() {
        let result = Key::from_bytes(&[1, 2, 3]);
        assert!(result.is_err());
    }

    // Test Base64 encode/decode
    #[test]
    fn test_base64_roundtrip() {
        let key = Key::generate();
        let b64 = key.to_base64();
        let decoded = Key::from_base64(&b64).unwrap();
        assert_eq!(decoded.as_bytes(), key.as_bytes());
    }

    // Test Base62 encode/decode (to_printable/from_printable)
    #[test]
    fn test_printable_roundtrip() {
        let key = Key::generate();
        let printable = key.to_printable();
        let decoded = Key::from_printable(&printable).unwrap();
        assert_eq!(decoded.as_bytes(), key.as_bytes());
    }

    // Test load_key from environment (Some)
    #[test]
    fn test_load_key_env() {
        let key = Key::generate();
        let key_str = key.to_printable();
        let (loaded, source) =
            Key::load_key(&Some(key_str.clone()), Path::new("/tmp/does_not_exist")).unwrap();
        assert_eq!(source, KeySource::Env);
        assert_eq!(loaded.to_printable(), key_str);
    }

    // Test load_key from file
    #[test]
    fn test_load_key_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("key.txt");

        let key = Key::generate();
        let key_str = key.to_printable();
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "{}", key_str).unwrap();

        let (loaded, source) = Key::load_key(&None, &file_path).unwrap();
        assert_eq!(source, KeySource::File(file_path.clone()));
        assert_eq!(loaded.to_printable(), key_str);
    }

    // Test save_key
    #[test]
    fn test_save_key() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("key.txt");

        let key = Key::generate();
        key.save_key(&file_path).unwrap();
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content.trim(), key.to_printable());
    }

    // Test encrypt/decrypt roundtrip
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let data: Vec<u8> = vec![0, 1, 2, 42];
        let key = Key::generate();
        let pack = EnvironmentPack::File(data.clone());

        let ciphertext = key.encrypt(&pack.to_bytes().unwrap()).unwrap();
        let decrypted = key.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted.content().unwrap(), data);
    }

    // Test decrypt_base64 fails on corrupted data
    #[test]
    fn test_decrypt_base64_corrupted() {
        let key = Key::generate();
        let result = key.decrypt_base64("thisisnotbase64");
        assert!(result.is_err());
    }
}
