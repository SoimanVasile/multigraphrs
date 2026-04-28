use crate::strategies::direction_strategy::DirectionStrategy;
use crate::storage::storage_backend::StorageBackend;
use crate::core::edge::Edge;
use crate::core::graph_errors::GraphErrors;

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

impl<W> DirectionStrategy<W> for WeightedDirected
where
    W: Clone + std::cmp::PartialEq,
{
    /// Adds a single directed edge from `source` to `target` with the specified `weight`.
    ///
    fn add_edge(
        graph: &mut impl StorageBackend<W>,
        source: u32, 
        target: u32, 
        weight: &W
    ) -> Result<Edge<W>, GraphErrors> {
        
        let edge = Edge::new(target, weight);
        graph.add_edge_to_node(source, &edge);

        // Maintain reverse index: target now has an incoming edge from source
        graph.add_reverse_edge(target, source);
        
        // Returns the single edge that was created
        Ok(edge)
    }

    fn remove_edge(graph: &mut impl StorageBackend<W>, source: u32, target: u32, weight: &W ) -> Result<Edge<W>, GraphErrors> {
        let edge = Edge::new(target, weight);
        let result = graph.remove_edge(source, &edge, |edge_1: &Edge<W>, edge_2: &Edge<W>| -> bool {
            return edge_1.get_weight() == edge_2.get_weight() && edge_1.get_target() == edge_2.get_target();
        })?;

        // Update reverse index
        graph.remove_reverse_edge(target, source);

        Ok(result)
    }

    /// Removes a node and all connected edges. O(degree_in + degree_out).
    ///
    /// Uses the reverse adjacency list to efficiently find and remove all
    /// incoming edges without scanning the entire graph.
    fn remove_node(graph: &mut impl StorageBackend<W>, node_id: u32) {
        // 1. Remove incoming edges: use reverse list to find who points to us
        let incoming = graph.get_reverse_edges(node_id);
        for source in incoming {
            graph.remove_edge_by_target(source, node_id);
        }
        graph.clear_reverse_edges(node_id);

        // 2. Remove outgoing edges: clean up reverse lists of our targets
        let outgoing: Vec<Edge<W>> = graph.get_edges(node_id).collect();
        for edge in outgoing {
            graph.remove_reverse_edge(edge.get_target(), node_id);
        }
        graph.clear_node_edges(node_id);

        graph.decrement_node_counter();
    }
}
