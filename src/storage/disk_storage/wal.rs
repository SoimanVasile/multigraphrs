use std::fs::{File, OpenOptions};
use std::io::{Write, Read, Seek};
use std::path::PathBuf;
use crate::storage::disk_storage::file_manager::FileManager;
use crate::core::db_error::DbError;

/// Identifies which backing file a WAL record targets.
///
/// Each variant maps to one of the four memory-mapped data files
/// that make up the on-disk graph storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileId {
    Node = 0,
    Structure = 1,
    Reverse = 2,
    Data = 3,
}

impl FileId {
    /// Converts a `u8` value into an `Option<FileId>`.
    ///
    /// # Errors
    /// Returns `None` if the provided value does not correspond to a valid `FileId`.
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

/// A single operation to be replayed against a data file.
///
/// Each record captures exactly one mutation so the WAL can
/// faithfully reconstruct the state of the data files after a crash.
#[derive(Debug, Clone)]
pub enum WalRecord {
    /// Overwrite bytes at the given offset.
    Write { file_id: FileId, offset: u64, bytes: Vec<u8> },
    /// Zero out the byte range `[offset, end)`.
    Zero { file_id: FileId, offset: u64, end: u64 },
    /// Copy the byte range `[src_start, src_end)` to `dest_start`.
    CopyWithin { file_id: FileId, src_start: u64, src_end: u64, dest_start: u64 },
    /// Double (or grow by 4 GB) the file, recording the target `size`
    /// so replay can skip the resize if the file is already large enough.
    IncreaseFileSize { file_id: FileId, size: u64 },
}

impl WalRecord {
    /// Returns the [`FileId`] this record targets.
    pub fn file_id(&self) -> FileId {
        match self {
            WalRecord::Write { file_id, .. } => *file_id,
            WalRecord::Zero { file_id, .. } => *file_id,
            WalRecord::CopyWithin { file_id, .. } => *file_id,
            WalRecord::IncreaseFileSize { file_id, .. } => *file_id,
        }
    }
}

/// A group of [`WalRecord`]s that are committed atomically.
///
/// The transaction is serialized with a checksum so that partially-written
/// transactions (due to a crash) are detected and skipped during replay.
#[derive(Debug)]
pub struct WalTransaction {
    pub records: Vec<WalRecord>,
}

impl Default for WalTransaction {
    fn default() -> Self {
        Self::new()
    }
}

impl WalTransaction {
    /// Creates a new, empty transaction.
    pub fn new() -> Self {
        Self { records: Vec::new() }
    }

    /// Appends a [`WalRecord::Write`] that overwrites `bytes` at `offset`.
    pub fn write_bytes(&mut self, file_id: FileId, offset: u64, bytes: &[u8]) {
        self.records.push(WalRecord::Write {
            file_id,
            offset,
            bytes: bytes.to_vec(),
        });
    }

    /// Appends a [`WalRecord::Zero`] that zeroes the range `[offset, end)`.
    pub fn zero_mmap(&mut self, file_id: FileId, offset: u64, end: u64) {
        self.records.push(WalRecord::Zero { file_id, offset, end });
    }

    /// Appends a [`WalRecord::CopyWithin`] that copies `[src_start, src_end)` to `dest_start`.
    pub fn copy_within(&mut self, file_id: FileId, src_start: u64, src_end: u64, dest_start: u64) {
        self.records.push(WalRecord::CopyWithin { file_id, src_start, src_end, dest_start });
    }

    /// Appends a [`WalRecord::IncreaseFileSize`] that records the new target `size`.
    pub fn increase_file_size(&mut self, file_id: FileId, size: u64) {
        self.records.push(WalRecord::IncreaseFileSize { file_id, size });
    }

    /// Computes a simple additive checksum over `payload`.
    ///
    /// This is intentionally lightweight — it catches torn writes and
    /// accidental corruption but is not cryptographic.
    fn calculate_checksum(payload: &[u8]) -> u32 {
        let mut sum = 0u32;
        for &b in payload {
            sum = sum.wrapping_add(b as u32);
        }
        sum
    }

