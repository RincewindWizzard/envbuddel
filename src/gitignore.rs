use log::{debug, info, trace};
use std::path::{Path, PathBuf};
use std::{env, fs};

pub fn find_repo() -> Result<PathBuf, String> {
    let mut current = Path::new(".").to_path_buf();

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
    info!("Excluding secret files using \".gitignore\".");

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
    } else {
        info!("Could not find a git repository. Skipping creation of .gitignore.");
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
