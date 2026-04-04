use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, ErrorKind, Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::database::*;

const ACTIVE_FILE_NAME: &str = "0";
const ACTIVE_FILE_NAME_ID: u32 = 0u32;

const FILE_HEADER_IDENTIFIER: &[u8] = b"rustcask";
const FILE_HEADER_VERSION: &[u8] = &[0x01, 0x00];
const FILE_HEADER_RESERVED: &[u8] = &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
const FILE_HEADER_SIZE: usize =
    FILE_HEADER_IDENTIFIER.len() + FILE_HEADER_VERSION.len() + FILE_HEADER_RESERVED.len();

const ENTRY_HEADER_SIZE: usize = std::mem::size_of::<u32>()
    + std::mem::size_of::<u64>()
    + std::mem::size_of::<u32>()
    + std::mem::size_of::<u32>()
    + std::mem::size_of::<u8>();

pub struct RustcaskDatabase {
    reader: BufReader<File>,
    writer: BufWriter<File>,
    keydir: HashMap<Vec<u8>, KeyDirEntry>,
}

#[derive(Debug, Clone, Copy)]
struct FileId(u32);

#[derive(Debug, Clone, Copy)]
struct ValuePosition(u64);

#[derive(Debug, Clone, Copy)]
struct ValueSize(u32);

#[derive(Debug, Clone, Copy)]
struct Timestamp(u64);

#[derive(Debug)]
struct KeyDirEntry {
    file_id: FileId, // name of the file in the data directory, timestamp seconds since epoch, or zero for the currently active file
    value_position: ValuePosition, // byte offset position within file
    value_size: ValueSize, // size of the value in bytes
    timestamp: Timestamp, // timestamp when entry was written in milliseconds since epoch
}

struct Header {
    version: u16,
    reserved: [u8; 6],
}

fn write_header(writer: &mut impl Write) -> Result<(), Error> {
    writer.write_all(FILE_HEADER_IDENTIFIER)?;
    writer.write_all(FILE_HEADER_VERSION)?;
    writer.write_all(FILE_HEADER_RESERVED)?;
    writer.flush()?;

    Ok(())
}

fn parse_header<R: Read + Seek>(r: &mut R) -> Result<Header, Error> {
    let mut buf = [0u8; FILE_HEADER_SIZE];
    let _ = r.read_exact(&mut buf).map_err(|e| {
        if e.kind() == ErrorKind::UnexpectedEof {
            Error::InvalidHeader
        } else {
            Error::Io(e)
        }
    })?;
    let _ = r.rewind()?;

    let identifier = &buf[0..8];
    let version = &buf[8..10];
    let reserved = &buf[10..16];

    if identifier == FILE_HEADER_IDENTIFIER
        && version == FILE_HEADER_VERSION
        && reserved == FILE_HEADER_RESERVED
    {
        Ok(Header {
            version: u16::from_le_bytes(version.try_into().unwrap()),
            reserved: reserved.try_into().unwrap(),
        })
    } else {
        Err(Error::InvalidHeader)
    }
}

fn build_file_entry_buffer(key: &[u8], value: &[u8], timestamp: Timestamp) -> Vec<u8> {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(key);
    hasher.update(value);
    let crc = hasher.finalize();

    let key_size = key.len() as u32;

    let value_size = value.len() as u32;

    let flags = if value.is_empty() { 1u8 } else { 0u8 }; // TODO: use bitflags crate

    let mut buffer = Vec::with_capacity(ENTRY_HEADER_SIZE + key.len() + value.len());

    buffer.extend_from_slice(&crc.to_le_bytes());
    buffer.extend_from_slice(&timestamp.0.to_le_bytes());
    buffer.extend_from_slice(&key_size.to_le_bytes());
    buffer.extend_from_slice(&value_size.to_le_bytes());
    buffer.extend_from_slice(&flags.to_le_bytes());
    buffer.extend_from_slice(key);
    buffer.extend_from_slice(value);

    buffer
}

