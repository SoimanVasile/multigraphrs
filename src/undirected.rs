use crate::{DirectionStrategy, graph_errors::GraphErrors};
use std::hash::Hash;
use crate::edge::Edge;

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
    K: Eq + Clone + Hash,
{
    /// Adds two edges (source -> target and target -> source) with a weight of `1`.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if the `source` or `target` node 
    /// is missing from the graph's adjacency list.
    fn add_edge(
        graph: &mut std::collections::HashMap<K, Vec<crate::edge::Edge<K, u32>>>, 
        source: &K, 
        target: &K, 
        weight: &u32
    ) -> Result<Vec<crate::edge::Edge<K, u32>>, crate::graph_errors::GraphErrors> {
        
        if !graph.contains_key(source) || !graph.contains_key(target) {
            return Err(GraphErrors::NodeNotFound);
        }

        let edge = Edge::new(target, weight);
        let reverse_edge = Edge::new(source, weight);
        
        graph.entry(source.clone()).or_default().push(edge.clone());
        graph.entry(target.clone()).or_default().push(reverse_edge.clone());
        
        // Returns both edges to confirm the bidirectional connection
        Ok(vec![edge, reverse_edge])
    }
}
