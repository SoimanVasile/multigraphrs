use crate::core::db_error::DbError;
use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;
use std::hash::Hash;

/// Abstracts the mapping of node data to internal IDs to ensure graph values have strict sizes.
///
/// `node_id` represents the internal ID of the respective node data.
pub trait DictionaryStrategy<K> 
where
    K: Clone + Hash + Eq + AsDiskBytes + FromDiskBytes
{

    /// Checks if a node exists in the graph.
    ///
    /// # Arguments
    /// * `key` - The node data to check for existence in the dictionary.
    fn contains_key ( &self, key: &K ) -> bool;

    /// Inserts node data into the graph. Returns `Ok(None)` if the node didn't exist. If the key already exists, it returns the old node ID.
    ///
    /// # Arguments
    /// * `key` - The node data to insert.
    /// * `node_id` - The internal ID to associate with the node data.
    ///
    /// # Errors
    /// Returns a [`DbError`] if an underlying storage operation fails during insertion.
    fn insert ( &mut self, key: K, node_id: u64) -> Result<Option<u64>, DbError>;

    /// Retrieves the internal ID of a node for use inside the graph engine.
    ///
    /// # Arguments
    /// * `key` - The node data to retrieve the ID for.
    fn get( &self, key: &K ) -> Option<u64>;

    /// Removes the key and returns its associated internal ID. Returns `Ok(None)` if the key didn't exist.
    ///
    /// # Arguments
    /// * `key` - The node data to remove from the dictionary.
    ///
    /// # Errors
    /// Returns a [`DbError`] if an underlying storage operation fails during removal.
    fn remove(&mut self, key: &K ) -> Result<Option<u64>, DbError>;

    fn reverse_node_data(&self, id: u64) -> Option<K>;

    fn bulk_insert(&mut self, nodes: &[(K, u64)]) -> Result<(), DbError>;
}