impl Database for RustcaskDatabase {
    #[tracing::instrument(skip(self))]
    fn compact(&mut self) -> Result<(), Error> {
        tracing::debug!("Compacting...");

        // TODO: Implement compaction

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(key = %String::from_utf8_lossy(key)))]
    fn get(&mut self, key: &[u8]) -> Result<Vec<u8>, Error> {
        if key.is_empty() {
            return Err(Error::EmptyKey);
        }

        let entry = self.keydir.get(key).ok_or(Error::KeyMissing)?;

        let _ = self.reader.seek(SeekFrom::Start(entry.value_position.0))?;

        let mut value = vec![0; entry.value_size.0 as usize];

        let _ = self.reader.read_exact(&mut value)?;

        Ok(value)
    }

    #[tracing::instrument(skip(self, value), fields(key = %String::from_utf8_lossy(key)))]
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        if key.is_empty() {
            return Err(Error::EmptyKey);
        }

        if value.is_empty() {
            return Err(Error::EmptyValue);
        }

        let timestamp = Timestamp(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(Error::TimestampError)?
                .as_millis()
                .try_into()
                .map_err(Error::TimestampOverflow)?,
        );

        let buffer = build_file_entry_buffer(key, value, timestamp);

        let position = self.writer.seek(SeekFrom::End(0))?;
        let _ = self.writer.write_all(&buffer)?;
        let _ = self.writer.flush()?;

        let value_position = position + ENTRY_HEADER_SIZE as u64 + key.len() as u64;

        let _ = self.keydir.insert(
            Vec::from(key),
            KeyDirEntry {
                file_id: FileId(ACTIVE_FILE_NAME_ID), // using only the active file for now
                value_position: ValuePosition(value_position),
                value_size: ValueSize(value.len() as u32),
                timestamp,
            },
        );

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(key = %String::from_utf8_lossy(key)))]
    fn delete(&mut self, key: &[u8]) -> Result<(), Error> {
        if key.is_empty() {
            return Err(Error::EmptyKey);
        }

        let timestamp = Timestamp(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(Error::TimestampError)?
                .as_millis()
                .try_into()
                .map_err(Error::TimestampOverflow)?,
        );

        let buffer = build_file_entry_buffer(key, &Vec::new(), timestamp);

        let _ = self.writer.seek(SeekFrom::End(0))?;
        let _ = self.writer.write_all(&buffer)?;
        let _ = self.writer.flush()?;

        let _ = self.keydir.remove(key);

        Ok(())
    }
}

