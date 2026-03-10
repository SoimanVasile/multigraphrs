use crate::DirectionStrategy;
use crate::Edge;
use crate::GraphErrors;
use std::collections::HashMap;
use std::hash::Hash;


#[derive(Debug)]
pub struct Weighted;
impl<K, W> DirectionStrategy<K, W> for Weighted
where
    K: Eq + Hash + Clone,
    W: Eq + Hash + Clone,
{
    fn add_edge(graph: &mut HashMap<K, Vec<Edge<K, W>>>, source: &K, target: &K, weight: &W) -> Result<Vec<Edge<K, W>>, GraphErrors> {

        if !graph.contains_key(&source) || !graph.contains_key(&target) {
            return Err(GraphErrors::NodeNotFound);
        }

        let edge = Edge::new(target, weight);
        let edge_reverse = Edge::new(source, weight);
        graph.entry(source.clone()).or_default().push(edge.clone());
        graph.entry(target.clone()).or_default().push(edge_reverse.clone());

        Ok(vec![edge, edge_reverse])
    }
}

