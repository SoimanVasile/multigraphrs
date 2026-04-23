use std::hash::Hash;

use crate::strategies::direction_strategy::DirectionStrategy;
use crate::core::edge::EdgeView;
use crate::MultiGraph;
use crate::storage::storage_backend::StorageBackend;

pub struct NodeIter<'a, K, W, S, B>
where
    K: Clone + Eq + Hash,
    W: Clone + PartialEq,
    S: DirectionStrategy<W>,
    B: StorageBackend<W>
{
    pub(crate) graph: &'a MultiGraph<K, W, S, B>,
    pub(crate) index: u32
}

impl<'a, K, W, S, B> Iterator for NodeIter<'a, K, W, S, B>
where
    K: Clone + Eq + Hash,
    W: Clone + PartialEq,
    S: DirectionStrategy<W>,
    B: StorageBackend<W>
{
    type Item = (&'a K, Vec<EdgeView<K, W>>);
    fn next(&mut self) -> Option<Self::Item>{
        if self.graph.next_id <= self.index{
            return None;
        }

        while self.index < self.graph.next_id && self.graph.reversed_hashed_nodes[self.index as usize].is_none(){
            self.index += 1;
        }

        if self.index >= self.graph.next_id {
            return None;
        }

        let current = self.index;
        self.index += 1;
        let neighbours: Vec<_> = self.graph.adjacency_list.get_edges(current).collect();
        Some((self.graph.reversed_hashed_nodes[current as usize].as_ref().unwrap(), neighbours.into_iter()
            .map(|e| EdgeView::new(self.graph.reversed_hashed_nodes[e.get_target() as usize].as_ref().unwrap(), &e.weight))
            .collect()
        ))
    }
}
