use std::hash::Hash;

use crate::storage::disk_storage::from_disk_bytes::{AsDiskBytes, FromDiskBytes};
use crate::storage::storage_backend::StorageBackend;
use crate::strategies::direction_strategy::DirectionStrategy;
use crate::core::edge::Edge;
use crate::core::graph_errors::GraphError;

/// A strategy for unweighted, undirected graphs.
///
/// In an undirected graph, a connection between node A and node B represents 
/// a two-way street. Therefore, adding an edge creates two internal entries: 
/// one from A to B, and one from B to A.
///
/// Because this is an "unweighted" strategy, the `MultiGraph` will automatically 
/// assign a default weight of `1` (as a `u32`) to both edges.
pub struct Undirected;

impl<K> DirectionStrategy<K, u32> for Undirected
where
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes
{
    /// Adds two edges (source -> target and target -> source) with a weight of `1`.
    ///
    /// - `graph`: The underlying storage backend to persist the bidirectional connection.
    /// - `source`: The source node identifier.
    /// - `target`: The target node identifier.
    /// - `weight`: The edge weight (expected to be `1` for unweighted graphs).
    ///
    /// # Errors
    /// Returns [`crate::core::graph_errors::GraphError::NodeNotFound`] if the `source` or `target` node is missing.
    fn add_edge<S: StorageBackend<K, u32>>(
        graph: &mut S,
        source: u64, 
        target: u64, 
        weight: &u32
    ) -> Result<Edge<u32>, GraphError>
    where GraphError: From<S::Error> {

        let edge = Edge::new(target, weight);
        let reverse_edge = Edge::new(source, weight);
        
        graph.add_edge_to_node(&source, &edge)?;
        graph.add_edge_to_node(&target, &reverse_edge)?;
        
        // Returns both edges to confirm the bidirectional connection
        Ok(edge)
    }

    /// Adds multiple undirected edges to the graph efficiently in bulk.
    ///
    /// - `graph`: The underlying storage backend to persist the bulk edges.
    /// - `hashed_nodes`: A slice of tuples containing the source, target, and weight for each edge.
    ///
    /// # Errors
    /// Returns a [`crate::core::graph_errors::GraphError`] if the underlying storage operations fail.
    fn bulk_add_edge<S: StorageBackend<K, u32>>(graph: &mut S, hashed_nodes: &[(u64, u64, u32)]) -> Result<(), GraphError>
    where GraphError: From<S::Error> {
        let mut edges: Vec<(u64, Edge<u32>)> = Vec::with_capacity(hashed_nodes.len());
        let mut reverse_edges: Vec<(u64, Edge<u32>)> = Vec::with_capacity(hashed_nodes.len());
        for (source, target, weight) in hashed_nodes{
            let edge = Edge::new(*target, weight);
            let reverse_edge = Edge::new(*source, weight);
            edges.push((*source, edge));
            reverse_edges.push((*target, reverse_edge));
        }

        graph.bulk_add_edge_to_node(&edges)?;
        graph.bulk_add_edge_to_node(&reverse_edges)?;

        Ok(())
    }

    /// Removes multiple unweighted undirected edges from the graph efficiently in bulk.
    ///
    /// - `graph`: The underlying storage backend to modify.
    /// - `edges`: A slice of tuples containing the source, target, and weight for each edge to remove.
    ///
    /// # Errors
    /// Returns a [`crate::core::graph_errors::GraphError`] if the underlying storage operations fail.
    fn bulk_remove_edge<S: StorageBackend<K, u32>>(graph: &mut S, edges: &[(u64, u64, u32)]) -> Result<(), GraphError>
    where GraphError: From<S::Error> {
        let mut edges_to_remove: Vec<(u64, Edge<u32>)> = Vec::with_capacity(edges.len() * 2);
        let mut reverse_edges_to_remove: Vec<(u64, u64)> = Vec::with_capacity(edges.len() * 2);
        
        for (source, target, weight) in edges {
            edges_to_remove.push((*source, Edge::new(*target, weight)));
            reverse_edges_to_remove.push((*target, *source));
            
            edges_to_remove.push((*target, Edge::new(*source, weight)));
            reverse_edges_to_remove.push((*source, *target));
        }
        
        graph.bulk_remove_edge(&edges_to_remove)?;
        graph.bulk_remove_reverse_edge(&reverse_edges_to_remove)?;

        Ok(())
    }

    /// Removes the undirected edge between `source` and `target`.
    ///
    /// Both the forward (`source` → `target`) and reverse (`target` → `source`)
    /// edges are removed. Matching is by target identity only.
    ///
    /// - `graph`: The underlying storage backend from which the edges are removed.
    /// - `source`: The source node identifier.
    /// - `target`: The target node identifier.
    /// - `weight`: The edge weight (ignored for matching in unweighted graphs).
    ///
    /// # Errors
    /// Returns [`crate::core::graph_errors::GraphError::EdgeDoesntExist`] if no matching edge is found.
    ///
    /// # Panics
    /// Panics if `source` or `target` is out of bounds in the storage backend.
    fn remove_edge<S: StorageBackend<K, u32>>(graph: &mut S, source: u64, target: u64, weight: &u32 ) -> Result<Edge<u32>, GraphError>
    where GraphError: From<S::Error> {
        let edge = Edge::new(target, weight);
        let reverse_edge = Edge::new(source, weight);
        graph.remove_edge(&target, &reverse_edge)?;
        Ok(graph.remove_edge(&source, &edge)?)
    }



    /// Removes a node and all connected edges. O(degree(node)).
    ///
    /// Since the graph is undirected, every edge in node's outgoing list implies
    /// a reverse edge in the neighbor's list. We use this to avoid a full scan.
    ///
    /// - `graph`: The underlying storage backend from which the node and edges are removed.
    /// - `node_id`: The node identifier to remove from the graph.
    ///
    /// # Errors
    /// Returns a [`crate::core::graph_errors::GraphError`] if a storage operation fails.
    fn remove_node<S: StorageBackend<K, u32>>(graph: &mut S, node_id: u64) -> Result<(), GraphError>
    where GraphError: From<S::Error> {
        // Collect outgoing edges first (tells us exactly who has edges back to us)
        let edges: Vec<Edge<u32>> = graph.get_edges(&node_id).collect();
        for edge in edges {
            // Remove the reverse edge from each neighbor's list
            graph.remove_edge_by_target(&edge.get_target(), &node_id)?;
        }
        // Clear our own outgoing edges
        graph.clear_node_edges(&node_id)?;
        graph.free_node_id(&node_id)?;
        
        Ok(())
    }
}
