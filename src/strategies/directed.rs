use crate::storage::storage_backend::StorageBackend;
use crate::strategies::direction_strategy::DirectionStrategy;
use crate::core::graph_errors::GraphErrors;
use crate::core::edge::Edge;

/// A strategy for unweighted, directed graphs.
///
/// In a directed graph, an edge from node A to node B does not imply 
/// a connection from node B back to node A. 
/// 
/// Because this is an "unweighted" strategy, the `MultiGraph` will automatically 
/// assign a default weight of `1` (as a `u32`) to every edge created.
pub struct Directed;

impl DirectionStrategy<u32> for Directed
{
    /// Adds a single directed edge from `source` to `target` with a weight of `1`.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if the `source` or `target` node 
    /// is missing from the graph's adjacency list.
    fn add_edge(
        graph: &mut impl StorageBackend<u32>,
        source: u32, 
        target: u32, 
        weight: &u32
    ) -> Result<Edge<u32>, GraphErrors> {

        let edge = Edge::new(target, weight);
        graph.add_edge_to_node(source, &edge);

        // Maintain reverse index: target now has an incoming edge from source
        graph.add_reverse_edge(target, source);
        
        // Returns the single edge that was created
        Ok(edge)
    }

    fn remove_edge(graph: &mut impl StorageBackend<u32>, source: u32, target: u32, weight: &u32 ) -> Result<Edge<u32>, GraphErrors> {
        let edge = Edge::new(target, weight);
        let result = graph.remove_edge(source, &edge, |edge_1: &Edge<u32>, edge_2: &Edge<u32>| -> bool {
            return edge_1.get_target() == edge_2.get_target();
        })?;

        // Update reverse index: target no longer has this incoming edge from source
        graph.remove_reverse_edge(target, source);

        Ok(result)
    }

    /// Removes a node and all connected edges. O(degree_in + degree_out).
    ///
    /// Uses the reverse adjacency list to efficiently find and remove all
    /// incoming edges without scanning the entire graph.
    fn remove_node(graph: &mut impl StorageBackend<u32>, node_id: u32) {
        // 1. Remove incoming edges: use reverse list to find who points to us
        let incoming = graph.get_reverse_edges(node_id);
        for source in incoming {
            graph.remove_edge_by_target(source, node_id);
        }
        graph.clear_reverse_edges(node_id);

        // 2. Remove outgoing edges: clean up reverse lists of our targets
        let outgoing: Vec<Edge<u32>> = graph.get_edges(node_id).collect();
        for edge in outgoing {
            graph.remove_reverse_edge(edge.get_target(), node_id);
        }
        graph.clear_node_edges(node_id);

        graph.decrement_node_counter();
    }
}
