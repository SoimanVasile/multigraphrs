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
        
        // Returns the single edge that was created
        Ok(edge)
    }

    fn remove_edge(graph: &mut impl StorageBackend<W>, source: u32, target: u32, weight: &W ) -> Result<Edge<W>, GraphErrors> {
        let edge = Edge::new(target, weight);
        graph.remove_edge(source, &edge, |edge_1: &Edge<W>, edge_2: &Edge<W>| -> bool {
            return edge_1.get_weight() == edge_2.get_weight() && edge_1.get_target() == edge_2.get_target();
        })
    }
}
