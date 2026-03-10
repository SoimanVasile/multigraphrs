use crate::{DirectionStrategy, graph_errors::GraphErrors};
use std::hash::Hash;
use crate::edge::Edge;

pub struct Undirected;
impl<K> DirectionStrategy<K, u32> for Undirected
where
    K: Eq + Clone + Hash{
    fn add_edge(graph: &mut std::collections::HashMap<K, Vec<crate::edge::Edge<K, u32>>>, source: &K, target: &K, weight: &u32) -> Result<Vec<crate::edge::Edge<K, u32>>, crate::graph_errors::GraphErrors> {
        if !graph.contains_key(&source) || !graph.contains_key(&target){
            return Err(GraphErrors::NodeNotFound);
        }
        let edge = Edge::new(target, weight);
        let reverse_edge = Edge::new(source, weight);
        graph.entry(source.clone()).or_default().push(edge.clone());
        graph.entry(target.clone()).or_default().push(reverse_edge.clone());
        Ok(vec![edge, reverse_edge])
    }
}
