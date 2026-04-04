use std::path::PathBuf;

#[derive(Debug)]
pub struct Settings {
    pub file_path: PathBuf,
    pub max_file_size: u8, // Max file size in MB, defaults to 4 MB
}

impl Settings {
    pub fn new(file_path: PathBuf, max_file_size: u8) -> Self {
        Self {
            file_path,
            max_file_size,
        }
    }
}
