use std::hash::Hash;

use crate::storage::disk_storage::from_disk_bytes::{AsDiskBytes, FromDiskBytes};
use crate::storage::storage_backend::StorageBackend;
use crate::strategies::direction_strategy::DirectionStrategy;
use crate::core::graph_errors::GraphError;
use crate::core::edge::Edge;

/// A strategy for unweighted, directed graphs.
///
/// In a directed graph, an edge from node A to node B does not imply 
/// a connection from node B back to node A. 
/// 
/// Because this is an "unweighted" strategy, the `MultiGraph` will automatically 
/// assign a default weight of `1` (as a `u32`) to every edge created.
pub struct Directed;

impl<K> DirectionStrategy<K, u32> for Directed
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes
{
    /// Adds a single directed edge from `source` to `target` with a weight of `1`.
    ///
    /// - `graph`: The underlying storage backend to persist the new edge.
    /// - `source`: The source node identifier from which the edge originates.
    /// - `target`: The target node identifier to which the edge points.
    /// - `weight`: The edge weight (expected to be `1` for unweighted graphs).
    ///
    /// # Errors
    /// Returns [`GraphError::NodeNotFound`] if the `source` or `target` node is missing.
    fn add_edge(
        graph: &mut impl StorageBackend<K, u32>,
        source: u64, 
        target: u64, 
        weight: &u32
    ) -> Result<Edge<u32>, GraphError> {

        let edge = Edge::new(target, weight);
        graph.add_edge_to_node(&source, &edge)?;

        // Maintain reverse index: target now has an incoming edge from source
        graph.add_reverse_edge(&target, &source)?;
        
        // Returns the single edge that was created
        Ok(edge) }

    /// Adds multiple unweighted directed edges to the graph efficiently in bulk.
    ///
    /// - `graph`: The underlying storage backend to persist the bulk edges.
    /// - `hashed_nodes`: A slice of tuples containing the source, target, and weight for each edge.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the underlying storage operations fail.
    fn bulk_add_edge(graph: &mut impl StorageBackend<K, u32>, hashed_nodes: &[(u64, u64, u32)]) -> Result<(), GraphError> {
        let mut edges: Vec<(u64, Edge<u32>)> = Vec::with_capacity(hashed_nodes.len());
        for (source, target, weight) in hashed_nodes{
            let edge = Edge::new(*target, weight);
            edges.push((*source, edge));
        }
        graph.bulk_add_edge_to_node(&edges)?;
        graph.bulk_add_reverse_edge(hashed_nodes)?;

        Ok(())
    }

    /// Removes multiple unweighted directed edges from the graph efficiently in bulk.
    ///
    /// - `graph`: The underlying storage backend to modify.
    /// - `edges`: A slice of tuples containing the source, target, and weight for each edge to remove.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the underlying storage operations fail.
    fn bulk_remove_edge(graph: &mut impl StorageBackend<K, u32>, edges: &[(u64, u64, u32)]) -> Result<(), GraphError> {
        let mut edges_to_remove: Vec<(u64, Edge<u32>)> = Vec::with_capacity(edges.len());
        let mut reverse_edges_to_remove: Vec<(u64, u64)> = Vec::with_capacity(edges.len());
        
        for (source, target, weight) in edges {
            edges_to_remove.push((*source, Edge::new(*target, weight)));
            reverse_edges_to_remove.push((*target, *source));
        }
        
        graph.bulk_remove_edge(&edges_to_remove)?;
        graph.bulk_remove_reverse_edge(&reverse_edges_to_remove)?;

        Ok(())
    }

    /// Removes a single directed edge from `source` to `target`.
    ///
    /// Matching is performed by target identity only (ignores weight for
    /// unweighted graphs). Also updates the reverse adjacency index.
    ///
    /// - `graph`: The underlying storage backend from which the edge is removed.
    /// - `source`: The source node identifier from which the edge originates.
    /// - `target`: The target node identifier to which the edge points.
    /// - `weight`: The edge weight (ignored for matching in unweighted graphs).
    ///
    /// # Errors
    /// Returns [`GraphError::EdgeDoesntExist`] if no matching edge is found.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds in the storage backend.
    fn remove_edge(graph: &mut impl StorageBackend<K, u32>, source: u64, target: u64, weight: &u32 ) -> Result<Edge<u32>, GraphError> {
        let edge = Edge::new(target, weight);
        let result = graph.remove_edge(&source, &edge)?;

        // Update reverse index: target no longer has this incoming edge from source
        graph.remove_reverse_edge(&target, &source)?;

        Ok(result)
    }

    /// Removes a node and all connected edges. O(degree_in + degree_out).
    ///
    /// Uses the reverse adjacency list to efficiently find and remove all
    /// incoming edges without scanning the entire graph.
    ///
    /// - `graph`: The underlying storage backend from which the node and edges are removed.
    /// - `node_id`: The node identifier to remove from the graph.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if a storage operation fails.
    fn remove_node(graph: &mut impl StorageBackend<K, u32>, node_id: u64) -> Result<(), GraphError> {
        // 1. Remove incoming edges: use reverse list to find who points to us
        let incoming = graph.get_reverse_edges(&node_id);
        for source in incoming {
            graph.remove_edge_by_target(&source, &node_id)?;
        }
        graph.clear_reverse_edges(&node_id)?;

        // 2. Remove outgoing edges: clean up reverse lists of our targets
        let outgoing: Vec<Edge<u32>> = graph.get_edges(&node_id).collect();
        for edge in outgoing {
            graph.remove_reverse_edge(&edge.get_target(), &node_id)?;
        }
        graph.clear_node_edges(&node_id)?;

        graph.free_node_id(&node_id)?;
        
        Ok(())
    }
}
