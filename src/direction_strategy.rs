use std::collections::HashMap;
use std::hash::Hash;
use crate::Edge;
use crate::graph_errors::GraphErrors;

pub trait DirectionStrategy<K, W>
where
    K: Eq + Hash + Clone,
    W: Eq + Hash + Clone,
{
    fn add_edge(graph: &mut HashMap<K, Vec<Edge<K, W>>>, source: &K, target: &K, weight: &W) -> Result<Vec<Edge<K, W>>, GraphErrors>;
    // fn remove_edge(graph: &mut HashMap<K, Vec<Edge<K, W>>>, source: K, target: K);
}
