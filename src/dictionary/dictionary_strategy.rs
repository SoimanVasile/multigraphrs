use crate::core::db_error::DbError;
use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;
use std::hash::Hash;

pub trait DictionaryStrategy<K> 
where
    K: Clone + Hash + Eq + AsDiskBytes + FromDiskBytes
{
    fn contains_key ( &self, key: &K ) -> bool;

    fn insert ( &mut self, key: K, node_id: u64) -> Result<(), DbError>;

    fn get( &self, key: &K ) -> Option<u64>;

    fn remove( &mut self, key: &K ) -> Result<Option<u64>, DbError>;
}
