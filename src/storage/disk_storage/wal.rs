use std::fs::{File, OpenOptions};
use std::io::{Write, Read, Seek};
use std::path::PathBuf;
use crate::storage::disk_storage::file_manager::FileManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileId {
    Node = 0,
    Structure = 1,
    Reverse = 2,
    Data = 3,
}

impl FileId {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(FileId::Node),
            1 => Some(FileId::Structure),
            2 => Some(FileId::Reverse),
            3 => Some(FileId::Data),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum WalRecord {
    Write { file_id: FileId, offset: u64, bytes: Vec<u8> },
    Zero { file_id: FileId, offset: u64, end: u64 },
    CopyWithin { file_id: FileId, src_start: u64, src_end: u64, dest_start: u64 },
    IncreaseFileSize { file_id: FileId },
}

impl WalRecord {
    pub fn file_id(&self) -> FileId {
        match self {
            WalRecord::Write { file_id, .. } => *file_id,
            WalRecord::Zero { file_id, .. } => *file_id,
            WalRecord::CopyWithin { file_id, .. } => *file_id,
            WalRecord::IncreaseFileSize { file_id } => *file_id,
        }
    }
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

    fn calculate_checksum(payload: &[u8]) -> u32 {
        let mut sum = 0u32;
        for &b in payload {
            sum = sum.wrapping_add(b as u32);
        }
        sum
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"WALT");
        let mut payload = Vec::new();
        payload.extend_from_slice(&(self.records.len() as u32).to_le_bytes());
        for record in &self.records {
            match record {
                WalRecord::Write { file_id, offset, bytes } => {
                    payload.push(0);
                    payload.push(*file_id as u8);
                    payload.extend_from_slice(&offset.to_le_bytes());
                    payload.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    payload.extend_from_slice(bytes);
                }
                WalRecord::Zero { file_id, offset, end } => {
                    payload.push(1);
                    payload.push(*file_id as u8);
                    payload.extend_from_slice(&offset.to_le_bytes());
                    payload.extend_from_slice(&end.to_le_bytes());
                }
                WalRecord::CopyWithin { file_id, src_start, src_end, dest_start } => {
                    payload.push(2);
                    payload.push(*file_id as u8);
                    payload.extend_from_slice(&src_start.to_le_bytes());
                    payload.extend_from_slice(&src_end.to_le_bytes());
                    payload.extend_from_slice(&dest_start.to_le_bytes());
                }
                WalRecord::IncreaseFileSize { file_id } => {
                    payload.push(3);
                    payload.push(*file_id as u8);
                }
            }
        }
        let payload_len = payload.len() as u32;
        buf.extend_from_slice(&payload_len.to_le_bytes());
        let checksum = Self::calculate_checksum(&payload);
        buf.extend_from_slice(&checksum.to_le_bytes());
        buf.extend_from_slice(&payload);
        buf
    }