    /// Serializes the transaction into a self-describing byte vector.
    ///
    /// Wire format:
    /// ```text
    /// [WALT magic (4 bytes)]
    /// [payload length (4 bytes LE)]
    /// [checksum (4 bytes LE)]
    /// [payload ...]
    ///   - record count (4 bytes LE)
    ///   - for each record: type (1 byte), file_id (1 byte), fields...
    /// ```
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
                WalRecord::IncreaseFileSize { file_id, size } => {
                    payload.push(3);
                    payload.push(*file_id as u8);
                    payload.extend_from_slice(&size.to_le_bytes());
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

    /// Deserializes every valid transaction from `file`, stopping at
    /// the first corrupted or incomplete record.
    ///
    /// This is used during crash recovery: any partially-written
    /// transaction at the tail of the file is silently discarded
    /// because its checksum will not match.
    pub fn deserialize_all(file: &mut File) -> Result<Vec<WalTransaction>, DbError> {
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
                        if cursor + 8 > payload.len() { valid = false; break; }
                        let mut buf8 = [0u8; 8];
                        buf8.copy_from_slice(&payload[cursor..cursor + 8]);
                        cursor += 8;
                        let size = u64::from_le_bytes(buf8);
                        records.push(WalRecord::IncreaseFileSize { file_id, size });
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

struct DBFiles<'a>{
    file_node: &'a mut FileManager,
    file_structure: &'a mut FileManager,
    file_reverse: &'a mut FileManager,
    file_data: &'a mut FileManager,
}

impl<'a> DBFiles<'a>{
    pub fn new(file_node: &'a mut FileManager, file_structure: &'a mut FileManager, file_reverse: &'a mut FileManager, file_data: &'a mut FileManager) -> Self{
        Self{file_node, file_structure, file_reverse, file_data}
    }
}

fn replay_file(path: &PathBuf, files: &mut DBFiles) -> Result<(), DbError> {
    if path.exists() {
        let mut file = OpenOptions::new().read(true).open(path)?;
        let transactions = WalTransaction::deserialize_all(&mut file)?;
        for tx in transactions {
            for record in tx.records {
                let fm = match record.file_id() {
                    FileId::Node => &mut files.file_node,
                    FileId::Structure => &mut files.file_structure,
                    FileId::Reverse => &mut files.file_reverse,
                    FileId::Data => &mut files.file_data,
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
                    WalRecord::IncreaseFileSize { size, .. } => {
                        if fm.file_len().unwrap() < size {
                            fm.increase_file_size()?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Message sent from [`WalManager::commit`] to the background thread.
///
/// Also used directly in tests to exercise concurrent multi-producer scenarios.
pub struct WalRequest {
    /// Pre-serialized transaction bytes ready to be appended to the WAL file.
    pub payload: Vec<u8>,
    /// One-shot channel the background thread uses to signal completion.
    /// The `bool` is `true` when the WAL file was rotated after this write,
    /// meaning the caller should flush all graph data files.
    pub response_tx: std::sync::mpsc::Sender<std::io::Result<bool>>,
}

/// Manages the write-ahead log with a background writer thread.
///
/// # Architecture
///
/// ```text
///  caller thread                background thread
///  ─────────────                ─────────────────
///  commit(tx)                         │
///    ├─ serialize ──► request_tx ──►   │
///    │                                ├─ write_all + sync_all
///    │                                ├─ rotate if full
///    │◄── response_rx ◄──────────────┘
///    └─ return Ok(rotated)
/// ```
///
/// The background thread batches multiple requests that arrive while it
/// is busy writing, so only one `sync_all` is issued per batch.
///
/// ## Rotation
///
/// When `wal.bin` exceeds `max_file_size` after a batch write:
/// 1. The current `wal.bin` is `sync_all`'d (data durable).
/// 2. `old_wal.bin` is deleted (its data was already flushed to graph
///    files by the caller during the *previous* rotation).
/// 3. `wal.bin` is renamed to `old_wal.bin`.
/// 4. A fresh `wal.bin` is created.
/// 5. `rotated = true` is returned to the **first** caller in the
///    batch so it flushes the four graph data files.
///
/// Subsequent callers in the same batch receive `rotated = false`
/// because a single flush is sufficient.
#[derive(Debug)]
pub struct WalManager {
    dir: PathBuf,
    /// The sender half of the channel to the background thread.
    /// Exposed as `pub` so tests can clone it for multi-producer scenarios.
    pub request_tx: Option<std::sync::mpsc::Sender<WalRequest>>,
}


impl WalManager {
    /// Creates a new [`WalManager`] rooted at `dir`.
    ///
    /// The directory is created if it does not exist. No background
    /// thread is started until [`start`](Self::start) is called.
    pub fn new(dir: PathBuf) -> Result<Self, std::io::Error> {
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir, request_tx: None })
    }

    /// Spawns the background writer thread.
    ///
    /// The thread will rotate the WAL file when it exceeds
    /// `max_file_size` bytes and signal the caller to flush.
    ///
    /// # Panics
    /// Panics if the initial `wal.bin` file cannot be opened.
    pub fn start(&mut self, max_file_size: u64) -> Result<(), std::io::Error> {
        let (request_tx, request_rx) = std::sync::mpsc::channel();
        self.request_tx = Some(request_tx);

        let dir = self.dir.clone();

        std::thread::spawn(move || {
            wal_background_thread(dir, max_file_size, request_rx);
        });
        Ok(())
    }

    /// Sends a serialized transaction to the background thread and
    /// blocks until the data is durable on disk.
    ///
    /// # Returns
    /// - `Ok(true)` — the WAL was rotated; the caller **must** flush
    ///   all four graph data files before issuing the next commit.
    /// - `Ok(false)` — normal commit, no flush needed.
    ///
    /// # Errors
    /// - [`std::io::ErrorKind::NotConnected`] if [`start`](Self::start)
    ///   was never called.
    /// - [`std::io::ErrorKind::BrokenPipe`] if the background thread
    ///   has panicked or been dropped.
    /// - Any I/O error propagated from the background thread's
    ///   `write_all` or `sync_all`.
    pub fn commit(&self, tx: &WalTransaction) -> Result<bool, std::io::Error> {
        let bytes = tx.serialize();

        let tx_sender = self.request_tx.as_ref().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotConnected, "WalManager thread not started")
        })?;

        let (response_tx, response_rx) = std::sync::mpsc::channel();
        let req = WalRequest {
            payload: bytes,
            response_tx,
        };

        if tx_sender.send(req).is_err() {
            return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "WAL thread died"));
        }

        response_rx.recv().unwrap_or_else(|_| {
            Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "WAL thread died before responding"))
        })
    }

