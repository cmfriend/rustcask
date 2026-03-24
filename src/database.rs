use std::collections::HashMap;

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

pub struct RustcaskDatabase {
    keydir: HashMap<Vec<u8>, Entry>,
}

struct Entry {
    file_id: u32,   // name of the file in the data directory, timestamp seconds since epoch
    value_pos: u64, // byte offset position within file
    value_sz: u32,  // size of the value in bytes
    tstamp: u32,    // timestamp when entry was written in seconds since epoch
}

impl Database for RustcaskDatabase {
    fn compact(&mut self) -> Result<(), Error> {
        todo!()
    }

    fn get(&self, key: &[u8]) -> Result<Vec<u8>, Error> {
        todo!()
    }

    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        todo!()
    }

    fn delete(&mut self, key: &[u8]) -> Result<(), Error> {
        todo!()
    }
}

impl RustcaskDatabase {
    pub fn new() -> Self {
        Self {
            keydir: HashMap::new(),
        }
    }
}

// Mock database for CLI testing

pub struct MockDatabase {
    // No file storage, just the in-memory keydir
    keydir: HashMap<Vec<u8>, Vec<u8>>,
}

impl Database for MockDatabase {
    fn compact(&mut self) -> Result<(), Error> {
        Ok(()) // No file storage for mock database
    }

    fn get(&self, key: &[u8]) -> Result<Vec<u8>, Error> {
        self.keydir
            .get(key)
            .map(|v| v.clone())
            .ok_or(Error::KeyMissing)
    }

    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.keydir.insert(Vec::from(key), Vec::from(value));
        Ok(())
    }

    fn delete(&mut self, key: &[u8]) -> Result<(), Error> {
        self.keydir.remove(key);
        Ok(())
    }
}

impl MockDatabase {
    pub fn new() -> Self {
        Self {
            keydir: HashMap::new(),
        }
    }
}