    pub fn deserialize_all(file: &mut File) -> Result<Vec<WalTransaction>, std::io::Error> {
        file.rewind()?;
        let mut transactions = Vec::new();
        loop {
            let mut magic = [0u8; 4];
            if file.read_exact(&mut magic).is_err() { break; }
            if &magic != b"WALT" { break; }
            let mut len_buf = [0u8; 4];
            if file.read_exact(&mut len_buf).is_err() { break; }
            let payload_len = u32::from_le_bytes(len_buf);
            let mut checksum_buf = [0u8; 4];
            if file.read_exact(&mut checksum_buf).is_err() { break; }
            let expected_checksum = u32::from_le_bytes(checksum_buf);
            let mut payload = vec![0u8; payload_len as usize];
            if file.read_exact(&mut payload).is_err() { break; }
            if Self::calculate_checksum(&payload) != expected_checksum { break; }

            let mut cursor = 0;
            if cursor + 4 > payload.len() { break; }
            let mut num_records_buf = [0u8; 4];
            num_records_buf.copy_from_slice(&payload[cursor..cursor+4]);
            cursor += 4;
            let num_records = u32::from_le_bytes(num_records_buf);

            let mut records = Vec::new();
            let mut valid = true;
            for _ in 0..num_records {
                if cursor + 2 > payload.len() { valid = false; break; }
                let rec_type = payload[cursor];
                cursor += 1;
                let file_id_val = payload[cursor];
                cursor += 1;
                let file_id = match FileId::from_u8(file_id_val) {
                    Some(id) => id,
                    None => { valid = false; break; }
                };

                match rec_type {
                    0 => {
                        if cursor + 12 > payload.len() { valid = false; break; }
                        let mut off_buf = [0u8; 8];
                        off_buf.copy_from_slice(&payload[cursor..cursor+8]);
                        cursor += 8;
                        let offset = u64::from_le_bytes(off_buf);
                        let mut len_buf = [0u8; 4];
                        len_buf.copy_from_slice(&payload[cursor..cursor+4]);
                        cursor += 4;
                        let bytes_len = u32::from_le_bytes(len_buf) as usize;
                        if cursor + bytes_len > payload.len() { valid = false; break; }
                        let bytes = payload[cursor..cursor+bytes_len].to_vec();
                        cursor += bytes_len;
                        records.push(WalRecord::Write { file_id, offset, bytes });
                    }
                    1 => {
                        if cursor + 16 > payload.len() { valid = false; break; }
                        let mut off_buf = [0u8; 8];
                        off_buf.copy_from_slice(&payload[cursor..cursor+8]);
                        cursor += 8;
                        let offset = u64::from_le_bytes(off_buf);
                        let mut end_buf = [0u8; 8];
                        end_buf.copy_from_slice(&payload[cursor..cursor+8]);
                        cursor += 8;
                        let end = u64::from_le_bytes(end_buf);
                        records.push(WalRecord::Zero { file_id, offset, end });
                    }
                    2 => {
                        if cursor + 24 > payload.len() { valid = false; break; }
                        let mut buf8 = [0u8; 8];
                        buf8.copy_from_slice(&payload[cursor..cursor+8]);
                        cursor += 8;
                        let src_start = u64::from_le_bytes(buf8);
                        buf8.copy_from_slice(&payload[cursor..cursor+8]);
                        cursor += 8;
                        let src_end = u64::from_le_bytes(buf8);
                        buf8.copy_from_slice(&payload[cursor..cursor+8]);
                        cursor += 8;
                        let dest_start = u64::from_le_bytes(buf8);
                        records.push(WalRecord::CopyWithin { file_id, src_start, src_end, dest_start });
                    }
                    3 => {
                        records.push(WalRecord::IncreaseFileSize { file_id });
                    }
                    _ => { valid = false; break; }
                }
            }
            if valid {
                transactions.push(WalTransaction { records });
            } else {
                break;
            }
        }
        Ok(transactions)
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
            .append(true)
            .open(path)?;
        Ok(Self { file })
    }

    pub fn commit(&mut self, tx: &WalTransaction) -> Result<(), std::io::Error> {
        let bytes = tx.serialize();
        self.file.write_all(&bytes)?;
        self.file.sync_all()?;
        Ok(())
    }

    pub fn replay(
        &mut self,
        file_node: &mut FileManager,
        file_structure: &mut FileManager,
        file_reverse: &mut FileManager,
        file_data: &mut FileManager,
    ) -> Result<(), std::io::Error> {
        let transactions = WalTransaction::deserialize_all(&mut self.file)?;
        for tx in transactions {
            for record in tx.records {
                let fm = match record.file_id() {
                    FileId::Node => &mut *file_node,
                    FileId::Structure => &mut *file_structure,
                    FileId::Reverse => &mut *file_reverse,
                    FileId::Data => &mut *file_data,
                };
                match record {
                    WalRecord::Write { offset, bytes, .. } => {
                        fm.writing_bytes_to_mmap(offset, offset + bytes.len() as u64, &bytes);
                    }
                    WalRecord::Zero { offset, end, .. } => {
                        fm.zeroing_mmap(offset, end);
                    }
                    WalRecord::CopyWithin { src_start, src_end, dest_start, .. } => {
                        fm.copy_within(src_start, src_end, dest_start);
                    }
                    WalRecord::IncreaseFileSize { .. } => {
                        fm.increase_file_size()?;
                    }
                }
            }
        }
        
        file_node.flush()?;
        file_structure.flush()?;
        file_reverse.flush()?;
        file_data.flush()?;

        self.file.set_len(0)?;
        Ok(())
    }
}
