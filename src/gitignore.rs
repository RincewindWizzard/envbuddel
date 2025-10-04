use log::{debug, info, trace, warn};
use std::path::{Path, PathBuf};
use std::{env, fs};

pub fn find_repo() -> Result<PathBuf, String> {
    let mut current = Path::new(".")
        .canonicalize()
        .map_err(|e| format!("{:?}", e))?
        .to_path_buf();

    loop {
        let parent = current.clone();
        let candidate = current.join(".git");
        trace!("repo path candidate: {:?}", candidate);
        if candidate.exists() && candidate.is_dir() {
            return Ok(parent);
        }

        // If we are at the root, stop
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    Err("No git repository found in current or parent directories".to_string())
}

pub fn gitignore(files: Vec<PathBuf>) -> Result<(), String> {
    if let Ok(repository) = find_repo() {
        let gitignore = repository.join(".gitignore");

        let content = if gitignore.exists() {
            fs::read_to_string(&gitignore)
                .map_err(|err| format!("Could not read .gitignore {:?}. {:?}", gitignore, err))?
        } else {
            "".to_string()
        };

        trace!("gitignore content: {}", content);

        let repository = repository.canonicalize().map_err(|err| {
            format!(
                "Could not canonicalize repository path {:?}. {:?}",
                repository, err
            )
        })?;

        let mut entries: Vec<String> = files
            .iter()
            .filter_map(|file| {
                // Absoluten Pfad berechnen, auch wenn die Datei noch nicht existiert
                let canonical = if file.is_absolute() {
                    file.to_path_buf()
                } else {
                    match env::current_dir() {
                        Ok(cwd) => cwd.join(file),
                        Err(err) => {
                            eprintln!("Warning: Could not get current directory: {}", err);
                            return None;
                        }
                    }
                };

                // Relativen Pfad zum Repository bestimmen
                let relative = match canonical.strip_prefix(&repository) {
                    Ok(r) => r,
                    Err(err) => {
                        eprintln!(
                            "Warning: Could not strip prefix {:?} from {:?}: {}",
                            repository, canonical, err
                        );
                        return None;
                    }
                };

                // In UTF-8 String umwandeln
                match relative.to_str() {
                    Some(s) => Some(s.to_string()),
                    None => {
                        eprintln!("Warning: Path {:?} is not valid UTF-8", relative);
                        None
                    }
                }
            })
            .collect();

        entries.push(".idea".to_string());

        let entries: Vec<&str> = entries.iter().map(|s| s.as_str()).collect();

        debug!("Finished reading .gitignore file");
        let content = add_files_to_gitignore(&content, &entries);
        fs::write(gitignore, content).map_err(|e| format!("{:?}", e))?;

        debug!("Finished writing .gitignore file");
        info!("🛡️ Added key and environment to .gitignore");
    } else {
        warn!("Could not find a git repository. Skipping creation of .gitignore.");
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn test_add_files_to_gitignore_adds_new_entries() {
        let content = "existing_file\n";
        let files = ["new_file", "existing_file"];
        let result = add_files_to_gitignore(content, &files);
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines.contains(&"existing_file"));
        assert!(lines.contains(&"new_file"));
        assert_eq!(lines.len(), 2); // keine Duplikate
    }

    #[test]
    fn test_add_files_to_gitignore_empty_content() {
        let content = "";
        let files = ["file1"];
        let result = add_files_to_gitignore(content, &files);
        assert_eq!(result, "file1\n");
    }



    #[test]
    fn test_find_repo_no_repo() {
        let tmp_dir = TempDir::new().unwrap();

        let current = env::current_dir().unwrap();
        env::set_current_dir(tmp_dir.path()).unwrap();

        let result = find_repo();
        assert!(result.is_err());

        env::set_current_dir(current).unwrap();
    }

    #[test]
    fn test_gitignore_skips_without_repo() {
        let tmp_dir = TempDir::new().unwrap();

        let current = env::current_dir().unwrap();
        env::set_current_dir(tmp_dir.path()).unwrap();

        // Should not fail even if no .git
        let keyfile = tmp_dir.path().join("secret.txt");
        File::create(&keyfile).unwrap();
        let files = vec![keyfile.clone()];

        gitignore(files).unwrap();

        // No .gitignore should exist
        assert!(!tmp_dir.path().join(".gitignore").exists());

        env::set_current_dir(current).unwrap();
    }
}
