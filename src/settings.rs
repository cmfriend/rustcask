use std::path::PathBuf;

#[derive(Debug)]
pub struct Settings {
    pub file_path: PathBuf,
    pub rotate_active_file_after_bytes: u64,
}

impl Settings {
    pub fn new(file_path: PathBuf, rotate_active_file_after_bytes: u64) -> Self {
        Self {
            file_path,
            rotate_active_file_after_bytes,
        }
    }
}
