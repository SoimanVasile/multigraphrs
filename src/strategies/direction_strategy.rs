use crate::core::edge::Edge;
use crate::storage::storage_backend::StorageBackend;
use crate::core::graph_errors::GraphError;

pub trait DirectionStrategy<W>
where
    W: Clone + std::cmp::PartialEq,
{
    fn add_edge(graph: &mut impl StorageBackend<W>, source: u64, target: u64, weight: &W) -> Result<Edge<W>, GraphError>;
    fn bulk_add_edge(graph: &mut impl StorageBackend<W>, hashed_nodes: &[(u64, u64, W)]) -> Result<(), GraphError>;
    fn bulk_remove_edge(graph: &mut impl StorageBackend<W>, edges: &[(u64, u64, W)]) -> Result<(), GraphError>;
    fn remove_edge(graph: &mut impl StorageBackend<W>, source: u64, target: u64, weight: &W) -> Result<Edge<W>, GraphError>;
    fn remove_node(graph: &mut impl StorageBackend<W>, node_id: u64) -> Result<(), GraphError>;
}
