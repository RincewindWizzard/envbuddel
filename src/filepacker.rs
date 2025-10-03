use bincode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, bincode::Encode, bincode::Decode)]
pub enum EnvironmentPack {
    Folder(Vec<u8>),
    File(Vec<u8>),
}

impl EnvironmentPack {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        if path.exists() {
            if path.is_file() {
                Ok(EnvironmentPack::File(
                    fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?,
                ))
            } else if path.is_dir() {
                Ok(EnvironmentPack::Folder(tar_directory(path)?))
            } else {
                Err(format!(
                    "Path {:?} exists but is neither a file nor a folder",
                    path
                ))
            }
        } else {
            Err(format!("Path {:?} does not exist", path))
        }
    }

    /// Unpack the EnvironmentPack into the given destination path
    pub fn unpack(&self, dst_path: &Path) -> Result<(), String> {
        match self {
            EnvironmentPack::File(data) => {
                fs::write(dst_path, data)
                    .map_err(|e| format!("Failed to write file {:?}: {}", dst_path, e))
            }
            EnvironmentPack::Folder(tar_bytes) => {
                let cursor = std::io::Cursor::new(tar_bytes);
                let mut archive = tar::Archive::new(cursor);
                archive
                    .unpack(dst_path)
                    .map_err(|e| format!("Failed to unpack TAR archive to {:?}: {}", dst_path, e))
            }
        }
    }

    /// Serialize the EnvironmentPack to bytes (for encryption)
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        bincode::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| format!("Failed to serialize EnvironmentPack: {}", e))
    }

    /// Deserialize bytes back into an EnvironmentPack (after decryption)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        bincode::decode_from_slice(bytes, bincode::config::standard())
            .map(|(pack, _)| pack)
            .map_err(|e| format!("Failed to deserialize EnvironmentPack: {}", e))
    }
}

/// Create a TAR archive in memory from a directory
/// `dir_path` should be the path to the directory
/// Returns a Vec<u8> containing the TAR archive
pub fn tar_directory(dir_path: &Path) -> Result<Vec<u8>, String> {
    // Check that the path exists and is a directory
    let metadata = fs::metadata(dir_path).map_err(|e| {
        format!(
            "Failed to read metadata for '{}': {}",
            dir_path.display(),
            e
        )
    })?;

    if !metadata.is_dir() {
        return Err(format!("Path '{}' is not a directory", dir_path.display()));
    }

    // Create an in-memory buffer
    let tar_buffer = Vec::new();
    let mut tar_builder = tar::Builder::new(tar_buffer);

    // Recursively append all files and subdirectories
    tar_builder
        .append_dir_all(".", dir_path)
        .map_err(|e| format!("Failed to append directory to tar: {}", e))?;

    // Finish the archive and take ownership of the underlying buffer
    tar_builder
        .into_inner()
        .map_err(|e| format!("Failed to finish tar archive: {}", e))
}
