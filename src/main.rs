mod crypto;
mod filepacker;
mod gitignore;

use crate::crypto::{Key, KeySource};
use crate::filepacker::{tar_directory, EnvironmentPack};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "envcaja")]
#[command(about = "File-based secret manager for CI/CD pipelines", long_about = None)]
struct Cli {
    /// Path to the keyfile
    #[arg(long, default_value = "safe.key")]
    keyfile: PathBuf,

    #[arg(long, env = "CI_SECRET")]
    key: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Shows info
    Info {},

    /// Initializes repository. Creates a new key and adds entries to .gitignore
    Init {},

    /// Encrypt the environment and stores everything in one file
    Encrypt {
        /// path to .env file
        #[arg(long, default_value = ".env")]
        dotenv: PathBuf,

        /// path to env.enc file
        #[arg(long, default_value = "env.enc")]
        vault: PathBuf,
    },

    /// Decrypts the environment
    Decrypt {
        /// path to .env file
        #[arg(long, default_value = ".env")]
        dotenv: PathBuf,

        /// path to env.enc file
        #[arg(long, default_value = "env.enc")]
        vault: PathBuf,
    },
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
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
            println!("Key written to {:?}", cli.keyfile);

            println!("Excluding secret files using \".gitignore\".");
            gitignore::gitignore();
            Ok(())
        }
        Commands::Info {} => {
            match Key::load_key(&cli.key, &cli.keyfile) {
                Ok((key, key_source)) => {
                    log_key_source(key_source);
                    println!("Successfully loaded the key:");
                    println!("CI_SECRET=\"{}\"", key.to_base64());
                }
                Err(err) => eprintln!("{}", err),
            }
            Ok(())
        }
        Commands::Encrypt { dotenv, vault } => {
            let (key, key_source) = Key::load_key(&cli.key, cli.keyfile.as_path())?;
            log_key_source(key_source);

            let dotenv = Path::new(dotenv);
            let vault = Path::new(vault);

            let pack = EnvironmentPack::from_path(dotenv)?;
            let ciphertext = key.encrypt_base64(&pack)?;

            // Write the ciphertext to output file
            fs::write(vault, ciphertext)?;

            println!("Encrypted content successfully written to {:?}", vault);
            Ok(())
        }
        Commands::Decrypt { dotenv, vault } => {
            let (key, key_source) = Key::load_key(&cli.key, cli.keyfile.as_path())?;
            log_key_source(key_source);
            let ciphertext = fs::read_to_string(vault)?;
            let pack = key.decrypt_base64(&ciphertext)?;
            pack.unpack(dotenv)?;

            println!("Decrypted content successfully written to {:?}", dotenv);
            Ok(())
        }
    }
}

fn log_key_source(key_source: KeySource) {
    match key_source {
        KeySource::File(key_file) => {
            println!("Key was loaded from {:?}", key_file)
        }
        KeySource::Env => {
            println!("Key was loaded from CI_SECRET")
        }
    }
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
