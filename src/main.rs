mod crypto;
mod gitignore;

use crate::crypto::Key;
use clap::{Parser, Subcommand};
use std::fs;

#[derive(Parser)]
#[command(name = "envcaja")]
#[command(about = "File-based secret manager for CI/CD pipelines", long_about = None)]
struct Cli {
    /// Path to the keyfile
    #[arg(long, default_value = "safe.key")]
    keyfile: String,

    #[arg(long, env = "CI_SECRET")]
    key: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Info {},
    Init {},
    Encrypt {
        /// path to .env file
        #[arg(long, default_value = ".env")]
        dotenv: String,

        /// path to env.enc file
        #[arg(long, default_value = "env.enc")]
        vault: String,
    },

    Decrypt {
        /// path to .env file
        #[arg(long, default_value = ".env")]
        dotenv: String,

        /// path to env.enc file
        #[arg(long, default_value = "env.enc")]
        vault: String,
    },
}
fn safe_load_key(cli: &Cli) -> Key {
    match Key::load_key(&cli.key, &cli.keyfile) {
        Ok(k) => k,
        Err(err) => {
            eprintln!("Could not load key: {}", err);
            std::process::exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init {} => {
            let key = Key::generate();
            println!("Please run this to provide the key as environment variable:\n");
            println!("  $ export CI_SECRET=\"{}\"", key.to_base64());
            println!();

            if let Err(err) = fs::write(&cli.keyfile, key.to_base64()) {
                eprintln!("{}", err); // print error to stderr
                std::process::exit(1); // exit with failure
            }
            println!("Key written to \"{}\"", cli.keyfile);

            println!("Excluding secret files using \".gitignore\".");
            gitignore::gitignore();
        }
        Commands::Info {} => match Key::load_key(&cli.key, &cli.keyfile) {
            Ok(key) => {
                println!("Successfully loaded the key:");
                println!("CI_SECRET=\"{}\"", key.to_base64());
            }
            Err(err) => eprintln!("{}", err),
        },
        Commands::Encrypt { dotenv, vault } => {
            let key = safe_load_key(&cli);

            // Read the input file
            let content = match fs::read_to_string(dotenv) {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("Failed to read file '{}': {}", dotenv, err);
                    std::process::exit(1);
                }
            };

            // Encrypt the content
            let ciphertext = match key.encrypt_string_base64(&content) {
                Ok(ct) => ct,
                Err(err) => {
                    eprintln!("Encryption failed: {}", err);
                    std::process::exit(1);
                }
            };

            // Write the ciphertext to output file
            if let Err(err) = fs::write(vault, ciphertext) {
                eprintln!("Failed to write file '{}': {}", vault, err);
                std::process::exit(1);
            }

            println!("Encrypted content successfully written to '{}'", vault);
        }
        Commands::Decrypt { dotenv, vault } => {
            let key = safe_load_key(&cli);

            // Read the encrypted file
            let ciphertext = match fs::read_to_string(vault) {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("Failed to read encrypted file '{}': {}", vault, err);
                    std::process::exit(1);
                }
            };

            // Decrypt the content
            let plaintext = match key.decrypt_string_base64(&ciphertext) {
                Ok(pt) => pt,
                Err(err) => {
                    eprintln!("Decryption failed: {}", err);
                    std::process::exit(1);
                }
            };

            // Write the decrypted content to output file
            if let Err(err) = fs::write(dotenv, plaintext) {
                eprintln!("Failed to write output file '{}': {}", dotenv, err);
                std::process::exit(1);
            }

            println!("Decrypted content successfully written to '{}'", dotenv);
        }
    }
}
