use std::hash::Hash;
use crate::{DirectionStrategy, edge::Edge, graph_errors::GraphErrors};

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
    K: Eq + Hash + Clone,
    W: Clone, // Note: Kept as Clone based on our f64 discussion
{
    /// Adds a single directed edge from `source` to `target` with the specified `weight`.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if the `source` or `target` node 
    /// is missing from the graph's adjacency list.
    fn add_edge(
        graph: &mut std::collections::HashMap<K, Vec<Edge<K, W>>>, 
        source: &K, 
        target: &K, 
        weight: &W
    ) -> Result<Vec<Edge<K, W>>, GraphErrors> {
        
        if !graph.contains_key(source) || !graph.contains_key(target) {
            return Err(GraphErrors::NodeNotFound);
        }

        let edge = Edge::new(target, weight);
        graph.entry(source.clone()).or_default().push(edge.clone());
        
        // Returns the single edge that was created
        Ok(vec![edge])
    }
}
