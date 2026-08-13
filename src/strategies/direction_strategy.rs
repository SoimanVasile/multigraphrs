use std::hash::Hash;

use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;
use crate::core::edge::Edge;
use crate::storage::storage_backend::StorageBackend;
use crate::core::graph_errors::GraphError;

/// Defines a strategy for managing edges in a graph based on directionality and weight.
pub trait DirectionStrategy<K, W>
where
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes,
{
    /// Adds a single edge between `source` and `target` with the given `weight`.
    /// 
    /// Depending on the strategy, this may add a single directed edge or multiple edges
    /// to represent undirected connections. It may also update auxiliary data structures
    /// like reverse indices.
    ///
    /// - `graph`: The underlying storage backend where the edges will be added to persist the graph state.
    /// - `source`: The identifier for the node where the edge originates, necessary to locate the node's edge list.
    /// - `target`: The identifier for the node where the edge terminates, establishing the connection.
    /// - `weight`: The data associated with the edge, used for matching, metadata, or algorithmic costs.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the underlying storage operations fail, such as if a node is missing.
    fn add_edge(graph: &mut impl StorageBackend<K, W>, source: u64, target: u64, weight: &W) -> Result<Edge<W>, GraphError>;

    /// Efficiently adds multiple edges to the graph in a single operation.
    /// 
    /// This is typically faster than adding edges individually as it batches storage operations.
    /// 
    /// - `graph`: The underlying storage backend where the edges will be bulk inserted.
    /// - `hashed_nodes`: A slice of tuples containing the source node, target node, and weight for each edge to be created.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the bulk insertion into the storage backend fails.
    fn bulk_add_edge(graph: &mut impl StorageBackend<K, W>, hashed_nodes: &[(u64, u64, W)]) -> Result<(), GraphError>;

    /// Efficiently removes multiple edges from the graph in a single operation.
    /// 
    /// - `graph`: The underlying storage backend from which edges will be bulk removed.
    /// - `edges`: A slice of tuples containing the source node, target node, and weight for each edge to remove.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the bulk deletion from the storage backend fails.
    fn bulk_remove_edge(graph: &mut impl StorageBackend<K, W>, edges: &[(u64, u64, W)]) -> Result<(), GraphError>;

    /// Removes a single edge between `source` and `target` matching the given `weight`.
    /// 
    /// This removes the forward edge, and for undirected graphs, the reverse edge as well.
    /// For directed graphs, it also cleans up any reverse adjacency indices.
    /// 
    /// - `graph`: The underlying storage backend from which the edge is removed.
    /// - `source`: The identifier for the node where the edge originates.
    /// - `target`: The identifier for the node where the edge terminates.
    /// - `weight`: The weight used to exactly match the edge to be removed, distinguishing it in multigraphs.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the edge does not exist or if a storage operation fails.
    fn remove_edge(graph: &mut impl StorageBackend<K, W>, source: u64, target: u64, weight: &W) -> Result<Edge<W>, GraphError>;

    /// Removes a node and all of its connected edges from the graph.
    /// 
    /// This ensures that no dangling edges remain by cleaning up both outgoing and 
    /// incoming edges connected to the node.
    /// 
    /// - `graph`: The underlying storage backend from which the node is removed.
    /// - `node_id`: The identifier for the node to be removed.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the node removal or associated edge cleanup fails in the storage backend.
    fn remove_node(graph: &mut impl StorageBackend<K, W>, node_id: u64) -> Result<(), GraphError>;
}
