# rustcask

An educational Rust implementation of the Bitcask key/value database based on the [design paper from Basho](https://riak.com/assets/bitcask-intro.pdf).

This project is meant for educational use only.  It is not intended for production use.

## License

This project is under the MIT license.

## Features To Be Implemented

| Feature | Implemented? |
| --- | --- |
| REPL for testing | Yes |
| Append Only Log Structured Storage | Yes |
| Log Rotation | No |
| CRC Validation | No |
| Startup KeyDir rebuilding | No |
| Log Compaction | No |
| Hint files | No |
| Concurrent access | No |
| Range Queries over keys | No |
| TCP connectivity (client/server mode) | No |
| REPL improvement with rustyline crate | No |

## Overall Structure

At a high level, the database consists of an in-memory hashmap containing keys that are raw byte sequences and values that contain pointers to the data stored on disk.

Note that at runtime the entire keyspace is maintained in memory so enough memory must be present to store all the keys the database will handle.

## Disk Data File Format

The data on disk is organized into files with a capped size that is configurable using the environment variables listed below.  Files are rotated as they reach this size.

The previously stored and compacted files are named `<timestamp>`, where `<timestamp>` is the string representation of seconds since epoch.  The file currently being written is named `0`.

## Binary Entry File Format

Each file begins with a 16 byte binary header.  This contains the following fields:

| Field | Size | Description |
| --- | --- | --- |
| Name | 8 bytes | "rustcask" ASCII string |
| Version | 2 bytes | u16 file version, little endian |
| Reserved | 6 bytes | Reserved for future use |

For instance, for version 1:

```
72 75 73 74 63 61 73 6B 01 00 00 00 00 00 00 00
```

### Entry Format

```
| crc (4 bytes) | timestamp in milliseconds since epoch (8 bytes) | key_size (4 bytes) | value_size (4 bytes) | flags (1 byte) | key (variable) | value (variable) |
```

Maximum allowed key and value sizes are therefore 4GiB.  All numeric fields are little endian.

Regular entries are represented with a `flags` value of zero (`00`).  Tombstone entries are represented with a `flags` value of 1 (`01`) with a `value_size` of zero and no bytes stored for the `value`.  Other `flags` values are reserved for future use.

## Supported Operations

### Compact

Run the compaction process on the files on disk.  As the underlying storage uses append-only files, disk usage grows over time as values are overwritten.  The compaction process removes stale entries, keeping only the most recent value for each key.

For keys that are deleted, tombstone entries are written in the append-only files.  If the most recent entry for a key is a tombstone entry, that key is deleted.

### Get

Provided a raw sequence of bytes, return the corresponding value bytes.

### Put

Provided a raw sequence of bytes, either store a new value sequence of bytes for that key or overwrite any existing value for that key.  Rustcask does not support storing keys with empty values.

### Delete

Provided a raw sequence of bytes, remove the corresponding value sequence of bytes for that key from the in-memory hashmap and from the files on disk.

## Crates

The repo consists of one binary crate and one library crate.

The library crate contains the core logic for the database and its operations.

The binary crate consists of a single command line interface (basically a REPL) to manually interact with the database.

When working with the REPL in the binary crate, all keys and values are stored as strings, and the REPL does not support the use of arbitrary byte sequences, though the underlying library crate does support this.

## Binary Crate Operations

To run the binary crate, use `cargo run`.  The test suite can be run with `cargo test`.

### Commands

`compact` - Runs the compaction operation.  Returns nothing if it succeeded or an error message if it did not.

`get <key>` - Returns the value currently stored for the string `<key>` provided or nothing if no value is stored for that key.  Otherwise an error message is returned if the operation was not successful.

`put <key> <value>` - Store the provided string `<value>` as the value for the specified key string `<key>`.  Returns nothing if the operation was successful or an error message if the operation failed.

`delete <key>` - Remove the stored value for the corresponding key string `<key>`.  Returns nothing if the operation was successful or an error message if the operation failed.

### Example Command Line Session

```sh
> get foo
> put foo bar
> get foo
bar
> put foo baz
> get foo
baz
> put test 123
> delete foo
> get foo
> get test
123
> exit
```

### Environment Variable Configuration

The following environment variables provide configuration values for the database.  The environment variables listed in the `Required Environment Variables` section are required to create the database.  The binary crate will not start unless these variables are set.

#### Required Environment Variables

`RUSTCASK_FILE_PATH` - The absolute file path under which to store the append-only disk files.

#### Optional Environment Variables

`RUSTCASK_MAX_FILE_SIZE_MB` - The maximum file size cap, in MB.  Defaults to 4 MB if not present.

## Running the Application

First, configure a local `.env` file to contain the following environment variables:

```
RUSTCASK_FILE_PATH=<absolute path to file storage for database data>
RUSTCASK_MAX_FILE_SIZE_MB=<integer file size, in MB>
RUST_LOG=<log level>
```

Run the application with `cargo run --features mock` to use the mock in-memory database for testing purposes.

Run the application with `cargo run` to use the regular filesystem database.