    /// Replays all WAL transactions on startup to recover from a crash.
    ///
    /// Reads `old_wal.bin` first (if it exists), then `wal.bin`,
    /// applying every record to the appropriate [`FileManager`].
    /// After replay the data files are flushed and both WAL files
    /// are deleted so the database starts with a clean slate.
    ///
    /// This must be called **before** [`start`](Self::start).
    pub fn replay(
        &mut self,
        file_node: &mut FileManager,
        file_structure: &mut FileManager,
        file_reverse: &mut FileManager,
        file_data: &mut FileManager,
    ) -> Result<(), DbError> {
        let old_wal_path = self.dir.join("old_wal.bin");
        let wal_path = self.dir.join("wal.bin");

        let mut files = DBFiles::new(file_node, file_structure, file_reverse, file_data);
        replay_file(&old_wal_path, &mut files)?;
        replay_file(&wal_path, &mut files)?;

        file_node.flush()?;
        file_structure.flush()?;
        file_reverse.flush()?;
        file_data.flush()?;

        if old_wal_path.exists() {
            std::fs::remove_file(&old_wal_path)?;
        }
        if wal_path.exists() {
            std::fs::remove_file(&wal_path)?;
        }
        Ok(())
    }
}

/// The background writer loop.
///
/// Runs on a dedicated thread, receiving serialized WAL transactions
/// via `request_rx`. Each iteration:
///
/// 1. **Block** on the first incoming request.
/// 2. **Drain** any additional requests that arrived in the meantime
///    to form a batch.
/// 3. **Write** all payloads to `wal.bin`.
/// 4. **`sync_all`** so the data is durable.
/// 5. **Rotate** if the file exceeded `max_file_size`:
///    - drop the file handle
///    - delete `old_wal.bin` (safe — its data was flushed to the
///      graph files by the caller after the *previous* rotation)
///    - rename `wal.bin` → `old_wal.bin`
///    - open a fresh `wal.bin`
/// 6. **Respond** to every caller in the batch. Only the first caller
///    receives `rotated = true` so exactly one flush happens.
///
/// The loop exits when all senders are dropped (channel disconnected).
fn wal_background_thread(dir: PathBuf, max_file_size: u64, request_rx: std::sync::mpsc::Receiver<WalRequest>) {
    let wal_path = dir.join("wal.bin");
    let old_wal_path = dir.join("old_wal.bin");


    let mut current_file: Option<File>;
    let mut current_size: u64;

    match open_new_file(&wal_path) {
        Ok((f, size)) => {
            current_file = Some(f);
            current_size = size;
        }
        Err(e) => {
            panic!("WAL thread failed to open initial log file: {}", e);
        }
    }

    loop {
        let req = match request_rx.recv() {
            Ok(req) => req,
            Err(_) => break, // all senders dropped — shut down
        };

        let mut batch = vec![req];
        while let Ok(req) = request_rx.try_recv() {
            batch.push(req);
        }

        let mut file_opt = current_file.take();
        let mut io_error = None;
        let mut rotated = false;

        // ── Step 1: write the entire batch ──
        for req in &batch {
            if io_error.is_none()
                && let Some(f) = file_opt.as_mut() {
                    match f.write_all(&req.payload) {
                        Ok(_) => {
                            current_size += req.payload.len() as u64;
                        }
                        Err(e) => {
                            io_error = Some(e);
                        }
                    }
                }
        }

        // ── Step 2: sync — data is durable after this ──
        if io_error.is_none()
            && let Some(f) = file_opt.as_mut()
                && let Err(e) = f.sync_all() {
                    io_error = Some(e);
                }

        // ── Step 3: rotate if WAL exceeded the size limit ──
        if io_error.is_none() && current_size >= max_file_size {

            match rotate_wal(&mut file_opt, &old_wal_path, &wal_path){
                Ok(_) => {},
                Err(e) => io_error = Some(e),
            };
            if io_error.is_none() {
                match open_new_file(&wal_path) {
                    Ok((new_file, size)) => {
                        file_opt = Some(new_file);
                        current_size = size;
                        rotated = true;
                    }
                    Err(e) => {
                        io_error = Some(e)
                    }
                }
            }
        }

        current_file = file_opt;

        // ── Step 4: respond to every caller ──
        if let Some(e) = io_error {
            for req in batch {
                let _ = req.response_tx.send(Err(std::io::Error::new(e.kind(), e.to_string())));
            }
        } else {
            let mut first = true;
            for req in batch {
                let _ = req.response_tx.send(Ok(first && rotated));
                first = false;
            }
        }
    }
}

fn rotate_wal(file_opt: &mut Option<File>, old_wal_path: &PathBuf, wal_path: &PathBuf) -> Result<(), std::io::Error>{
    
    // Drop the file handle before renaming
    drop(file_opt.take());

    (|| -> std::io::Result<()> {
        if old_wal_path.exists() {
            std::fs::remove_file(&old_wal_path)?;
        }
        std::fs::rename(&wal_path, &old_wal_path)?;
        Ok(())
    })()?;
    
    Ok(())
}


fn open_new_file(wal_path: &PathBuf) -> std::io::Result<(File, u64)> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&wal_path)?;
    let metadata = file.metadata()?;
    Ok((file, metadata.len()))
}
