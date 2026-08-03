use std::hash::Hash;

use ahash::AHashMap;

use crate::dictionary::dictionary_strategy::DictionaryStrategy;

pub struct RamDictionary<K>
where
    K: Clone + Hash + Eq
{
    hashed_nodes: AHashMap<K, u64>
}

impl<K> DictionaryStrategy<K> for RamDictionary<K>
where
    K: Clone + Hash + Eq
{
    fn contains_key (&self, key: &K ) -> bool {
        self.hashed_nodes.contains_key(key)
    }

    fn insert ( &mut self, key: K, node_id: u64 ) {
        self.hashed_nodes.insert( key, node_id);
    }

    fn get( &self, key: &K ) -> Option<&u64>{
        self.hashed_nodes.get(key)
    }

    fn remove( &mut self, key: &K) -> Option<u64>{
        self.hashed_nodes.remove(key)
    }
}

impl<K> RamDictionary<K>
where
    K: Clone + Eq + Hash
{
    pub fn new() -> Self{
        Self { hashed_nodes: AHashMap::new()}
    }

    pub fn with_capacity(n: usize) -> Self{
        Self{ hashed_nodes: AHashMap::with_capacity(n)}
    }
    
}
