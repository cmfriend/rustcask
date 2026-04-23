use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, ErrorKind, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use crate::database::*;

const ACTIVE_FILE_NAME: &str = "0";
const ACTIVE_FILE_NAME_ID: u32 = 0u32;

const FILE_HEADER_IDENTIFIER_BYTES: &[u8] = b"rustcask";
const FILE_HEADER_VERSION_BYTES: &[u8] = &[0x01, 0x00];
const FILE_HEADER_RESERVED_BYTES: &[u8] = &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
const FILE_HEADER_SIZE: usize = FILE_HEADER_IDENTIFIER_BYTES.len()
    + FILE_HEADER_VERSION_BYTES.len()
    + FILE_HEADER_RESERVED_BYTES.len();
const FILE_HEADER_IDENTIFIER_BYTES_OFFSET: usize = 0;
const FILE_HEADER_VERSION_BYTES_OFFSET: usize =
    FILE_HEADER_IDENTIFIER_BYTES_OFFSET + FILE_HEADER_IDENTIFIER_BYTES.len();
const FILE_HEADER_RESERVED_BYTES_OFFSET: usize =
    FILE_HEADER_VERSION_BYTES_OFFSET + FILE_HEADER_VERSION_BYTES.len();

const FILE_ENTRY_HEADER_CRC_OFFSET: usize = 0;
const FILE_ENTRY_HEADER_TIMESTAMP_OFFSET: usize =
    FILE_ENTRY_HEADER_CRC_OFFSET + std::mem::size_of::<Crc>();
const FILE_ENTRY_HEADER_KEY_SIZE_OFFSET: usize =
    FILE_ENTRY_HEADER_TIMESTAMP_OFFSET + std::mem::size_of::<Timestamp>();
const FILE_ENTRY_HEADER_VALUE_SIZE_OFFSET: usize =
    FILE_ENTRY_HEADER_KEY_SIZE_OFFSET + std::mem::size_of::<KeySize>();
const FILE_ENTRY_HEADER_FLAGS_OFFSET: usize =
    FILE_ENTRY_HEADER_VALUE_SIZE_OFFSET + std::mem::size_of::<ValueSize>();

