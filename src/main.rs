mod database;
mod settings;

use database::*;
use settings::*;
use std::{
    env,
    io::{self, Write},
    path::PathBuf,
};

fn main() {
    let settings = load_settings();

    println!("Provided settings: {:?}", settings);

    // Build database instance
    // TODO: Replace with file backed database when ready
    let db = MockDatabase::new();

    cmd_loop(db);
}

fn load_settings() -> Settings {
    // Load environment variables, exit with error if required ones are missing
    let file_path_env_value =
        env::var("RUSTCASK_FILE_PATH").expect("RUSTCASK_FILE_PATH must be set to a string");
    let file_path = PathBuf::from(file_path_env_value);

    if !file_path.is_absolute() || !file_path.exists() {
        panic!(
            "Environment variable RUSTCASK_FILE_PATH did not parse into an existing absolute file path: {:?}",
            file_path
        );
    }

    let max_file_size_env_value = env::var("RUSTCASK_MAX_FILE_SIZE")
        .unwrap_or("4".into())
        .parse::<u8>()
        .expect("RUSTCASK_MAX_FILE_SIZE did not parse into a u8");

    Settings::new(file_path, max_file_size_env_value)
}

fn cmd_loop(mut db: impl Database) {
    // Enter CLI loop, use ctrl-c to exit
    println!("Rustcask REPL.  Press Ctrl-C to exit.");
    loop {
        print!("> ");
        io::stdout().flush().expect("Problem flushing to stdout");

        // Wait for input up to /n
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            eprintln!("Bad input");
            continue;
        }

        // Parse input
        let parts: Vec<&str> = input.split_whitespace().collect();

        // Handle input
        match parts[0].to_lowercase().as_str() {
            "compact" => {
                // TODO: Implement real compaction
                println!("Compacting...");
            }
            "get" => {
                let key = parts[1].as_bytes();

                match db.get(key) {
                    Ok(v) => {
                        // panic here if bytes are not a valid string, which shouldn't be able to happen with this REPL
                        let s = String::from_utf8(v).expect("invalid UTF-8");
                        println!("{s}");
                    }
                    Err(Error::KeyMissing) => {}
                    Err(e) => {
                        eprintln!("{:?}", e);
                    }
                }
            }
            "put" => {
                let key = parts[1].as_bytes();
                let value = parts[2].as_bytes();

                if let Err(e) = db.put(key, value) {
                    eprintln!("{:?}", e);
                }
            }
            "delete" => {
                let key = parts[1].as_bytes();

                if let Err(e) = db.delete(key) {
                    eprintln!("{:?}", e);
                }
            }
            _ => {
                println!("Invalid input");
            }
        }
    }
}
