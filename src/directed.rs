use crate::direction_strategy::DirectionStrategy;
use std::hash::Hash;
use crate::graph_errors::GraphErrors;
use crate::Edge;
use std::collections::HashMap;


pub struct Directed;
impl <K> DirectionStrategy<K, u32> for Directed
where
    K: Eq + Hash + Clone
{
    fn add_edge(graph: &mut HashMap<K, Vec<Edge<K, u32>>>, source: &K, target:&K, weight: &u32) -> Result<Vec<Edge<K, u32>>, GraphErrors>{

        if !graph.contains_key(&source) || !graph.contains_key(&target) {
            return Err(GraphErrors::NodeNotFound);
        }

        let edge = Edge::new(target, weight);
        graph.entry(source.clone()).or_default().push(edge.clone());
        Ok(vec![edge])
    }
}
