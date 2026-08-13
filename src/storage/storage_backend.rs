use std::hash::Hash;

use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;
use crate::core::edge::Edge;
use crate::core::graph_errors::GraphError;

/// Trait abstracting graph storage, allowing both in-memory (RAM) and
/// disk-backed implementations.
///
/// All methods operate on internal numeric node IDs (`&u64`).
pub trait StorageBackend<K, W>
where
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes,
{
    /// Associated iterator type returned by [`get_edges`](Self::get_edges).
    type EdgeIter<'a>: Iterator<Item=Edge<W>> where Self: 'a, W: 'a, K: 'a;

    /// Appends a clone of `edge` to the adjacency list of `node`, incrementing the internal edge counter.
    ///
    /// # Arguments
    /// * `node` - The internal node ID specifying which adjacency list to append to.
    /// * `edge` - The edge containing the target and weight to be cloned and stored.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the underlying storage fails to append the edge.
    ///
    /// # Panics
    /// Panics if `node` is out of bounds of the internal storage.
    fn add_edge_to_node(&mut self, node: &u64, edge: &Edge<W>) -> Result<(), GraphError>;

    /// Bulk adds multiple edges to their respective nodes.
    ///
    /// # Arguments
    /// * `edges` - A slice of node ID and edge pairs to be inserted in bulk.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the underlying storage fails to add the edges.
    ///
    /// # Panics
    /// Panics if any of the provided node IDs are out of bounds.
    fn bulk_add_edge_to_node(&mut self, edges: &[(u64, Edge<W>)]) -> Result<(), GraphError>;

    /// Creates a new, empty node slot and increments the internal node counter.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the underlying storage fails to create a node.
    fn add_node(&mut self) -> Result<u64, GraphError>;

    /// Bulk adds a specified number of nodes to the storage.
    ///
    /// # Arguments
    /// * `number_of_nodes` - The exact number of node slots to create.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the underlying storage fails to create the nodes.
    fn bulk_add_node(&mut self, number_of_nodes: &u64) -> Result<Vec<u64>, GraphError>;

    /// Returns the number of outgoing edges for `node`.
    ///
    /// # Arguments
    /// * `node` - The internal node ID to query for outgoing edge count.
    ///
    /// # Panics
    /// Panics if `node` is out of bounds.
    fn node_len(&self, node: &u64) -> usize;

    /// Returns an iterator that yields cloned `Edge<W>` values for all outgoing edges of `node`.
    ///
    /// # Arguments
    /// * `node` - The internal node ID whose outgoing edges should be iterated.
    ///
    /// # Panics
    /// Panics if `node` is out of bounds.
    fn get_edges<'a>(&'a self, node: &u64) -> Self::EdgeIter<'a> where W: 'a, K: 'a;

    /// Removes the first edge from `source` which the target and weight match.
    ///
    /// # Arguments
    /// * `source` - The source node ID from which to remove the edge.
    /// * `edge` - The edge containing the target and weight to match for removal.
    ///
    /// # Errors
    /// Returns a [`GraphError::EdgeDoesntExist`] if the edge doesn't exist, or another [`GraphError`] on backend failure.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    fn remove_edge(&mut self, source: &u64, edge: &Edge<W>) -> Result<Edge<W>, GraphError>;

    /// Bulk removes the edges in the `edges` array which the target and weight match.
    ///
    /// # Arguments
    /// * `edges` - A slice of node ID and edge pairs specifying the edges to remove.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the underlying storage fails to remove the edges.
    ///
    /// # Panics
    /// Panics if any source node is out of bounds.
    fn bulk_remove_edge(&mut self, edges: &[(u64, Edge<W>)]) -> Result<(), GraphError>;

    /// Removes the first edge from `source` for which `func(edge, candidate)` returns `true`, using swap-remove semantics.
    ///
    /// # Arguments
    /// * `source` - The source node ID from which to remove the edge.
    /// * `edge` - The edge parameter passed to the matching function.
    /// * `func` - The closure used to determine if a candidate edge matches.
    ///
    /// # Errors
    /// Returns a [`GraphError::EdgeDoesntExist`] if no edge matches, or another [`GraphError`] on backend failure.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    fn remove_edge_by_property<F>(&mut self, source: &u64, edge: &Edge<W>, func: F) -> Result<Edge<W>, GraphError>
    where
        F: Fn(&Edge<W>, &Edge<W>) -> bool;

    /// Searches for an edge from `source` to `target`.
    ///
    /// # Arguments
    /// * `source` - The source node ID where the edge begins.
    /// * `target` - The target node ID where the edge ends.
    ///
    /// # Errors
    /// Returns a [`GraphError::EdgeDoesntExist`] if no such edge exists, or another [`GraphError`] on backend failure.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    fn contains_edge(&self, source: &u64, target: &u64) -> Result<Edge<W>, GraphError>;

    /// Returns the total node count.
    fn node_count(&self) -> usize;

    /// Returns the total edge count.
    fn edge_count(&self) -> usize;

    /// Increments the internal node counter without allocating a new slot.
    /// Used when re-adding a node to a previously freed ID.
    ///
    /// # Errors
    /// Returns a [`GraphError`] on backend failure.
    fn increment_node_counter(&mut self) -> Result<(), GraphError>;

    // --- Primitives for strategy-driven remove_node ---

    /// Clears all outgoing edges from a node and updates the edge count.
    ///
    /// # Arguments
    /// * `node` - The internal node ID whose edges will be cleared.
    ///
    /// # Errors
    /// Returns a [`GraphError`] on backend failure.
    ///
    /// # Panics
    /// Panics if `node` is out of bounds.
    fn clear_node_edges(&mut self, node: &u64) -> Result<(), GraphError>;

    /// Removes the first edge from `source` that points to `target`, updating the edge count.
    ///
    /// # Arguments
    /// * `source` - The internal node ID from which the edge originates.
    /// * `target` - The internal node ID to which the edge points.
    ///
    /// # Errors
    /// Returns a [`GraphError`] on backend failure.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    fn remove_edge_by_target(&mut self, source: &u64, target: &u64) -> Result<(), GraphError>;

    /// Records that `source` has an incoming edge from `origin` (reverse index).
    ///
    /// # Arguments
    /// * `source` - The internal node ID that receives the edge.
    /// * `origin` - The internal node ID that originates the edge.
    ///
    /// # Errors
    /// Returns a [`GraphError`] on backend failure.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    fn add_reverse_edge(&mut self, source: &u64, origin: &u64) -> Result<(), GraphError>;

    /// Bulk adds reverse edge records.
    ///
    /// # Arguments
    /// * `edges` - A slice of tuples containing the source, target, and weight of the edges to reverse-index.
    ///
    /// # Errors
    /// Returns a [`GraphError`] on backend failure.
    ///
    /// # Panics
    /// Panics if any of the target node ids are out of bounds.
    fn bulk_add_reverse_edge(&mut self, edges: &[(u64, u64, W)]) -> Result<(), GraphError>;

    /// Returns all node IDs that have outgoing edges pointing to `node`.
    ///
    /// # Arguments
    /// * `node` - The internal node ID to query for incoming edges.
    ///
    /// # Panics
    /// Panics if `node` is out of bounds.
    fn get_reverse_edges(&self, node: &u64) -> Vec<u64>;

    /// Clears the reverse edge list for a node.
    ///
    /// # Arguments
    /// * `node` - The internal node ID whose reverse edges will be cleared.
    ///
    /// # Errors
    /// Returns a [`GraphError`] on backend failure.
    ///
    /// # Panics
    /// Panics if `node` is out of bounds.
    fn clear_reverse_edges(&mut self, node: &u64) -> Result<(), GraphError>;

    /// Removes a single reverse entry where `origin` no longer points to `source`.
    ///
    /// # Arguments
    /// * `source` - The internal node ID that was receiving the edge.
    /// * `origin` - The internal node ID that was originating the edge.
    ///
    /// # Errors
    /// Returns a [`GraphError`] on backend failure.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    fn remove_reverse_edge(&mut self, source: &u64, origin: &u64) -> Result<(), GraphError>;

    /// Bulk removes reverse edge records.
    ///
    /// # Arguments
    /// * `edges` - A slice of tuples containing the source and origin of the reverse edges to remove.
    ///
    /// # Errors
    /// Returns a [`GraphError`] on backend failure.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds.
    fn bulk_remove_reverse_edge(&mut self, edges: &[(u64, u64)]) -> Result<(), GraphError>;

    /// Decrements the node counter and marks the ID as free.
    ///
    /// # Arguments
    /// * `node_id` - The internal node ID to free.
    ///
    /// # Errors
    /// Returns a [`GraphError`] on backend failure.
    fn free_node_id(&mut self, node_id: &u64) -> Result<(), GraphError>;

    fn hashed_nodes_contains_key(&self, key: &K) -> Result<bool, GraphError>;

    fn hashed_nodes_insert(&mut self, key: K, node_id: u64) -> Result<(), GraphError>;

    fn hashed_nodes_get(&self,  key: &K) -> Result<Option<u64>, GraphError>;

    fn hashed_nodes_remove(&mut self, key: &K) -> Result<Option<u64>, GraphError>;

    fn reverse_hashing_get_node_data(&self, id: u64) -> Option<K>;

    fn hashed_nodes_bulk_insert(&mut self, nodes: &[(K, u64)]) -> Result<(), GraphError>;
}
