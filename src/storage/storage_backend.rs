use crate::core::edge::Edge;
use crate::core::graph_errors::GraphErrors;

/// Trait abstracting graph storage, allowing both in-memory (RAM) and
/// disk-backed implementations.
///
/// All methods operate on internal numeric node IDs (`u64`).
pub trait StorageBackend<W>
where
    W: Clone + std::cmp::PartialEq,
{
    /// Associated iterator type returned by [`get_edges`](Self::get_edges).
    type EdgeIter<'a>: Iterator<Item=Edge<W>> where Self: 'a, W: 'a;

    /// Appends a **clone** of `edge` to the adjacency list of `node`.
    /// Increments the internal edge counter.
    ///
    /// # Panics
    /// Panics if `node` is out of bounds of the internal storage.
    fn add_edge_to_node(&mut self, node: u64, edge: &Edge<W>);

    /// Creates a new, empty node slot. Increments the node counter.
    ///
    /// # Panics
    /// This method does not panic under normal circumstances.
    fn add_node(&mut self);

    /// Returns the number of outgoing edges for `node` (**copy**, `usize` is `Copy`).
    ///
    /// # Panics
    /// Panics if `node` is out of bounds.
    fn node_len(&self, node: u64) -> usize;

    /// Returns an iterator that yields **cloned** `Edge<W>` values
    /// for all outgoing edges of `node`.
    ///
    /// # Panics
    /// Panics if `node` is out of bounds.
    fn get_edges<'a>(&'a self, node: u64) -> Self::EdgeIter<'a> where W: 'a;

    /// Removes the first edge from `source` for which `func(edge, candidate)`
    /// returns `true`, using swap-remove semantics.
    ///
    /// # Returns
    /// The removed `Edge` (**owned**) on success.
    ///
    /// # Errors
    /// Returns `GraphErrors::EdgeDoesntExists` if no edge matches.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    fn remove_edge<F>(&mut self, source: u64, edge: &Edge<W>, func: F) -> Result<Edge<W>, GraphErrors>
    where
        F: Fn(&Edge<W>, &Edge<W>) -> bool;

    /// Searches for an edge from `source` to `target`.
    ///
    /// # Returns
    /// A **clone** of the matching `Edge` on success.
    ///
    /// # Errors
    /// Returns `GraphErrors::EdgeDoesntExists` if no such edge exists.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    fn contains_edge(&self, source: u64, target: u64) -> Result<Edge<W>, GraphErrors>;

    /// Returns the total node count (**copy**).
    ///
    /// # Panics
    /// This method does not panic.
    fn node_count(&self) -> usize;

    /// Returns the total edge count (**copy**).
    ///
    /// # Panics
    /// This method does not panic.
    fn edge_count(&self) -> usize;

    /// Increments the internal node counter without allocating a new slot.
    /// Used when re-adding a node to a previously freed ID.
    ///
    /// # Panics
    /// This method does not panic.
    fn increment_node_counter(&mut self);

    // --- Primitives for strategy-driven remove_node ---

    /// Clears all outgoing edges from a node and updates the edge count.
    fn clear_node_edges(&mut self, node: u64);

    /// Removes the first edge from `source` that points to `target`.
    /// Updates the edge count.
    fn remove_edge_by_target(&mut self, source: u64, target: u64);

    /// Records that `source` has an incoming edge from `origin` (reverse index).
    fn add_reverse_edge(&mut self, source: u64, origin: u64);

    /// Returns all node IDs that have outgoing edges pointing to `node`.
    fn get_reverse_edges(&self, node: u64) -> Vec<u64>;

    /// Clears the reverse edge list for a node.
    fn clear_reverse_edges(&mut self, node: u64);

    /// Removes a single reverse entry: `origin` no longer points to `source`.
    fn remove_reverse_edge(&mut self, source: u64, origin: u64);

    /// Decrements the node counter.
    fn decrement_node_counter(&mut self);
}
