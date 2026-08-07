use std::hash::Hash;

use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;
use crate::strategies::direction_strategy::DirectionStrategy;
use crate::storage::storage_backend::StorageBackend;
use crate::core::edge::Edge;
use crate::core::graph_errors::GraphError;

/// A strategy for weighted, directed graphs.
///
/// In a weighted, directed graph, an edge represents a one-way connection 
/// from a source node to a target node, carrying a specific cost, distance, 
/// or metadata (the `weight`).
///
/// Unlike the undirected `Weighted` strategy, this will not automatically 
/// create a reverse connection. If a two-way connection with different weights 
/// is needed, the user must call `add_edge` twice.
pub struct WeightedDirected;

impl<K, W> DirectionStrategy<K, W> for WeightedDirected
where
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes,
{
    /// Adds a single directed edge from `source` to `target` with the specified `weight`.
    ///
    /// - `graph`: The underlying storage backend to persist the new edge.
    /// - `source`: The source node identifier from which the edge originates.
    /// - `target`: The target node identifier to which the edge points.
    /// - `weight`: The weight data associated with the edge.
    ///
    /// # Errors
    /// Returns [`GraphError::NodeNotFound`] if the source or target nodes do not exist.
    fn add_edge(
        graph: &mut impl StorageBackend<K, W>,
        source: u64, 
        target: u64, 
        weight: &W
    ) -> Result<Edge<W>, GraphError> {
        
        let edge = Edge::new(target, weight);
        graph.add_edge_to_node(&source, &edge)?;

        // Maintain reverse index: target now has an incoming edge from source
        graph.add_reverse_edge(&target, &source)?;
        
        // Returns the single edge that was created
        Ok(edge)
    }

    /// Adds multiple directed edges to the graph efficiently in bulk.
    ///
    /// - `graph`: The underlying storage backend to persist the bulk edges.
    /// - `hashed_nodes`: A slice of tuples containing the source, target, and weight for each edge.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the underlying storage operations fail.
    fn bulk_add_edge(graph: &mut impl StorageBackend<K, W>, hashed_nodes: &[(u64, u64, W)]) -> Result<(), GraphError> {
        let mut edges: Vec<(u64, Edge<W>)> = Vec::with_capacity(hashed_nodes.len());

        for (source, target, weight) in hashed_nodes{
            let edge = Edge::new(*target, weight);
            edges.push((*source, edge));
        }

        graph.bulk_add_edge_to_node(&edges)?;
        let reverse_nodes: Vec<(u64, u64, W)> = hashed_nodes.iter().map(|(s, t, w)| (*s, *t, w.clone())).collect();
        graph.bulk_add_reverse_edge(&reverse_nodes)?;

        Ok(())
    }

    /// Removes multiple weighted directed edges from the graph efficiently in bulk.
    ///
    /// - `graph`: The underlying storage backend to modify.
    /// - `edges`: A slice of tuples containing the source, target, and weight for each edge to remove.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the underlying storage operations fail.
    fn bulk_remove_edge(graph: &mut impl StorageBackend<K, W>, edges: &[(u64, u64, W)]) -> Result<(), GraphError> {
        let mut edges_to_remove: Vec<(u64, Edge<W>)> = Vec::with_capacity(edges.len());
        let mut reverse_edges_to_remove: Vec<(u64, u64)> = Vec::with_capacity(edges.len());
        
        for (source, target, weight) in edges {
            edges_to_remove.push((*source, Edge::new(*target, weight)));
            reverse_edges_to_remove.push((*target, *source));
        }
        
        graph.bulk_remove_edge(&edges_to_remove)?;
        graph.bulk_remove_reverse_edge(&reverse_edges_to_remove)?;

        Ok(())
    }

    /// Removes a single directed, weighted edge from `source` to `target`
    /// matching both target identity **and** weight equality.
    ///
    /// Also updates the reverse adjacency index.
    ///
    /// - `graph`: The underlying storage backend from which the edge is removed.
    /// - `source`: The source node identifier from which the edge originates.
    /// - `target`: The target node identifier to which the edge points.
    /// - `weight`: The edge weight to match for removal.
    ///
    /// # Errors
    /// Returns [`GraphError::EdgeDoesntExist`] if no matching edge is found.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds in the storage backend.
    fn remove_edge(graph: &mut impl StorageBackend<K, W>, source: u64, target: u64, weight: &W ) -> Result<Edge<W>, GraphError> {
        let edge = Edge::new(target, weight);
        let result = graph.remove_edge(&source, &edge)?;

        // Update reverse index
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
    fn remove_node(graph: &mut impl StorageBackend<K, W>, node_id: u64) -> Result<(), GraphError> {
        // 1. Remove incoming edges: use reverse list to find who points to us
        let incoming = graph.get_reverse_edges(&node_id);
        for source in incoming {
            graph.remove_edge_by_target(&source, &node_id)?;
        }
        graph.clear_reverse_edges(&node_id)?;

        // 2. Remove outgoing edges: clean up reverse lists of our targets
        let outgoing: Vec<Edge<W>> = graph.get_edges(&node_id).collect();
        for edge in outgoing {
            graph.remove_reverse_edge(&edge.get_target(), &node_id)?;
        }
        graph.clear_node_edges(&node_id)?;

        graph.free_node_id(&node_id)?;
        
        Ok(())
    }
}
