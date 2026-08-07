use crate::core::db_error::DbError;
use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;
use std::hash::Hash;

use ahash::AHashMap;

use crate::dictionary::dictionary_strategy::DictionaryStrategy;

/// In-memory dictionary implementation mapping node data to internal IDs.
///
/// Keeps track of node entries strictly in RAM using a hash map.
pub struct RamDictionary<K>
where
    K: Clone + Hash + Eq + AsDiskBytes + FromDiskBytes
{
    hashed_nodes: AHashMap<K, u64>,
    reverese_hashed_nodes: Vec<Option<K>>,
}

impl<K> DictionaryStrategy<K> for RamDictionary<K>
where
    K: Clone + Hash + Eq + AsDiskBytes + FromDiskBytes
{
    /// Checks if a node exists in the in-memory graph.
    ///
    /// # Arguments
    /// * `key` - The node data to check for existence in the dictionary.
    fn contains_key (&self, key: &K ) -> bool {
        self.hashed_nodes.contains_key(key)
    }

    /// Inserts node data into the in-memory graph. Returns `Ok(None)` if the node didn't exist. If the key already exists, it returns the old node ID.
    ///
    /// # Arguments
    /// * `key` - The node data to insert.
    /// * `node_id` - The internal ID to associate with the node data.
    fn insert ( &mut self, key: K, node_id: u64 ) -> Result<Option<u64>, DbError>{
        self.resize_reverse(node_id);
        self.reverese_hashed_nodes[node_id as usize] = Some(key.clone());
        Ok(self.hashed_nodes.insert( key, node_id))
    }

    /// Retrieves the internal ID of a node for use inside the graph engine from memory.
    ///
    /// # Arguments
    /// * `key` - The node data to retrieve the ID for.
    fn get( &self, key: &K ) -> Option<u64>{
        self.hashed_nodes.get(key).copied()
    }

    /// Removes the key and returns its associated internal ID. Returns `Ok(None)` if the key didn't exist.
    ///
    /// # Arguments
    /// * `key` - The node data to remove from the dictionary.
    fn remove( &mut self, key: &K) -> Result<Option<u64>, DbError>{
        let id = self.hashed_nodes.remove(key);

        if let Some(idx) = id{
            self.reverese_hashed_nodes[idx as usize] = None;
        }

        Ok(id)
    }

    fn reverse_node_data(&self, id: u64) -> Option<K> {
        if id >= self.reverese_hashed_nodes.len() as u64{
            return None;
        }
        self.reverese_hashed_nodes[id as usize].clone()
    }

    fn bulk_insert(&mut self, nodes: &[(K, u64)]) -> Result<(), DbError> {
        for (data, id) in nodes{
            self.insert(data.clone(), *id);
        }
        Ok(())
    }
}

impl<K> RamDictionary<K>
where
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes
{
    /// Creates a new, empty in-memory dictionary.
    pub fn new() -> Self{
        Self { hashed_nodes: AHashMap::new(),
        reverese_hashed_nodes: Vec::new()}
    }

    /// Creates a new, empty in-memory dictionary with the specified capacity.
    ///
    /// # Arguments
    /// * `n` - The initial capacity of the hash map, useful to avoid reallocations.
    pub fn with_capacity(n: usize) -> Self{
        Self{ hashed_nodes: AHashMap::with_capacity(n),
        reverese_hashed_nodes: Vec::with_capacity(n)}
    }

    fn resize_reverse(&mut self, node_id: u64){
        if node_id >= self.reverese_hashed_nodes.len() as u64{
            self.reverese_hashed_nodes.resize(node_id as usize + 1, None);
        }
    }
    
}
