use std::{hash::Hash, marker::PhantomData, path::Path};

use ahash::AHashMap;

use crate::{core::db_error::DbError, dictionary::dictionary_strategy::DictionaryStrategy, storage::disk_storage::{file_manager::FileManager, from_disk_bytes::{AsDiskBytes, FromDiskBytes}, wal::{FileId, WalManager, WalRecord, WalTransaction}}};

use crate::dictionary::node_id::NodeId;

/// On-disk dictionary implementation mapping node data to internal IDs.
///
/// Keeps an in-memory map of keys to node IDs, but persists data and ID layouts using WAL logs and data files.
#[derive(Debug)]
pub struct DiskDictionary<K>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes
{
    file_node_id: FileManager,
    file_data: FileManager,
    wal_manager: WalManager,
    node_type: PhantomData<K>,
    hashing: AHashMap<K, u64>,
    data_eof: u64,
}

impl<K> DiskDictionary<K>
where
    K: Eq + Clone + Hash + AsDiskBytes + FromDiskBytes
{
    /// Creates a new on-disk dictionary.
    ///
    /// Opens or creates the `node_id.bin` and `data.bin` files at the provided path to persist the mappings.
    ///
    /// # Arguments
    /// * `path` - The directory path where the dictionary files should reside.
    /// * `wal_manager` - A reference to the write-ahead log manager used for durable transactions.
    ///
    /// # Panics
    /// Panics if opening or creating the `node_id.bin` or `data.bin` files fails.
    pub fn new<W, P:AsRef<Path>>(path: P, wal_manager: &WalManager) -> Self
    where
        W: Clone + PartialEq + FromDiskBytes + AsDiskBytes
    {
        let dir = path.as_ref();
        
        let node_id_path = dir.join("node_id.bin");
        let node_id_value_path = dir.join("node_id_data.bin");

        let (file_node_id, _) = FileManager::new(node_id_path)
            .expect("Failed to open the node_id");

        let (file_node_value_id, _) = FileManager::new(node_id_value_path)
            .expect("Failed to open the node_id_value");

        let wal_manager = wal_manager.clone();

        let mut dict = Self{
            file_data: file_node_value_id,
            file_node_id,
            wal_manager,
            node_type: PhantomData,
            hashing: AHashMap::new(),
            data_eof: 0,
        };
        
        dict.populating().expect("Failed to populate DiskDictionary");
        dict
    }

    /// Calculates the file byte offset for a given node ID.
    ///
    /// # Arguments
    /// * `node_id` - The internal node ID to calculate the storage offset for.
    fn calculate_offset_from_id(&self, node_id: u64) -> u64{
        node_id * size_of::<NodeId>() as u64
    }

    /// Calculates the internal node ID from a file byte offset.
    ///
    /// # Arguments
    /// * `offset` - The file byte offset to derive the node ID from.
    fn calculate_id_from_offset(&self, offset: u64) -> u64{
        offset / size_of::<NodeId>() as u64
    }

    /// Populates the in-memory hash map by reading node IDs and data from the disk files.
    ///
    /// # Errors
    /// Returns a [`DbError`] if an underlying storage or file system read operation fails.
    fn populating(&mut self) -> Result<(), DbError>{
        let mut offset = 0;
        let mut max_data_eof = 0;
        while offset < self.file_node_id.file_len()?{
            let node_id: NodeId = self.file_node_id.reading_bytes(offset, offset + size_of::<NodeId>() as u64, |b: &[u8]| NodeId::from_bytes(b))?;
                
            if node_id.data_len == 0{
                break;
            }

            if node_id.data_len != u64::MAX {
                let key = self.file_data.reading_bytes(node_id.data_offset, node_id.data_offset + node_id.data_len, |b: &[u8]| K::from_bytes(b))?;

                self.hashing.insert(key, self.calculate_id_from_offset(offset));
                
                let current_eof = node_id.data_offset + node_id.data_len;
                if current_eof > max_data_eof {
                    max_data_eof = current_eof;
                }
            }

            offset += size_of::<NodeId>() as u64;
        }
        self.data_eof = max_data_eof;
        Ok(())
    }

    fn resize_file_node(&mut self, offset: u64, tx: &mut WalTransaction) -> Result<(), DbError>{
        while offset > self.file_node_id.file_len()? {
            let next_size = self.file_node_id.check_next_size(self.file_node_id.file_len()?)?;
            tx.increase_file_size(FileId::NodeId, next_size);
            self.file_node_id.increase_file_size()?;
        }
        Ok(())
    }

    fn resize_file_data(&mut self, offset: u64, tx: &mut WalTransaction) -> Result<(), DbError>{
        while offset > self.file_data.file_len()? {
            let next_size = self.file_data.check_next_size(self.file_data.file_len()?)?;
            tx.increase_file_size(FileId::NodeValue, next_size);
            self.file_data.increase_file_size()?;
        }
        Ok(())
    }


    pub fn apply_wal_transaction(&mut self, tx: &WalTransaction) {
        for record in &tx.records {
            match record {
                WalRecord::Write { file_id, offset, bytes } => {
                    let fm = match file_id {
                        FileId::NodeId => &mut self.file_node_id,
                        FileId::NodeValue => &mut self.file_data, 
                        _ => continue,
                    };
                    fm.writing_bytes_to_mmap(*offset, *offset + bytes.len() as u64, bytes);
                }
                WalRecord::Zero { file_id, offset, end } => {
                    let fm = match file_id {
                        FileId::NodeId => &mut self.file_node_id,
                        FileId::NodeValue => &mut self.file_data,
                        _ => continue,
                    };
                    fm.zeroing_mmap(*offset, *end);
                }
                WalRecord::CopyWithin { file_id, src_start, src_end, dest_start } => {
                    let fm = match file_id {
                        FileId::NodeId => &mut self.file_node_id,
                        FileId::NodeValue => &mut self.file_data,
                        _ => continue,
                    };
                    fm.copy_within(*src_start, *src_end, *dest_start);
                }
                WalRecord::IncreaseFileSize { file_id: _, size: _} => {                    // Already applied directly to satisfy length checks during buffer phase.
                }
            }
        }
    }
}