pub struct RustcaskDatabase {
    reader: BufReader<File>,
    writer: BufWriter<File>,
    keydir: HashMap<Vec<u8>, KeyDirEntry>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct FileId(u32);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct ValuePosition(u64);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct ValueSize(u32);

// timestamp in milliseconds since unix epoch
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Timestamp(u64);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct KeyDirEntry {
    file_id: FileId, // name of the file in the data directory, timestamp seconds since epoch, or zero for the currently active file
    value_position: ValuePosition, // byte offset position within file
    value_size: ValueSize, // size of the value in bytes
    timestamp: Timestamp, // timestamp when entry was written in milliseconds since epoch
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct FileHeaderVersion(u16);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct FileHeaderReserved([u8; FILE_HEADER_RESERVED_BYTES.len()]);

struct FileHeader {
    version: FileHeaderVersion,
    reserved: FileHeaderReserved,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Crc(u32);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct KeySize(u32);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Flags(u8); // TODO: use bitflags crate

const FILE_ENTRY_HEADER_SERIALIZED_SIZE: usize = std::mem::size_of::<Crc>()
    + std::mem::size_of::<Timestamp>()
    + std::mem::size_of::<KeySize>()
    + std::mem::size_of::<ValueSize>()
    + std::mem::size_of::<Flags>();

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct FileEntryHeader {
    crc: Crc,
    timestamp: Timestamp,
    key_size: KeySize,
    value_size: ValueSize,
    flags: Flags,
}

impl FileEntryHeader {
    fn deserialize(le_bytes: [u8; FILE_ENTRY_HEADER_SERIALIZED_SIZE]) -> Result<Self, Error> {
        Ok(Self {
            crc: Crc(u32::from_le_bytes(
                le_bytes[FILE_ENTRY_HEADER_CRC_OFFSET..FILE_ENTRY_HEADER_TIMESTAMP_OFFSET]
                    .try_into()?,
            )),
            timestamp: Timestamp(u64::from_le_bytes(
                le_bytes[FILE_ENTRY_HEADER_TIMESTAMP_OFFSET..FILE_ENTRY_HEADER_KEY_SIZE_OFFSET]
                    .try_into()?,
            )),
            key_size: KeySize(u32::from_le_bytes(
                le_bytes[FILE_ENTRY_HEADER_KEY_SIZE_OFFSET..FILE_ENTRY_HEADER_VALUE_SIZE_OFFSET]
                    .try_into()?,
            )),
            value_size: ValueSize(u32::from_le_bytes(
                le_bytes[FILE_ENTRY_HEADER_VALUE_SIZE_OFFSET..FILE_ENTRY_HEADER_FLAGS_OFFSET]
                    .try_into()?,
            )),
            flags: Flags(u8::from_le_bytes(
                le_bytes[FILE_ENTRY_HEADER_FLAGS_OFFSET..FILE_ENTRY_HEADER_SERIALIZED_SIZE]
                    .try_into()?,
            )),
        })
    }

    fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(FILE_ENTRY_HEADER_SERIALIZED_SIZE);

        buffer.extend_from_slice(&self.crc.0.to_le_bytes());
        buffer.extend_from_slice(&self.timestamp.0.to_le_bytes());
        buffer.extend_from_slice(&self.key_size.0.to_le_bytes());
        buffer.extend_from_slice(&self.value_size.0.to_le_bytes());
        buffer.extend_from_slice(&self.flags.0.to_le_bytes());

        buffer
    }
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

        let buffer = Self::build_file_entry_buffer(key, value, timestamp);

        let position = self.writer.seek(SeekFrom::End(0))?;
        let _ = self.writer.write_all(&buffer)?;
        let _ = self.writer.flush()?;

        let value_position = position + FILE_ENTRY_HEADER_SERIALIZED_SIZE as u64 + key.len() as u64;

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

        let buffer = Self::build_file_entry_buffer(key, &Vec::new(), timestamp);

        let _ = self.writer.seek(SeekFrom::End(0))?;
        let _ = self.writer.write_all(&buffer)?;
        let _ = self.writer.flush()?;

        let _ = self.keydir.remove(key);

        Ok(())
    }
}

impl RustcaskDatabase {
    fn write_header(writer: &mut impl Write) -> Result<(), Error> {
        writer.write_all(FILE_HEADER_IDENTIFIER_BYTES)?;
        writer.write_all(FILE_HEADER_VERSION_BYTES)?;
        writer.write_all(FILE_HEADER_RESERVED_BYTES)?;
        writer.flush()?;

        Ok(())
    }

