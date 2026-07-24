use crate::core::edge::Edge;
use crate::core::graph_errors::GraphError;

/// Trait abstracting graph storage, allowing both in-memory (RAM) and
/// disk-backed implementations.
///
/// All methods operate on internal numeric node IDs (`&u64`).
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
    ///
    /// # Side Effects
    /// Mutates the internal adjacency list and increments the edge count.
    fn add_edge_to_node(&mut self, node: &u64, edge: &Edge<W>) -> Result<(), GraphError>;

    /// Bulks adds multiple edges to their respective nodes.
    ///
    /// # Panics
    /// Panics if any of the provided `node` ids are out of bounds.
    ///
    /// # Side Effects
    /// Mutates the internal adjacency list and increments the edge count by the number of edges.
    fn bulk_add_edge_to_node(&mut self, edges: &[(u64, Edge<W>)]) -> Result<(), GraphError>;

    /// Creates a new, empty node slot. Increments the node counter.
    ///
    /// # Side Effects
    /// Adds a new node entry in the storage and increments the node count.
    fn add_node(&mut self) -> Result<u64, GraphError>;

    /// Bulks adds a specified number of nodes to the storage.
    ///
    /// # Side Effects
    /// Adds `number_of_nodes` entries in the storage and increments the node count.
    fn bulk_add_node(&mut self, number_of_nodes: &u64) -> Result<Vec<u64>, GraphError>;

    /// Returns the number of outgoing edges for `node` (**copy**, `usize` is `Copy`).
    ///
    /// # Panics
    /// Panics if `node` is out of bounds.
    ///
    fn node_len(&self, node: &u64) -> usize;

    /// Returns an iterator that yields **cloned** `Edge<W>` values
    /// for all outgoing edges of `node`.
    ///
    /// # Panics
    /// Panics if `node` is out of bounds.
    ///
    fn get_edges<'a>(&'a self, node: &u64) -> Self::EdgeIter<'a> where W: 'a;

    /// Removes the first edge from `source` which the target and weight match
    ///
    /// # Arguments
    /// * `source` - the source node
    /// * `edge` - which edge should be removed
    ///
    /// # Returns
    /// The removed [`Edge`] (**owned**) on success
    ///
    /// # Error
    /// If the edge doesnt exist it will return [`GraphError::EdgeDoesntExist`]
    ///
    /// # Panics
    /// Panics if `source` is out of bounds
    fn remove_edge(&mut self, source: &u64, edge: &Edge<W>) -> Result<Edge<W>, GraphError>;

    /// Removes the edges in the `edges` array which the target and weight match
    ///
    /// # Arguments
    /// * `edges` - an array with the structure [(source, target, weight)]
    ///
    /// # Panics
    /// Panics if `source` is out of bounds
    fn bulk_remove_edge(&mut self, edges: &[(u64, Edge<W>)]) -> Result<(), GraphError>;

    /// Removes the first edge from `source` for which `func(edge, candidate)`
    /// returns `true`, using swap-remove semantics.
    ///
    /// # Returns
    /// The removed `Edge` (**owned**) on success.
    ///
    /// # Errors
    /// Returns [`GraphError::EdgeDoesntExist`] if no edge matches.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    fn remove_edge_by_property<F>(&mut self, source: &u64, edge: &Edge<W>, func: F) -> Result<Edge<W>, GraphError>
    where
        F: Fn(&Edge<W>, &Edge<W>) -> bool;

    /// Searches for an edge from `source` to `target`.
    ///
    /// # Returns
    /// A **clone** of the matching `Edge` on success.
    ///
    /// # Errors
    /// Returns `GraphError::EdgeDoesntExist` if no such edge exists.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    ///
    fn contains_edge(&self, source: &u64, target: &u64) -> Result<Edge<W>, GraphError>;

    /// Returns the total node count (**copy**).
    ///
    fn node_count(&self) -> usize;

    /// Returns the total edge count (**copy**).
    ///
    fn edge_count(&self) -> usize;

    /// Increments the internal node counter without allocating a new slot.
    /// Used when re-adding a node to a previously freed ID.
    ///
    /// # Side Effects
    /// Increments the internal node count.
    fn increment_node_counter(&mut self) -> Result<(), GraphError>;

    // --- Primitives for strategy-driven remove_node ---

    /// Clears all outgoing edges from a node and updates the edge count.
    ///
    /// # Panics
    /// Panics if `node` is out of bounds.
    ///
    /// # Side Effects
    /// Empties the outgoing edges list for the node and decrements the global edge count.
    fn clear_node_edges(&mut self, node: &u64) -> Result<(), GraphError>;

    /// Removes the first edge from `source` that points to `target`.
    /// Updates the edge count.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    ///
    /// # Side Effects
    /// Mutates the edge list for `source` and decrements the edge count.
    fn remove_edge_by_target(&mut self, source: &u64, target: &u64) -> Result<(), GraphError>;

    /// Records that `source` has an incoming edge from `origin` (reverse index).
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    ///
    /// # Side Effects
    /// Appends `origin` to the reverse adjacency list of `source`.
    fn add_reverse_edge(&mut self, source: &u64, origin: &u64) -> Result<(), GraphError>;

    /// Bulks adds reverse edge records.
    ///
    /// # Panics
    /// Panics if any of the target node ids are out of bounds.
    ///
    /// # Side Effects
    /// Appends sources to the reverse adjacency lists of targets.
    fn bulk_add_reverse_edge(&mut self, edges: &[(u64, u64, W)]) -> Result<(), GraphError>;

    /// Returns all node IDs that have outgoing edges pointing to `node`.
    ///
    /// # Panics
    /// Panics if `node` is out of bounds.
    ///
    fn get_reverse_edges(&self, node: &u64) -> Vec<u64>;

    /// Clears the reverse edge list for a node.
    ///
    /// # Panics
    /// Panics if `node` is out of bounds.
    ///
    /// # Side Effects
    /// Empties the reverse edge list for the specified node.
    fn clear_reverse_edges(&mut self, node: &u64) -> Result<(), GraphError>;

    /// Removes a single reverse entry: `origin` no longer points to `source`.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    ///
    /// # Side Effects
    /// Mutates the reverse adjacency list of `source` by removing `origin`.
    fn remove_reverse_edge(&mut self, source: &u64, origin: &u64) -> Result<(), GraphError>;

    /// Bulks removes reverse edge records.
    ///
    /// # Arguments
    /// * `edges` - an array with the structure [(source, origin)]
    ///
    /// # Panics
    /// Panics if `source` is out of bounds
    fn bulk_remove_reverse_edge(&mut self, edges: &[(u64, u64)]) -> Result<(), GraphError>;

    /// Decrements the node counter.
    ///
    /// # Side Effects
    /// Frees the `node_id` internally and decrements the total node count.
    fn free_node_id(&mut self, node_id: &u64) -> Result<(), GraphError>;
}
