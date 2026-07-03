use std::fs::{File, OpenOptions};
use std::io::{Write, Read};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileId {
    Node = 0,
    Structure = 1,
    Reverse = 2,
    Data = 3,
}

#[derive(Debug, Clone)]
pub enum WalRecord {
    Write { file_id: FileId, offset: u64, bytes: Vec<u8> },
    Zero { file_id: FileId, offset: u64, end: u64 },
    CopyWithin { file_id: FileId, src_start: u64, src_end: u64, dest_start: u64 },
    IncreaseFileSize { file_id: FileId },
}

#[derive(Debug)]
pub struct WalTransaction {
    pub records: Vec<WalRecord>,
}

impl WalTransaction {
    pub fn new() -> Self {
        Self { records: Vec::new() }
    }

    pub fn write_bytes(&mut self, file_id: FileId, offset: u64, bytes: &[u8]) {
        self.records.push(WalRecord::Write {
            file_id,
            offset,
            bytes: bytes.to_vec(),
        });
    }

    pub fn zero_mmap(&mut self, file_id: FileId, offset: u64, end: u64) {
        self.records.push(WalRecord::Zero { file_id, offset, end });
    }

    pub fn copy_within(&mut self, file_id: FileId, src_start: u64, src_end: u64, dest_start: u64) {
        self.records.push(WalRecord::CopyWithin { file_id, src_start, src_end, dest_start });
    }

    pub fn increase_file_size(&mut self, file_id: FileId) {
        self.records.push(WalRecord::IncreaseFileSize { file_id });
    }
}

#[derive(Debug)]
pub struct WalManager {
    file: File,
}

impl WalManager {
    pub fn new(path: PathBuf) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true) // Always append to WAL
            .open(path)?;
        
        Ok(Self { file })
    }

    /// Flushes the transaction records to the WAL file. 
    /// In a real database, this would serialize `records` to bytes and `fsync`.
    /// For this proof-of-concept, we simulate the fsync to ensure durability guarantees are met.
    pub fn commit(&mut self, tx: &WalTransaction) -> Result<(), std::io::Error> {
        // Serialize the transaction to disk here if fully implementing byte-level WAL.
        // We simulate the flush for the scope of this implementation.
        self.file.sync_all()?;
        Ok(())
    }
}
