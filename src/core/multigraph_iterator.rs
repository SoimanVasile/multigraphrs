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
    pub(crate) index: u64
}

impl<'a, K, W, S, B> Iterator for NodeIter<'a, K, W, S, B>
where
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes,
    W: Clone + PartialEq + AsDiskBytes + FromDiskBytes,
    S: DirectionStrategy<K, W>,
    B: StorageBackend<K, W>
{
    type Item = (&'a K, Vec<EdgeView<K, W>>);
    
    /// Advances the iterator and returns the next node and its outgoing edges.
    ///
    /// # Side Effects
    /// Mutates the iterator's internal `index` state to point to the next valid node.
    ///
    /// # Panics
    /// Panics if an edge's target node cannot be unwrapped from the reverse lookup (indicating an internal bug).
    ///
    /// # Errors
    /// This function does not return an error; it returns `None` when iteration is complete.
    fn next(&mut self) -> Option<Self::Item>{
        if (self.graph.reversed_hashed_nodes.len() as u64) <= self.index{
            return None;
        }

        while self.index < (self.graph.reversed_hashed_nodes.len() as u64) && self.graph.reversed_hashed_nodes[self.index as usize].is_none(){
            self.index += 1;
        }

        if self.index >= (self.graph.reversed_hashed_nodes.len() as u64) {
            return None;
        }

        let current = self.index;
        self.index += 1;
        let neighbours: Vec<_> = self.graph.adjacency_list.get_edges(&current).collect();
        Some((self.graph.reversed_hashed_nodes[current as usize].as_ref().unwrap(), neighbours.into_iter()
            .map(|e| EdgeView::new(self.graph.reversed_hashed_nodes[e.get_target() as usize].as_ref().unwrap(), &e.weight))
            .collect()
        ))
    }
}