    // Parses file header, leaves r at the next byte past the file header bytes
    fn parse_file_header<R: Read + Seek>(r: &mut R) -> Result<FileHeader, Error> {
        let mut buf = [0u8; FILE_HEADER_SIZE];
        let _ = r.read_exact(&mut buf).map_err(|e| {
            if e.kind() == ErrorKind::UnexpectedEof {
                Error::InvalidHeader
            } else {
                Error::Io(e)
            }
        })?;

        let identifier =
            &buf[FILE_HEADER_IDENTIFIER_BYTES_OFFSET..FILE_HEADER_VERSION_BYTES_OFFSET];
        let version = &buf[FILE_HEADER_VERSION_BYTES_OFFSET..FILE_HEADER_RESERVED_BYTES_OFFSET];
        let reserved = &buf[FILE_HEADER_RESERVED_BYTES_OFFSET..FILE_HEADER_SIZE];

        if identifier == FILE_HEADER_IDENTIFIER_BYTES
            && version == FILE_HEADER_VERSION_BYTES
            && reserved == FILE_HEADER_RESERVED_BYTES
        {
            Ok(FileHeader {
                version: FileHeaderVersion(u16::from_le_bytes(version.try_into().unwrap())),
                reserved: FileHeaderReserved(reserved.try_into().unwrap()),
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

        let file_entry_header = FileEntryHeader {
            crc: Crc(crc),
            timestamp,
            key_size: KeySize(key.len() as u32),
            value_size: ValueSize(value.len() as u32),
            flags: Flags(if value.is_empty() { 1u8 } else { 0u8 }),
        };

        let mut buffer = file_entry_header.serialize();
        buffer.extend_from_slice(key);
        buffer.extend_from_slice(value);

        buffer
    }

    fn rebuild_keydir<R: Read + Seek>(r: &mut R) -> Result<HashMap<Vec<u8>, KeyDirEntry>, Error> {
        let _file_header = Self::parse_file_header(r)?;

        let mut keydir = HashMap::new();

        let mut value_offset = FILE_HEADER_SIZE as u64;

        loop {
            let mut first_byte = [0u8; 1];

            let byte_count = r.read(&mut first_byte)?; // TODO: Need to retry on ErrorKind::Interrupted?

            if byte_count == 0 {
                break; // clean EOF, no more entries
            }

            let mut rest = [0u8; FILE_ENTRY_HEADER_SERIALIZED_SIZE - 1];
            let _ = r.read_exact(&mut rest)?; // UnexpectedEof here means corrupt/truncated

            let header = [&first_byte[..], &rest[..]].concat();

            // Parse entry header
            let entry = FileEntryHeader::deserialize(header.as_slice().try_into()?)?;

            // Attempt to read key bytes
            let mut key_buffer = vec![0u8; entry.key_size.0 as usize];
            let _ = r.read_exact(&mut key_buffer)?;

            value_offset += FILE_ENTRY_HEADER_SERIALIZED_SIZE as u64 + entry.key_size.0 as u64;

            if entry.flags.0 == 0 {
                // TODO: Create enum constants for flag values like insert, delete, etc.
                // Attempt to read value bytes
                let mut value_buffer = vec![0u8; entry.value_size.0 as usize];
                let _ = r.read_exact(&mut value_buffer)?;

                // Validate key bytes and value bytes are consistent with crc, return Error if not
                let mut hasher = crc32fast::Hasher::new();
                hasher.update(&key_buffer);
                hasher.update(&value_buffer);

                if hasher.finalize() != entry.crc.0 {
                    return Err(Error::InvalidEntry); // TODO: Add file info to error to aid troubleshooting
                }

                // Construct KeyDirEntry with value offset
                let keydir_entry = KeyDirEntry {
                    file_id: FileId(ACTIVE_FILE_NAME_ID),
                    timestamp: entry.timestamp,
                    value_position: ValuePosition(value_offset as u64),
                    value_size: entry.value_size,
                };

                // Add to keydir
                let _ = keydir.insert(key_buffer, keydir_entry);
            } else {
                // Remove from keydir
                let _ = keydir.remove(&key_buffer);
            }

            // Increment value offset to next entry
            value_offset += entry.value_size.0 as u64;
        }

        Ok(keydir)
    }

    fn build_database(active_file_path: PathBuf) -> Result<Self, Error> {
        let exists = active_file_path.exists();

        let mut read_file = OpenOptions::new()
            .create(!active_file_path.exists())
            .read(true)
            .truncate(false)
            .append(true)
            .open(&active_file_path)?;

        let mut write_file = OpenOptions::new()
            .create(!active_file_path.exists())
            .read(true)
            .truncate(false)
            .append(true)
            .open(&active_file_path)?;

        let keydir;

        if exists {
            keydir = Self::rebuild_keydir(&mut read_file)?;
        } else {
            let _ = Self::write_header(&mut write_file)?;
            keydir = HashMap::new();
        }

        let reader = BufReader::new(read_file);

        let writer = BufWriter::new(write_file);

        Ok(Self {
            reader,
            writer,
            keydir,
        })
    }

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

        Self::build_database(active_file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;

    #[test]
    fn test_write_header() {
        let mut buf = Vec::new();

        let result = RustcaskDatabase::write_header(&mut buf);

        assert!(result.is_ok());

        let mut target_buf = Vec::new();
        target_buf.extend_from_slice(FILE_HEADER_IDENTIFIER_BYTES);
        target_buf.extend_from_slice(FILE_HEADER_VERSION_BYTES);
        target_buf.extend_from_slice(FILE_HEADER_RESERVED_BYTES);

        assert_eq!(buf, target_buf);
    }

    #[test]
    fn test_parse_header_good_header() {
        let mut data = Vec::new();
        data.extend_from_slice(FILE_HEADER_IDENTIFIER_BYTES);
        data.extend_from_slice(FILE_HEADER_VERSION_BYTES);
        data.extend_from_slice(FILE_HEADER_RESERVED_BYTES);
        let mut cursor = Cursor::new(data);
        let result = RustcaskDatabase::parse_file_header(&mut cursor);

        let valid_header = FileHeader {
            version: FileHeaderVersion(u16::from_le_bytes(
                FILE_HEADER_VERSION_BYTES.try_into().unwrap(),
            )),
            reserved: FileHeaderReserved(FILE_HEADER_RESERVED_BYTES.try_into().unwrap()),
        };

        assert!(matches!(result, Ok(valid_header)));
    }

    #[test]
    fn test_parse_header_too_short() {
        let data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
        let mut cursor = Cursor::new(data);
        let result = RustcaskDatabase::parse_file_header(&mut cursor);

        assert!(matches!(result, Err(Error::InvalidHeader)));
    }

    #[test]
    fn test_parse_header_wrong_sequence() {
        let data = vec![0x00; 42];
        let mut cursor = Cursor::new(data);
        let result = RustcaskDatabase::parse_file_header(&mut cursor);

        assert!(matches!(result, Err(Error::InvalidHeader)));
    }

    #[test]
    fn test_build_file_entry_buffer_put() {
        let key = "key12345";
        let key_bytes = key.as_bytes();
        let value = "value67890";
        let value_bytes = value.as_bytes();
        let timestamp = Timestamp(123456789u64);

        let buffer = RustcaskDatabase::build_file_entry_buffer(key_bytes, value_bytes, timestamp);

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(key_bytes);
        hasher.update(value_bytes);
        let crc = hasher.finalize();

        let key_size = key.len() as u32;

        let value_size = value.len() as u32;

        let flags = 0u8;

        let mut target_buffer =
            Vec::with_capacity(FILE_ENTRY_HEADER_SERIALIZED_SIZE + key.len() + value.len());

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

        let buffer = RustcaskDatabase::build_file_entry_buffer(key_bytes, &value_bytes, timestamp);

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(key_bytes);
        hasher.update(&value_bytes);
        let crc = hasher.finalize();

        let key_size = key.len() as u32;

        let value_size = 0u32;

        let flags = 1u8;

        let mut target_buffer =
            Vec::with_capacity(std::mem::size_of::<FileEntryHeader>() + key.len());

        target_buffer.extend_from_slice(&crc.to_le_bytes());
        target_buffer.extend_from_slice(&timestamp.0.to_le_bytes());
        target_buffer.extend_from_slice(&key_size.to_le_bytes());
        target_buffer.extend_from_slice(&value_size.to_le_bytes());
        target_buffer.extend_from_slice(&flags.to_le_bytes());
        target_buffer.extend_from_slice(key_bytes);
        target_buffer.extend_from_slice(&value_bytes);

        assert_eq!(buffer, target_buffer);
    }

    #[test]
    fn test_rebuild_keydir() {
        let data: &[u8] = include_bytes!("../tests/fixtures/sample.rustcask");
        let mut cursor = Cursor::new(data);
        let keydir = RustcaskDatabase::rebuild_keydir(&mut cursor).unwrap();

        assert_eq!(keydir.len(), 2);

        assert!(keydir.get(b"foo".as_slice()).is_none());

        let abc = keydir.get(b"abc".as_slice()).unwrap();
        assert_eq!(abc.file_id, FileId(0));
        assert_eq!(abc.value_size, ValueSize(3));
        assert_eq!(abc.timestamp, Timestamp(1776294722721));
        assert_eq!(abc.value_position, ValuePosition(145));

        let ghi = keydir.get(b"ghi".as_slice()).unwrap();
        assert_eq!(ghi.file_id, FileId(0));
        assert_eq!(ghi.value_size, ValueSize(3));
        assert_eq!(ghi.timestamp, Timestamp(1776294290866));
        assert_eq!(ghi.value_position, ValuePosition(94));
    }
}
