use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;
use std::hash::Hash;

use crate::strategies::direction_strategy::DirectionStrategy;
use crate::core::edge::EdgeView;
use crate::MultiGraph;
use crate::storage::storage_backend::StorageBackend;

pub struct NodeIter<'a, K, W, S, B>
where
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes,
    W: Clone + PartialEq + AsDiskBytes + FromDiskBytes,
    S: DirectionStrategy<K, W>,
    B: StorageBackend<K, W>
{
    pub(crate) graph: &'a MultiGraph<K, W, S, B>,
    pub(crate) number_of_nodes: u64,
    pub(crate) index: u64,
}

impl<'a, K, W, S, B> Iterator for NodeIter<'a, K, W, S, B>
where
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes,
    W: Clone + PartialEq + AsDiskBytes + FromDiskBytes,
    S: DirectionStrategy<K, W>,
    B: StorageBackend<K, W>
{
    type Item = (K, Vec<EdgeView<K, W>>);
    
    /// Advances the iterator and returns the next node and its outgoing edges.
    ///
    /// # Panics
    /// Panics if an edge's target node cannot be unwrapped from the reverse lookup (indicating an internal bug).
    fn next(&mut self) -> Option<Self::Item>{
        if self.number_of_nodes == 0{
            return None
        }

        while self.graph.adjacency_list.reverse_hashing_get_node_data(self.index).is_none(){
            self.index += 1;
        }

        let current = self.index;
        self.index += 1;
        self.number_of_nodes -= 1;
        let neighbours: Vec<_> = self.graph.adjacency_list.get_edges(&current).collect();
        Some((self.graph.adjacency_list.reverse_hashing_get_node_data(current).unwrap().clone(), neighbours.into_iter()
            .map(|e| EdgeView::new(&self.graph.adjacency_list.reverse_hashing_get_node_data(e.get_target()).unwrap(), &e.weight))
            .collect()
        ))
    }
}
