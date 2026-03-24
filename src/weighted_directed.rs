use crate::{DirectionStrategy, adjacency_list::AdjacencyList, edge::Edge, graph_errors::GraphErrors};

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
    W: Clone, // Note: Kept as Clone based on our f64 discussion
{
    /// Adds a single directed edge from `source` to `target` with the specified `weight`.
    ///
    fn add_edge(
        graph: &mut AdjacencyList<W>,
        source: &usize, 
        target: &usize, 
        weight: &W
    ) -> Result<Vec<Edge<W>>, GraphErrors> {
        
        let edge = Edge::new(target, weight);
        graph.add_edge_to_node(source, &edge);
        
        // Returns the single edge that was created
        Ok(vec![edge])
    }
}
