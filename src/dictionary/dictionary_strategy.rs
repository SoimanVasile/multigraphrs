use std::hash::Hash;

pub trait DictionaryStrategy<K> 
where
    K: Clone + Hash + Eq
{
    fn contains_key ( &self, key: &K ) -> bool;

    fn insert ( &mut self, key: K, node_id: u64);

    fn get( &self, key: &K ) -> Option<&u64>;

    fn remove( &mut self, key: &K ) -> Option<u64>;
}
