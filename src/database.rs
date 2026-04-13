pub trait Database {
    fn compact(&mut self) -> Result<(), Error>;

    fn get(&mut self, key: &[u8]) -> Result<Vec<u8>, Error>;

    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error>;

    fn delete(&mut self, key: &[u8]) -> Result<(), Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Provided key was empty")]
    EmptyKey,

    #[error("Provided value was empty")]
    EmptyValue,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Log data file header was not valid")]
    InvalidHeader,

    #[error("The provided key was missing")]
    KeyMissing,

    #[error("Supplied path is not a directory: {0}")]
    NotDir(String),

    #[error("Timestamp error: {0}")]
    TimestampError(#[from] std::time::SystemTimeError),

    #[error("Timestamp cannot be stored as u64: {0}")]
    TimestampOverflow(#[from] std::num::TryFromIntError),
}
