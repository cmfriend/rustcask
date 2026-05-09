mod database;
#[cfg(feature = "mock")]
mod mock_database;
#[cfg(not(feature = "mock"))]
mod rustcask_database;
mod settings;

use database::{Database, Error};
use dotenv::dotenv;
#[cfg(feature = "mock")]
use mock_database::*;
#[cfg(not(feature = "mock"))]
use rustcask_database::*;
use settings::*;
use std::{
    env,
    io::{self, Write},
    path::PathBuf,
};
use tracing_subscriber::EnvFilter;

const DEFAULT_RUSTCASK_ROTATE_ACTIVE_FILE_AFTER_BYTES: u64 = 4194304;

#[derive(Debug)]
enum ReplError {
    FlushError(std::io::Error),
    ParseError(std::string::FromUtf8Error),
    SettingsError(String),
    DatabaseError(Error),
}

fn main() -> Result<(), ReplError> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("rustcask=debug".parse().unwrap()),
        )
        .init();

    let settings = load_settings()?;

    tracing::debug!(?settings, "loaded settings");

    #[cfg(feature = "mock")]
    let db = MockDatabase::open();
    #[cfg(not(feature = "mock"))]
    let db = RustcaskDatabase::open(settings.file_path, settings.rotate_active_file_after_bytes)
        .map_err(|e| ReplError::DatabaseError(e))?;

    cmd_loop(db)
}

#[tracing::instrument]
fn load_settings() -> Result<Settings, ReplError> {
    // Load environment variables, exit with error if required ones are missing
    let file_path_env_value =
        env::var("RUSTCASK_FILE_PATH").expect("RUSTCASK_FILE_PATH must be set to a string");
    let file_path = PathBuf::from(file_path_env_value);

    if !file_path.is_absolute() || !file_path.exists() {
        return Err(ReplError::SettingsError(format!(
            "Environment variable RUSTCASK_FILE_PATH did not parse into an existing absolute file path: {:?}",
            file_path
        )));
    }

    let rotate_active_file_after_bytes_env_value =
        env::var("RUSTCASK_ROTATE_ACTIVE_FILE_AFTER_BYTES")
            .map_or(Ok(DEFAULT_RUSTCASK_ROTATE_ACTIVE_FILE_AFTER_BYTES), |v| {
                v.parse::<u64>()
            })
            .expect("RUSTCASK_ROTATE_ACTIVE_FILE_AFTER_BYTES did not parse into a u64");

    Ok(Settings::new(
        file_path,
        rotate_active_file_after_bytes_env_value,
    ))
}

#[tracing::instrument(skip(db))]
fn cmd_loop(mut db: impl Database) -> Result<(), ReplError> {
    tracing::info!("starting REPL");

    // Enter CLI loop
    println!("Rustcask REPL.  Enter 'exit' to exit.");
    loop {
        print!("> ");
        io::stdout().flush().map_err(|e| ReplError::FlushError(e))?; // exit on flush error

        // Wait for input up to /n
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            tracing::warn!("failed to read input line");
            continue;
        }

        // Parse input
        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.is_empty() {
            // Just display the prompt again
            continue;
        }

        // Handle input
        match parts[0].to_lowercase().as_str() {
            "compact" => {
                if parts.len() != 1 {
                    tracing::warn!("Invalid input for compact: {input}");
                    continue;
                }

                println!("Compacting...");

                if let Err(e) = db.compact() {
                    tracing::error!(?e, "compact failed")
                }
            }
            "get" => {
                if parts.len() != 2 {
                    tracing::warn!("Invalid input for get: {input}");
                    continue;
                }

                let key = parts[1].as_bytes();

                match db.get(key) {
                    Ok(v) => {
                        let s = String::from_utf8(v).map_err(|e| ReplError::ParseError(e))?;
                        println!("{s}");
                    }
                    Err(database::Error::KeyMissing) => {
                        tracing::debug!("get on missing key: {:?}", String::from_utf8_lossy(key))
                    }
                    Err(e) => {
                        tracing::error!(?e, "get failed")
                    }
                }
            }
            "put" => {
                if parts.len() != 3 {
                    tracing::warn!("Invalid input for put: {input}");
                    continue;
                }

                let key = parts[1].as_bytes();
                let value = parts[2].as_bytes();

                if let Err(e) = db.put(key, value) {
                    tracing::error!(?e, "put failed")
                }
            }
            "delete" => {
                if parts.len() != 2 {
                    tracing::warn!("Invalid input for delete: {input}");
                    continue;
                }

                let key = parts[1].as_bytes();

                if let Err(e) = db.delete(key) {
                    tracing::error!(?e, "delete failed")
                }
            }
            "exit" => {
                // Ignore the rest of the input line and assume the user wants to exit

                return Ok(());
            }
            _ => {
                tracing::warn!("Invalid input: {input}");
            }
        }
    }
}
