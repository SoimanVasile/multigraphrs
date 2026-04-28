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
    fn add_edge(graph: &mut impl StorageBackend<W>, source: u32, target: u32, weight: &W) -> Result<Edge<W>, GraphErrors>;

    fn remove_edge(graph: &mut impl StorageBackend<W>, source: u32, target: u32, weight: &W ) -> Result<Edge<W>, GraphErrors>;

    /// Removes a node and all edges connected to it.
    /// The strategy determines how to efficiently find and remove incoming edges.
    fn remove_node(graph: &mut impl StorageBackend<W>, node_id: u32);
}
