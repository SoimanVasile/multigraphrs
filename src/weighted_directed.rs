use std::hash::Hash;

use crate::{DirectionStrategy, edge::Edge, graph_errors::GraphErrors};

pub struct WeightedDirected;
impl<K, W> DirectionStrategy<K, W> for WeightedDirected
where
    K: Eq + Hash + Clone,
    W: Eq + Hash + Clone,{
    fn add_edge(graph: &mut std::collections::HashMap<K, Vec<crate::edge::Edge<K, W>>>, source: &K, target: &K, weight: &W) -> Result<Vec<crate::edge::Edge<K, W>>, crate::graph_errors::GraphErrors> {
        if !graph.contains_key(&source) || !graph.contains_key(&target){
            return Err(GraphErrors::NodeNotFound);
        }

        let edge = Edge::new(target, weight);
        graph.entry(source.clone()).or_default().push(edge.clone());
        Ok(vec![edge])
    }
}

