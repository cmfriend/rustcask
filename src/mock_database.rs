// Mock database for CLI testing
use std::collections::HashMap;

use crate::database::*;

pub struct MockDatabase {
    // No file storage, just the in-memory keydir
    keydir: HashMap<Vec<u8>, Vec<u8>>,
}

impl Database for MockDatabase {
    #[tracing::instrument(skip(self))]
    fn compact(&mut self) -> Result<(), Error> {
        Ok(()) // No file storage for mock database
    }

    #[tracing::instrument(skip(self), fields(key = %String::from_utf8_lossy(key)))]
    fn get(&mut self, key: &[u8]) -> Result<Vec<u8>, Error> {
        self.keydir
            .get(key)
            .map(|v| v.clone())
            .ok_or(Error::KeyMissing)
    }

    #[tracing::instrument(skip(self, value), fields(key = %String::from_utf8_lossy(key)))]
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        let _ = self.keydir.insert(Vec::from(key), Vec::from(value));
        Ok(())
    }

    #[tracing::instrument(skip(self), fields(key = %String::from_utf8_lossy(key)))]
    fn delete(&mut self, key: &[u8]) -> Result<(), Error> {
        let _ = self.keydir.remove(key);
        Ok(())
    }
}

impl MockDatabase {
    pub fn open() -> Self {
        Self {
            keydir: HashMap::new(),
        }
    }
}
