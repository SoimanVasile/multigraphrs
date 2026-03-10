use std::collections::HashMap;
use std::hash::Hash;
use crate::Edge;
use crate::graph_errors::GraphErrors;

/// A trait defining how edges are inserted into the graph's adjacency list.
///
/// By implementing this trait, different graph types (Directed, Undirected, etc.) 
/// can share the same core `MultiGraph` structure while maintaining unique behavior.
pub trait DirectionStrategy<K, W>
where
    K: Eq + Hash + Clone,
    W: Clone,
{
    /// Processes the raw source, target, and weight, mutating the `graph` directly.
    /// Returns the edges that were successfully created.
    fn add_edge(graph: &mut HashMap<K, Vec<Edge<K, W>>>, source: &K, target: &K, weight: &W) -> Result<Vec<Edge<K, W>>, GraphErrors>;
}
