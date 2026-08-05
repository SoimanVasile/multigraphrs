use std::{hash::Hash, marker::PhantomData, path::Path};

use crate::{DirectionStrategy, DiskStorage, dictionary::dictionary_strategy::DictionaryStrategy, storage::disk_storage::{file_manager::FileManager, from_disk_bytes::{FromDiskBytes, AsDiskBytes}, wal::WalManager}};

use crate::dictionary::node_id::NodeId;

#[derive(Debug)]
pub struct DiskDictionary<K>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes
{
    file_node_id: FileManager,
    file_data: FileManager,
    wal_manager: WalManager,
    node_type: PhantomData<K>
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
        let node_id_value_path = dir.join("node_id_value.bin");

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
        }
    }

    fn calculate_node_id_offset(&self, node_id: u64) -> u64{
        node_id * size_of::<NodeId>() as u64
    }
}

impl<K> DictionaryStrategy<K> for DiskDictionary<K>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
{
    fn contains_key(&self, _key: &K) -> bool {
        todo!()
    }

    fn insert(&mut self, key: K, node_id: u64) {
        let offset = self.calculate_node_id_offset(node_id);
        let data_offset = self.file_data.file_len().unwrap();
        let data_len = size_of::<K>() as u64;
        let node_id = NodeId::new(data_len, data_offset);

        let bytes = node_id.convert_to_bytes();

        self.file_node_id.writing_bytes_to_mmap(offset, offset + size_of::<NodeId>() as u64, bytes);

        let key_bytes = key.as_disk_bytes();
        self.file_data.writing_bytes_to_mmap(data_offset, data_offset + data_len, &key_bytes);
    }

    fn get(&self, _key: &K) -> Option<u64> {
        todo!()
    }

    fn remove(&mut self, _key: &K) -> Option<u64> {
        todo!()
    }
}
