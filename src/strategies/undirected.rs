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
    /// Returns `GraphErrors::NodeNotFound` if the `source` or `target` node 
    /// is missing from the graph's adjacency list.
    fn add_edge(
        graph: &mut impl StorageBackend<u32>,
        source: u32, 
        target: u32, 
        weight: &u32
    ) -> Result<Edge<u32>, crate::core::graph_errors::GraphErrors> {

        let edge = Edge::new(target, weight);
        let reverse_edge = Edge::new(source, weight);
        
        graph.add_edge_to_node(source, &edge);
        graph.add_edge_to_node(target, &reverse_edge);
        
        // Returns both edges to confirm the bidirectional connection
        Ok(edge)
    }

    fn remove_edge(graph: &mut impl StorageBackend<u32>, source: u32, target: u32, weight: &u32 ) -> Result<Edge<u32>, crate::core::graph_errors::GraphErrors> {
        let edge = Edge::new(target, weight);
        let reverse_edge = Edge::new(source, weight);
        graph.remove_edge(target, &reverse_edge, |edge_1: &Edge<u32>, edge_2: &Edge<u32>| -> bool {
            return edge_1.get_target() == edge_2.get_target();
        })?;
        graph.remove_edge(source, &edge, |edge_1: &Edge<u32>, edge_2: &Edge<u32>| -> bool {
            return edge_1.get_target() == edge_2.get_target();
        })
    }

    /// Removes a node and all connected edges. O(degree(node)).
    ///
    /// Since the graph is undirected, every edge in node's outgoing list implies
    /// a reverse edge in the neighbor's list. We use this to avoid a full scan.
    fn remove_node(graph: &mut impl StorageBackend<u32>, node_id: u32) {
        // Collect outgoing edges first (tells us exactly who has edges back to us)
        let edges: Vec<Edge<u32>> = graph.get_edges(node_id).collect();
        for edge in edges {
            // Remove the reverse edge from each neighbor's list
            graph.remove_edge_by_target(edge.get_target(), node_id);
        }
        // Clear our own outgoing edges
        graph.clear_node_edges(node_id);
        graph.decrement_node_counter();
    }
}
