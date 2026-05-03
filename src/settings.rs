use std::path::PathBuf;

#[derive(Debug)]
pub struct Settings {
    pub file_path: PathBuf,
    pub max_file_size: u64,
}

impl Settings {
    pub fn new(file_path: PathBuf, max_file_size: u64) -> Self {
        Self {
            file_path,
            max_file_size,
        }
    }
}
