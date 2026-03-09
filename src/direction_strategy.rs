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
