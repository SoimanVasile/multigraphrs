use std::hash::Hash;

use crate::DirectionStrategy;
use crate::EdgeView;
use crate::MultiGraph;

pub struct NodeIter<'a, K, W, S>
where
    K: Clone + Eq + Hash,
    W: Clone + PartialEq,
    S: DirectionStrategy<W>
{
    pub(crate) graph: &'a MultiGraph<K, W, S>,
    pub(crate) index: usize
}

impl<'a, K, W, S> Iterator for NodeIter<'a, K, W, S>
where
    K: Clone + Eq + Hash,
    W: Clone + PartialEq,
    S: DirectionStrategy<W>,
{
    type Item = (&'a K, Vec<EdgeView<K, W>>);
    fn next(&mut self) -> Option<Self::Item>{
        if self.graph.next_id as usize <= self.index{
            return None;
        }

        while self.index < self.graph.next_id as usize && self.graph.reversed_hashed_nodes[self.index].is_none(){
            self.index += 1;
        }

        if self.index >= self.graph.next_id as usize {
            return None;
        }

        let current = self.index;
        self.index += 1;
        let neighbours = self.graph.adjacency_list.get_edges_ref(&current);
        Some((self.graph.reversed_hashed_nodes[current].as_ref().unwrap(), neighbours.iter()
            .map(|e| EdgeView::new(self.graph.reversed_hashed_nodes[e.get_target()].as_ref().unwrap(), &e.weight))
            .collect()
        ))
    }
}
