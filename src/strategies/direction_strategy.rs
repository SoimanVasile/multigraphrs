use std::hash::Hash;

use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;
use crate::core::edge::Edge;
use crate::storage::storage_backend::StorageBackend;
use crate::core::graph_errors::GraphError;

pub trait DirectionStrategy<K, W>
where
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes,
{
    fn add_edge(graph: &mut impl StorageBackend<K, W>, source: u64, target: u64, weight: &W) -> Result<Edge<W>, GraphError>;

    fn bulk_add_edge(graph: &mut impl StorageBackend<K, W>, hashed_nodes: &[(u64, u64, W)]) -> Result<(), GraphError>;

    fn bulk_remove_edge(graph: &mut impl StorageBackend<K, W>, edges: &[(u64, u64, W)]) -> Result<(), GraphError>;

    fn remove_edge(graph: &mut impl StorageBackend<K, W>, source: u64, target: u64, weight: &W) -> Result<Edge<W>, GraphError>;

    fn remove_node(graph: &mut impl StorageBackend<K, W>, node_id: u64) -> Result<(), GraphError>;
}
