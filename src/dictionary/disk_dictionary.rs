use std::{hash::Hash, marker::PhantomData, path::Path};

use ahash::AHashMap;

use crate::{DirectionStrategy, DiskStorage, core::db_error::DbError, dictionary::dictionary_strategy::DictionaryStrategy, storage::disk_storage::{file_manager::FileManager, from_disk_bytes::{AsDiskBytes, FromDiskBytes}, wal::{FileId, WalManager, WalTransaction}}};

use crate::dictionary::node_id::NodeId;

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
}

impl<K> DiskDictionary<K>
where
    K: Eq + Clone + Hash + AsDiskBytes + FromDiskBytes
{
    pub fn new<W, P:AsRef<Path>>(path: P, wal_manager: &WalManager) -> Self
    where
        W: Clone + PartialEq + FromDiskBytes + AsDiskBytes
    {
        let dir = path.as_ref();
        
        let node_id_path = dir.join("node_id.bin");
        let node_id_value_path = dir.join("data.bin");

        let (file_node_id, _) = FileManager::new(node_id_path)
            .expect("Failed to open the node_id");

        let (file_node_value_id, _) = FileManager::new(node_id_value_path)
            .expect("Failed to open the node_id_value");

        let wal_manager = wal_manager.clone();

        Self{
            file_data: file_node_value_id,
            file_node_id,
            wal_manager,
            node_type: PhantomData,
            hashing: AHashMap::new()
        }
    }

    fn calculate_node_id_offset(&self, node_id: u64) -> u64{
        node_id * size_of::<NodeId>() as u64
    }

    fn calculate_id_from_offset(&self, offset: u64) -> u64{
        offset / size_of::<NodeId>() as u64
    }

    fn populating(&mut self) -> Result<(), DbError>{
        let mut offset = 0;
        while offset < self.file_node_id.file_len()?{
            let node_id_bytes = self.file_node_id.reading_bytes(0, 0 + size_of::<NodeId>() as u64);
            let node_id: NodeId = NodeId::from_bytes(node_id_bytes);
                
            if node_id.data_len == 0{
                break;
            }

            if node_id.data_len == u64::MAX{
                continue;
            }

            let bytes = self.file_data.reading_bytes(node_id.data_offset, node_id.data_offset + node_id.data_len);
            let key = K::from_bytes(bytes);

            self.hashing.insert(key, self.calculate_id_from_offset(node_id.data_offset));

            offset += size_of::<NodeId>() as u64;
        }
        Ok(())
    }
}

impl<K> DictionaryStrategy<K> for DiskDictionary<K>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
{
    fn contains_key(&self, key: &K) -> bool {
        self.hashing.contains_key(key)
    }

    fn insert(&mut self, key: K, id: u64) -> Result<(), DbError> {
        let offset = self.calculate_node_id_offset(id);
        let data_offset = self.file_data.file_len().unwrap();
        let data_len = size_of::<K>() as u64;
        let node_id = NodeId::new(data_len, data_offset);

        let bytes = node_id.convert_to_bytes();

        let mut tx = WalTransaction::new();

        tx.write_bytes(FileId::NodeId, offset, bytes);

        let key_bytes = key.as_disk_bytes();
        tx.write_bytes(FileId::Data, data_offset, &key_bytes);

        self.wal_manager.commit(&tx).map_err(|e| {
            DbError::Io(e)
        })?;

        self.hashing.insert(key, id);
        Ok(())
    }

    fn get(&self, key: &K) -> Option<u64> {
        self.hashing.get(key).copied()
    }

    fn remove(&mut self, key: &K) -> Result<Option<u64>, DbError> {
        let id = match self.hashing.get(key){
            Some(t) => *t,
            None => return Ok(None),
        };

        let offset = self.calculate_node_id_offset(id);
        let node_id = NodeId::new(u64::MAX, u64::MAX);
        let node_id_bytes = node_id.convert_to_bytes();

        let mut tx = WalTransaction::new();

        tx.write_bytes(FileId::NodeId, offset, node_id_bytes);
        self.wal_manager.commit(&tx);

        Ok(self.hashing.remove(key))
    }
}