impl RustcaskDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();

        // It is assumed that `path` contains only rustcask data files.
        // The files are named either `0` for the current actively written file,
        // or filenames consisting of a string representation of the unix epoch in seconds,
        // when that file was created after file rotation.

        let path_stat = fs::metadata(path)?;

        if !path_stat.is_dir() {
            return Err(Error::NotDir(path.to_str().map_or(
                "Cannot represent supplied path as Rust String".into(),
                |s| s.into(),
            )));
        }

        let active_file_path = path.join(ACTIVE_FILE_NAME);

        if active_file_path.exists() {
            tracing::debug!(
                "Active file found {:?}, checking format...",
                active_file_path
            );

            let mut read_file = OpenOptions::new()
                .create(false)
                .read(true)
                .truncate(false)
                .append(false)
                .open(&active_file_path)?;

            let _ = parse_header(&mut read_file)?;

            let write_file = OpenOptions::new()
                .create(false)
                .read(true)
                .truncate(false)
                .append(true)
                .open(&active_file_path)?;

            // TODO: Rebuild self.keydir from existing log files
            let keydir = HashMap::new();

            Ok(Self {
                reader: BufReader::new(read_file),
                writer: BufWriter::new(write_file),
                keydir: keydir,
            })
        } else {
            tracing::debug!("Active file not found {:?}, creating...", active_file_path);

            let read_file = OpenOptions::new()
                .create(true)
                .read(true)
                .truncate(false)
                .append(true)
                .open(&active_file_path)?;

            let reader = BufReader::new(read_file);

            let write_file = OpenOptions::new()
                .create(true)
                .read(true)
                .truncate(false)
                .append(true)
                .open(&active_file_path)?;

            let mut writer = BufWriter::new(write_file);

            let _ = write_header(&mut writer)?;

            Ok(Self {
                reader: reader,
                writer: writer,
                keydir: HashMap::new(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;

    #[test]
    fn test_write_header() {
        let mut buf = Vec::new();

        let result = write_header(&mut buf);

        assert!(result.is_ok());

        let mut target_buf = Vec::new();
        target_buf.extend_from_slice(FILE_HEADER_IDENTIFIER);
        target_buf.extend_from_slice(FILE_HEADER_VERSION);
        target_buf.extend_from_slice(FILE_HEADER_RESERVED);

        assert_eq!(buf, target_buf);
    }

    #[test]
    fn test_parse_header_good_header() {
        let mut data = Vec::new();
        data.extend_from_slice(FILE_HEADER_IDENTIFIER);
        data.extend_from_slice(FILE_HEADER_VERSION);
        data.extend_from_slice(FILE_HEADER_RESERVED);
        let mut cursor = Cursor::new(data);
        let result = parse_header(&mut cursor);

        let valid_header = Header {
            version: u16::from_le_bytes(FILE_HEADER_VERSION.try_into().unwrap()),
            reserved: FILE_HEADER_RESERVED.try_into().unwrap(),
        };

        assert!(matches!(result, Ok(valid_header)));
    }

    #[test]
    fn test_parse_header_too_short() {
        let data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
        let mut cursor = Cursor::new(data);
        let result = parse_header(&mut cursor);

        assert!(matches!(result, Err(Error::InvalidHeader)));
    }

    #[test]
    fn test_parse_header_wrong_sequence() {
        let data = vec![0x00; 42];
        let mut cursor = Cursor::new(data);
        let result = parse_header(&mut cursor);

        assert!(matches!(result, Err(Error::InvalidHeader)));
    }

    #[test]
    fn test_build_file_entry_buffer_put() {
        let key = "key12345";
        let key_bytes = key.as_bytes();
        let value = "value67890";
        let value_bytes = value.as_bytes();
        let timestamp = Timestamp(123456789u64);

        let buffer = build_file_entry_buffer(key_bytes, value_bytes, timestamp);

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(key_bytes);
        hasher.update(value_bytes);
        let crc = hasher.finalize();

        let key_size = key.len() as u32;

        let value_size = value.len() as u32;

        let flags = 0u8;

        let mut target_buffer = Vec::with_capacity(ENTRY_HEADER_SIZE + key.len() + value.len());

        target_buffer.extend_from_slice(&crc.to_le_bytes());
        target_buffer.extend_from_slice(&timestamp.0.to_le_bytes());
        target_buffer.extend_from_slice(&key_size.to_le_bytes());
        target_buffer.extend_from_slice(&value_size.to_le_bytes());
        target_buffer.extend_from_slice(&flags.to_le_bytes());
        target_buffer.extend_from_slice(key_bytes);
        target_buffer.extend_from_slice(value_bytes);

        assert_eq!(buffer, target_buffer);
    }

    #[test]
    fn test_build_file_entry_buffer_tombstone() {
        let key = "key54321";
        let key_bytes = key.as_bytes();
        let value_bytes = Vec::new();
        let timestamp = Timestamp(123456789u64);

        let buffer = build_file_entry_buffer(key_bytes, &value_bytes, timestamp);

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(key_bytes);
        hasher.update(&value_bytes);
        let crc = hasher.finalize();

        let key_size = key.len() as u32;

        let value_size = 0u32;

        let flags = 1u8;

        let mut target_buffer = Vec::with_capacity(ENTRY_HEADER_SIZE + key.len());

        target_buffer.extend_from_slice(&crc.to_le_bytes());
        target_buffer.extend_from_slice(&timestamp.0.to_le_bytes());
        target_buffer.extend_from_slice(&key_size.to_le_bytes());
        target_buffer.extend_from_slice(&value_size.to_le_bytes());
        target_buffer.extend_from_slice(&flags.to_le_bytes());
        target_buffer.extend_from_slice(key_bytes);
        target_buffer.extend_from_slice(&value_bytes);

        assert_eq!(buffer, target_buffer);
    }
}
