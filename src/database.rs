pub trait Database {
    fn compact(&mut self) -> Result<(), Error>;

    fn get(&self, key: &[u8]) -> Result<Vec<u8>, Error>;

    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error>;

    fn delete(&mut self, key: &[u8]) -> Result<(), Error>;
}

#[derive(Debug)]
pub enum Error {
    IO(String),
    KeyMissing,
}