impl<K> DictionaryStrategy<K> for DiskDictionary<K>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
{
    type Error = DbError;
    /// Checks if a node exists in the dictionary.
    ///
    /// # Arguments
    /// * `key` - The node data to check for existence in the dictionary.
    fn contains_key(&self, key: &K) -> bool {
        self.hashing.contains_key(key)
    }

    /// Inserts node data into the dictionary, persisting it to disk. Returns `Ok(None)` if the node didn't exist. If the key already exists, it returns the old node ID.
    ///
    /// # Arguments
    /// * `key` - The node data to insert.
    /// * `id` - The internal ID to associate with the node data.
    ///
    /// # Errors
    /// Returns a [`DbError`] if an underlying storage operation fails during insertion or write-ahead log commit.
    fn insert(&mut self, key: K, id: u64) -> Result<Option<u64>, DbError> {

        let offset = self.calculate_offset_from_id(id);
        let data_offset = self.data_eof;

        let key_bytes = key.as_disk_bytes();

        let data_len = key_bytes.len() as u64;
        let node_id = NodeId::new(data_len, data_offset);

        let bytes = node_id.convert_to_bytes();

        let mut tx = WalTransaction::new();

        self.resize_file_node(offset + bytes.len() as u64, &mut tx)?;
        self.resize_file_data(data_offset + key_bytes.len() as u64, &mut tx)?;
        // 2. Resize Data file if necessary
        tx.write_bytes(FileId::NodeId, offset, bytes);

        tx.write_bytes(FileId::NodeValue, data_offset, &key_bytes);

        self.wal_manager.commit(&tx).map_err(|e| {
            DbError::Io(e)
        })?;

        self.file_node_id.writing_bytes_to_mmap(offset, offset + bytes.len() as u64, bytes);
        self.file_data.writing_bytes_to_mmap(data_offset, data_offset + key_bytes.len() as u64, &key_bytes);
        
        self.data_eof += key_bytes.len() as u64;

        Ok(self.hashing.insert(key, id))
    }

    /// Retrieves the internal ID of a node for use inside the graph engine.
    ///
    /// # Arguments
    /// * `key` - The node data to retrieve the ID for.
    fn get(&self, key: &K) -> Option<u64> {
        self.hashing.get(key).copied()
    }

    /// Removes the key and returns its associated internal ID, writing a tombstone ID to disk. Returns `Ok(None)` if the key didn't exist.
    ///
    /// # Arguments
    /// * `key` - The node data to remove from the dictionary.
    ///
    /// # Errors
    /// Returns a [`DbError`] if an underlying storage operation fails.
    fn remove(&mut self, key: &K) -> Result<Option<u64>, DbError> {
        let id = match self.hashing.get(key){
            Some(t) => *t,
            None => return Ok(None),
        };

        let offset = self.calculate_offset_from_id(id);
        let node_id = NodeId::new(u64::MAX, u64::MAX);
        let node_id_bytes = node_id.convert_to_bytes();

        let mut tx = WalTransaction::new();

        tx.write_bytes(FileId::NodeId, offset, node_id_bytes);
        self.wal_manager.commit(&tx)?;
        
        self.file_node_id.writing_bytes_to_mmap(offset, offset + node_id_bytes.len() as u64, node_id_bytes);

        Ok(self.hashing.remove(key))
    }

    fn reverse_node_data(&self, id: u64) -> Option<K> {
        let offset = self.calculate_offset_from_id(id);

        let node_id: NodeId = self.file_node_id.reading_bytes(offset, offset + std::mem::size_of::<NodeId>() as u64, |b: &[u8]| NodeId::from_bytes(b)).unwrap();

        if node_id.data_len == u64::MAX{
            None
        }else{
            let start = node_id.data_offset;
            let end = start + node_id.data_len;
            let key = self.file_data.reading_bytes(start, end, |b: &[u8]| K::from_bytes(b)).unwrap();

            Some(key)
        }
    }

    fn bulk_insert(&mut self, nodes: &[(K, u64)]) -> Result<(), DbError> {
        let mut tx = WalTransaction::new();
        for (node_data, id) in nodes{
            let offset = self.calculate_offset_from_id(*id);
            let data_bytes = node_data.as_disk_bytes();
            
            let data_offset = self.data_eof;
            let node_id = NodeId::new(data_bytes.len() as u64, data_offset);

            self.resize_file_node(offset + size_of::<NodeId>() as u64, &mut tx)?;
            self.resize_file_data(data_offset + data_bytes.len() as u64, &mut tx)?;

            tx.write_bytes(FileId::NodeId, offset, &node_id.as_disk_bytes());
            tx.write_bytes(FileId::NodeValue, data_offset, &data_bytes);

            self.data_eof += data_bytes.len() as u64;

            self.hashing.insert(node_data.clone(), *id);
        }

        self.wal_manager.commit(&tx)?;

        self.apply_wal_transaction(&tx);
        Ok(())
    }
}
