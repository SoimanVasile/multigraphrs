use crate::DirectionStrategy;
use crate::Edge;
use crate::GraphErrors;
use std::collections::HashMap;
use std::hash::Hash;

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
    K: Eq + Hash + Clone,
    W: Clone, // Note: Kept as Clone based on our f64 discussion
{
    /// Adds two edges (source -> target and target -> source) with the specified `weight`.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if the `source` or `target` node 
    /// is missing from the graph's adjacency list.
    fn add_edge(
        graph: &mut HashMap<K, Vec<Edge<K, W>>>, 
        source: &K, 
        target: &K, 
        weight: &W
    ) -> Result<Vec<Edge<K, W>>, GraphErrors> {

        if !graph.contains_key(source) || !graph.contains_key(target) {
            return Err(GraphErrors::NodeNotFound);
        }

        let edge = Edge::new(target, weight);
        let edge_reverse = Edge::new(source, weight);
        
        graph.entry(source.clone()).or_default().push(edge.clone());
        graph.entry(target.clone()).or_default().push(edge_reverse.clone());

        // Returns both edges to confirm the bidirectional connection
        Ok(vec![edge, edge_reverse])
    }
}
