use std::hash::Hash;

use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;
use crate::strategies::direction_strategy::DirectionStrategy;
use crate::core::edge::Edge;
use crate::core::graph_errors::GraphError;
use crate::storage::storage_backend::StorageBackend;

/// A strategy for weighted, undirected graphs.
///
/// In a weighted, undirected graph, an edge represents a two-way connection 
/// between nodes, where the connection has an associated cost, distance, or metadata 
/// (the `weight`). 
///
/// Adding an edge between node A and node B will create two internal edge entries:
/// one from A to B, and one from B to A, both sharing the exact same cloned weight.
#[derive(Debug)]
pub struct Weighted;

impl<K, W> DirectionStrategy<K, W> for Weighted
where
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes,
{
    /// Adds two edges (source -> target and target -> source) with the specified `weight`.
    ///
    /// - `graph`: The underlying storage backend to persist the bidirectional connection.
    /// - `source`: The source node identifier.
    /// - `target`: The target node identifier.
    /// - `weight`: The weight data associated with the edge.
    ///
    /// # Errors
    /// Returns [`GraphError::NodeNotFound`] if the `source` or `target` node is missing.
    fn add_edge(
        graph: &mut impl StorageBackend<K, W>,
        source: u64, 
        target: u64, 
        weight: &W
    ) -> Result<Edge<W>, GraphError> 
    {

        let edge = Edge::new(target, weight);
        let edge_reverse = Edge::new(source, weight);
        
        graph.add_edge_to_node(&source, &edge)?;
        graph.add_edge_to_node(&target, &edge_reverse)?;

        // Returns both edges to confirm the bidirectional connection
        Ok(edge)
    }

    /// Adds multiple weighted undirected edges to the graph efficiently in bulk.
    ///
    /// - `graph`: The underlying storage backend to persist the bulk edges.
    /// - `hashed_nodes`: A slice of tuples containing the source, target, and weight for each edge.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the underlying storage operations fail.
    fn bulk_add_edge(graph: &mut impl StorageBackend<K, W>, hashed_nodes: &[(u64, u64, W)]) -> Result<(), GraphError> {
        let mut edges: Vec<(u64, Edge<W>)> = Vec::with_capacity(hashed_nodes.len());
        let mut reverse_edges: Vec<(u64, Edge<W>)> = Vec::with_capacity(hashed_nodes.len());

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

    /// Removes multiple weighted undirected edges from the graph efficiently in bulk.
    ///
    /// - `graph`: The underlying storage backend to modify.
    /// - `edges`: A slice of tuples containing the source, target, and weight for each edge to remove.
    ///
    /// # Errors
    /// Returns a [`GraphError`] if the underlying storage operations fail.
    fn bulk_remove_edge(graph: &mut impl StorageBackend<K, W>, edges: &[(u64, u64, W)]) -> Result<(), GraphError> {
        let mut edges_to_remove: Vec<(u64, Edge<W>)> = Vec::with_capacity(edges.len() * 2);
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

    /// Removes the undirected, weighted edge between `source` and `target`
    /// matching both target identity **and** weight equality.
    ///
    /// Both the forward and reverse edges are removed.
    ///
    /// - `graph`: The underlying storage backend from which the edges are removed.
    /// - `source`: The source node identifier.
    /// - `target`: The target node identifier.
    /// - `weight`: The edge weight to match for removal.
    ///
    /// # Errors
    /// Returns [`GraphError::EdgeDoesntExist`] if no matching edge is found.
    ///
    /// # Panics
    /// Panics if `source` or `target` is out of bounds in the storage backend.
    fn remove_edge(graph: &mut impl StorageBackend<K, W>, source: u64, target: u64, weight: &W) -> Result<Edge<W>, GraphError> {
        let edge = Edge::new(target, weight);
        let reverse_edge = Edge::new(source, weight);
        graph.remove_edge(&target, &reverse_edge)?;
        graph.remove_edge(&source, &edge)
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
    /// Returns a [`GraphError`] if a storage operation fails.
    fn remove_node(graph: &mut impl StorageBackend<K, W>, node_id: u64) -> Result<(), GraphError> {
        let edges: Vec<Edge<W>> = graph.get_edges(&node_id).collect();
        for edge in edges {
            graph.remove_edge_by_target(&edge.get_target(), &node_id)?;
        }
        graph.clear_node_edges(&node_id)?;
        graph.free_node_id(&node_id)?;
        
        Ok(())
    }
}
