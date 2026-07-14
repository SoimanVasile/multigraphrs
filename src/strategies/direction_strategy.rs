use crate::core::edge::Edge;
use crate::storage::storage_backend::StorageBackend;
use crate::core::graph_errors::GraphErrors;

/// A trait defining how edges are inserted into the graph's adjacency list.
///
/// By implementing this trait, different graph types (Directed, Undirected, etc.) 
/// can share the same core `MultiGraph` structure while maintaining unique behavior.
pub trait DirectionStrategy<W>
where
    W: Clone + std::cmp::PartialEq,
{
    /// Processes the raw source, target, and weight, mutating the `graph` directly.
    /// Returns the edges that were successfully created.
    ///
    /// # Errors
    /// Returns a `GraphErrors` if the operation fails.
    ///
    /// # Side Effects
    /// Mutates the `graph` storage backend to add the edge.
    fn add_edge(graph: &mut impl StorageBackend<W>, source: u64, target: u64, weight: &W) -> Result<Edge<W>, GraphErrors>;

    /// Adds multiple edges to the graph efficiently in bulk.
    ///
    /// # Errors
    /// Returns a `GraphErrors` if the bulk operation fails.
    ///
    /// # Side Effects
    /// Mutates the `graph` storage backend to add multiple edges.
    fn bulk_add_edge(graph: &mut impl StorageBackend<W>, hashed_nodes: &[(u64, u64, W)]) -> Result<(), GraphErrors>;

    /// Removes multiple edges from the graph efficiently in bulk.
    ///
    /// # Side Effects
    /// Mutates the `graph` storage backend to remove multiple edges.
    fn bulk_remove_edge(graph: &mut impl StorageBackend<W>, edges: &[(u64, u64, W)]);

    /// Removes an edge from `source` to `target` with the given `weight`,
    /// using strategy-specific matching and cleanup logic.
    ///
    /// # Returns
    /// The removed `Edge` (**owned**) on success.
    ///
    /// # Errors
    /// Returns `GraphErrors::EdgeDoesntExists` if no matching edge is found.
    ///
    /// # Side Effects
    /// Mutates the `graph` storage backend to remove the edge.
    fn remove_edge(graph: &mut impl StorageBackend<W>, source: u64, target: u64, weight: &W ) -> Result<Edge<W>, GraphErrors>;

    /// Removes a node and all edges connected to it.
    /// The strategy determines how to efficiently find and remove incoming edges.
    ///
    /// # Side Effects
    /// Mutates the `graph` storage backend to remove the node and its incident edges.
    fn remove_node(graph: &mut impl StorageBackend<W>, node_id: u64);
}
