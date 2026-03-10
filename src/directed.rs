use crate::direction_strategy::DirectionStrategy;
use std::hash::Hash;
use crate::graph_errors::GraphErrors;
use crate::Edge;
use std::collections::HashMap;

/// A strategy for unweighted, directed graphs.
///
/// In a directed graph, an edge from node A to node B does not imply 
/// a connection from node B back to node A. 
/// 
/// Because this is an "unweighted" strategy, the `MultiGraph` will automatically 
/// assign a default weight of `1` (as a `u32`) to every edge created.
pub struct Directed;

impl<K> DirectionStrategy<K, u32> for Directed
where
    K: Eq + Hash + Clone,
{
    /// Adds a single directed edge from `source` to `target` with a weight of `1`.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if the `source` or `target` node 
    /// is missing from the graph's adjacency list.
    fn add_edge(
        graph: &mut HashMap<K, Vec<Edge<K, u32>>>, 
        source: &K, 
        target: &K, 
        weight: &u32
    ) -> Result<Vec<Edge<K, u32>>, GraphErrors> {

        if !graph.contains_key(source) || !graph.contains_key(target) {
            return Err(GraphErrors::NodeNotFound);
        }

        let edge = Edge::new(target, weight);
        graph.entry(source.clone()).or_default().push(edge.clone());
        
        // Returns the single edge that was created
        Ok(vec![edge])
    }
}
