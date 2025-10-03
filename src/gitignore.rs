use log::trace;
use std::fs::{read_to_string, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Searches upward from the current directory until a .gitignore is found.
/// Returns the PathBuf to the .gitignore file or an error if none is found.
fn find_gitignore(start: &Path) -> Result<PathBuf, String> {
    let mut current = start.to_path_buf();

    loop {
        let candidate = current.join(".gitignore");
        if candidate.exists() {
            return Ok(candidate);
        }

        // If we are at the root, stop
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    Err("No .gitignore found in current or parent directories".to_string())
}

pub fn read_gitignore() -> Result<String, String> {
    let gitignore_path = find_gitignore(Path::new("."))?;
    trace!(".gitignore found: {}", gitignore_path.display());
    if gitignore_path.exists() {
        return Ok(read_to_string(gitignore_path).map_err(|_| "Failed to read .gitignore")?);
    }
    Err("Could not find .gitignore".to_string())
}

pub fn write_gitignore(content: &str) -> Result<(), String> {
    let gitignore_path = find_gitignore(Path::new("."))?;
    trace!(".gitignore found: {}", gitignore_path.display());
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(gitignore_path)
        .expect("Failed to open or create .gitignore");

    file.write_all(content.as_bytes())
        .map_err(|_| "Failed to write .gitignore")?;

    Ok(())
}

pub fn add_files_to_gitignore(content: &str, files: &[&str]) -> String {
    let mut existing: Vec<String> = content
        .lines()
        .map(|line| line.trim().to_string())
        .collect();

    for &file in files {
        if !existing.iter().any(|line| line == file) {
            existing.push(file.to_string());
        }
    }

    existing.join("\n") + "\n"
}
