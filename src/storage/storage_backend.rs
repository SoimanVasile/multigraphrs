use crate::core::edge::Edge;
use crate::core::graph_errors::GraphErrors;

pub trait StorageBackend<W>
where
    W: Clone + std::cmp::PartialEq,
{
    type EdgeIter<'a>: Iterator<Item=Edge<W>> where Self: 'a, W: 'a;
    fn add_edge_to_node(&mut self, node: u32, edge: &Edge<W>);
    fn add_node(&mut self);
    fn node_len(&self, node: u32) -> usize;
    fn get_edges<'a>(&'a self, node: u32) -> Self::EdgeIter<'a> where W: 'a;
    fn remove_edge<F>(&mut self, source: u32, edge: &Edge<W>, func: F) -> Result<Edge<W>, GraphErrors>
    where
        F: Fn(&Edge<W>, &Edge<W>) -> bool;
    fn contains_edge(&self, source: u32, target: u32) -> Result<Edge<W>, GraphErrors>;
    fn node_count(&self) -> usize;
    fn edge_count(&self) -> usize;
    fn increment_node_counter(&mut self);

    // --- Primitives for strategy-driven remove_node ---

    /// Clears all outgoing edges from a node and updates the edge count.
    fn clear_node_edges(&mut self, node: u32);

    /// Removes the first edge from `source` that points to `target`.
    /// Updates the edge count.
    fn remove_edge_by_target(&mut self, source: u32, target: u32);

    /// Records that `source` has an incoming edge from `origin` (reverse index).
    fn add_reverse_edge(&mut self, source: u32, origin: u32);

    /// Returns all node IDs that have outgoing edges pointing to `node`.
    fn get_reverse_edges(&self, node: u32) -> Vec<u32>;

    /// Clears the reverse edge list for a node.
    fn clear_reverse_edges(&mut self, node: u32);

    /// Removes a single reverse entry: `origin` no longer points to `source`.
    fn remove_reverse_edge(&mut self, source: u32, origin: u32);

    /// Decrements the node counter.
    fn decrement_node_counter(&mut self);
}
