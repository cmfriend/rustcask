use std::collections::HashMap;

use crate::database::*;

pub struct RustcaskDatabase {
    keydir: HashMap<Vec<u8>, Entry>,
}

#[derive(Debug, Clone, Copy)]
struct FileId(u32);

#[derive(Debug, Clone, Copy)]
struct ValuePosition(u64);

#[derive(Debug, Clone, Copy)]
struct ValueSize(u32);

#[derive(Debug, Clone, Copy)]
struct Timestamp(u32);

struct Entry {
    file_id: FileId, // name of the file in the data directory, timestamp seconds since epoch
    value_pos: ValuePosition, // byte offset position within file
    value_sz: ValueSize, // size of the value in bytes
    tstamp: Timestamp, // timestamp when entry was written in seconds since epoch
}

impl Database for RustcaskDatabase {
    #[tracing::instrument(skip(self))]
    fn compact(&mut self) -> Result<(), Error> {
        todo!()
    }

    #[tracing::instrument(skip(self), fields(key = %String::from_utf8_lossy(key)))]
    fn get(&self, key: &[u8]) -> Result<Vec<u8>, Error> {
        todo!()
    }

    #[tracing::instrument(skip(self, value), fields(key = %String::from_utf8_lossy(key)))]
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        todo!()
    }

    #[tracing::instrument(skip(self), fields(key = %String::from_utf8_lossy(key)))]
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
