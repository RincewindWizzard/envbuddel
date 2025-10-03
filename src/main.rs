mod crypto;
mod filepacker;
mod gitignore;

use crate::crypto::{Key, KeySource};
use crate::filepacker::EnvironmentPack;
use clap::{Parser, Subcommand};
use log::{error, info, warn};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "envbuddel")]
#[command(about = "File-based secret manager for CI/CD pipelines", long_about = None)]
struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Path to the keyfile
    #[arg(long, default_value = "safe.key")]
    keyfile: PathBuf,

    /// Content of the key
    #[arg(long, env = "CI_SECRET")]
    key: Option<String>,

    /// path to .env file or folder
    #[arg(long, default_value = ".env")]
    env_conf: PathBuf,

    /// path to the vault file
    #[arg(long, default_value = "env.enc")]
    vault: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Shows debug info (key, vault, environment path) to the supplied options
    Info {},

    /// Initializes repository. Creates a new key, environment and vault and adds entries to .gitignore
    Init {
        /// Creates folder instead of a single file for the environment
        #[arg(long)]
        folder: bool,
    },

    /// Encrypt the environment and stores everything in the vault
    Encrypt {},

    /// Decrypts the environment from the vault and unpacks them to --env-conf path
    Decrypt {},
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match &cli.command {
        Commands::Init { folder } => {
            let key = Key::generate();
            info!("Please run this to provide the key as environment variable:\n");
            info!("  $ export CI_SECRET=\"{}\"", key.to_base64());
            info!("");

            key.save_key(&cli.keyfile)?;
            info!("Key written to {:?}", cli.keyfile);

            info!("Excluding secret files using \".gitignore\".");
            gitignore::gitignore();

            if *folder {
                fs::create_dir_all(&cli.env_conf)?;
                info!("Created folder {:?}", cli.env_conf);
            } else {
                fs::write(&cli.env_conf, "")?;
            }

            let pack = EnvironmentPack::from_path(&cli.env_conf)?;
            let ciphertext = key.encrypt_base64(&pack)?;

            // Write the ciphertext to output file
            fs::write(&cli.vault, ciphertext)?;

            info!("Encrypted content successfully written to {:?}", cli.vault);

            Ok(())
        }
        Commands::Info {} => {
            if let Some(key) = cli.key.clone() {
                match Key::load_key(&Some(key), Path::new("/dev/null")) {
                    Ok((key, _)) => {
                        info!("CI_SECRET=\"{}\"", key.to_base64());
                    }
                    Err(_) => {
                        warn!("Failed to load private key from CI_SECRET=\"{}\". It needs to be 32 bytes encoded as base64!", cli.key.clone().unwrap_or("".to_string()));
                    }
                }
            }

            match Key::load_key(&None, &cli.keyfile) {
                Ok((key, _)) => {
                    info!(
                        "Key contained in {:?}: \"{}\"",
                        &cli.keyfile,
                        key.to_base64()
                    );
                }
                Err(err) => error!("{}", err),
            }

            let (key, _) = Key::load_key(&cli.key, &cli.keyfile)?;
            if cli.vault.exists() && cli.vault.is_file() {
                info!("Vault files exist.");
                let ciphertext = fs::read_to_string(cli.vault)?;
                let _ = key.decrypt_base64(&ciphertext)?;
                info!("Successfully decrypted vault file.");
            } else {
                warn!("No vault file detected!");
            }

            if cli.env_conf.exists() {
                if cli.env_conf.is_file() {
                    info!("Environment configuration file found.");
                } else if cli.env_conf.is_dir() {
                    info!("Environment configuration folder found.");
                } else {
                    warn!("Environment configuration is neither file nor folder!");
                }
            } else {
                warn!("Environment configuration file/folder does not exists!");
            }

            Ok(())
        }
        Commands::Encrypt {} => {
            let (key, key_source) = Key::load_key(&cli.key, cli.keyfile.as_path())?;
            log_key_source(key_source);

            let pack = EnvironmentPack::from_path(&cli.env_conf)?;
            let ciphertext = key.encrypt_base64(&pack)?;

            // Write the ciphertext to output file
            fs::write(&cli.vault, ciphertext)?;

            info!("Encrypted content successfully written to {:?}", cli.vault);
            Ok(())
        }
        Commands::Decrypt {} => {
            let (key, key_source) = Key::load_key(&cli.key, cli.keyfile.as_path())?;
            log_key_source(key_source);
            let ciphertext = fs::read_to_string(cli.vault)?;
            let pack = key.decrypt_base64(&ciphertext)?;
            pack.unpack(&cli.env_conf)?;

            info!(
                "Decrypted content successfully written to {:?}",
                cli.env_conf
            );
            Ok(())
        }
    }
}

fn log_key_source(key_source: KeySource) {
    match key_source {
        KeySource::File(key_file) => {
            info!("Key was loaded from {:?}", key_file)
        }
        KeySource::Env => {
            info!("Key was loaded from CI_SECRET")
        }
    }
}

fn init_logger(verbosity: u8) {
    use env_logger::{Builder, Target};
    use std::io::Write;

    Builder::new()
        .format(|buf, record| {
            let msg = format!("{}", record.args());
            match record.level() {
                log::Level::Warn => writeln!(buf, "\x1b[33m[WARN] {}\x1b[0m", msg), // red
                log::Level::Error => writeln!(buf, "\x1b[91m[ERROR] {}\x1b[0m", msg), // bright red
                _ => writeln!(buf, "{}", msg),                                      // default
            }
        })
        .target(Target::Stdout)
        .filter_level(match verbosity {
            0 => log::LevelFilter::Info,  // always show info & higher
            1 => log::LevelFilter::Debug, // debug + info + warn + error
            _ => log::LevelFilter::Trace, // trace + debug + info + warn + error
        })
        .init();
}

fn main() {
    let cli = Cli::parse();
    init_logger(cli.verbose);
    if let Err(err) = run(cli) {
        error!("{}", err);
        std::process::exit(1);
    }
}
