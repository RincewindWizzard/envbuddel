use std::fs::{read_to_string, OpenOptions};
use std::io::Write;
use std::path::Path;

const GITIGNORE_PATH: &str = ".gitignore";
const SECRET_FILES: [&str; 3] = ["safe.key", ".env", ".idea"];

fn read_gitignore() -> String {
    let gitignore_path = Path::new(GITIGNORE_PATH);

    if gitignore_path.exists() {
        read_to_string(gitignore_path).expect("Failed to read .gitignore")
    } else {
        String::new()
    }
}

fn write_gitignore(content: &str) {
    let gitignore_path = Path::new(GITIGNORE_PATH);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(gitignore_path)
        .expect("Failed to open or create .gitignore");

    file.write_all(content.as_bytes())
        .expect("Failed to write to .gitignore");
}

fn add_files_to_gitignore(content: &str, files: &[&str]) -> String {
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

pub fn gitignore() {
    let content = read_gitignore();
    let content = add_files_to_gitignore(&content, &SECRET_FILES);
    write_gitignore(&content);
}
