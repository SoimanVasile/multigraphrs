use crate::storage::storage_backend::StorageBackend;
use crate::strategies::direction_strategy::DirectionStrategy;
use crate::core::edge::Edge;

/// A strategy for unweighted, undirected graphs.
///
/// In an undirected graph, a connection between node A and node B represents 
/// a two-way street. Therefore, adding an edge creates two internal entries: 
/// one from A to B, and one from B to A.
///
/// Because this is an "unweighted" strategy, the `MultiGraph` will automatically 
/// assign a default weight of `1` (as a `u32`) to both edges.
pub struct Undirected;

impl DirectionStrategy<u32> for Undirected
where
{
    /// Adds two edges (source -> target and target -> source) with a weight of `1`.
    ///
    /// # Errors
    /// Returns `GraphError::NodeNotFound` if the `source` or `target` node 
    /// is missing from the graph's adjacency list.
    ///
    /// # Side Effects
    /// Mutates the `graph` storage backend by adding the forward and reverse edges.
    fn add_edge(
        graph: &mut impl StorageBackend<u32>,
        source: u64, 
        target: u64, 
        weight: &u32
    ) -> Result<Edge<u32>, crate::core::graph_errors::GraphError> {

        let edge = Edge::new(target, weight);
        let reverse_edge = Edge::new(source, weight);
        
        graph.add_edge_to_node(&source, &edge)?;
        graph.add_edge_to_node(&target, &reverse_edge)?;
        
        // Returns both edges to confirm the bidirectional connection
        Ok(edge)
    }

    /// Adds multiple undirected edges to the graph efficiently in bulk.
    ///
    /// # Errors
    /// Returns a `GraphError` if the underlying storage operations fail.
    ///
    /// # Side Effects
    /// Mutates the `graph` storage backend to add the given undirected edges in bulk.
    fn bulk_add_edge(graph: &mut impl StorageBackend<u32>, hashed_nodes: &[(u64, u64, u32)]) -> Result<(), crate::core::graph_errors::GraphError> {
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

    fn bulk_remove_edge(graph: &mut impl StorageBackend<u32>, edges: &[(u64, u64, u32)]) -> Result<(), crate::core::graph_errors::GraphError> {
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
    /// # Returns
    /// The removed forward `Edge` (**owned**) on success.
    ///
    /// # Errors
    /// Returns `GraphError::EdgeDoesntExist` if no matching edge is found
    /// in either direction.
    ///
    /// # Panics
    /// Panics if `source` or `target` is out of bounds in the storage backend.
    ///
    /// # Side Effects
    /// Mutates the `graph` storage backend by removing both the forward and reverse edges.
    fn remove_edge(graph: &mut impl StorageBackend<u32>, source: u64, target: u64, weight: &u32 ) -> Result<Edge<u32>, crate::core::graph_errors::GraphError> {
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
    /// # Side Effects
    /// Mutates the `graph` storage backend by clearing all incident edges and freeing the `node_id`.
    fn remove_node(graph: &mut impl StorageBackend<u32>, node_id: u64) -> Result<(), crate::core::graph_errors::GraphError> {
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
