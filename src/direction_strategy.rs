use std::collections::HashMap;
use std::hash::Hash;
use crate::Edge;

pub trait DirectionStrategy<K, W>
where
    K: Eq + Hash + Clone,
    W: Eq + Hash + Clone,
{
    fn add_edge(graph: &mut HashMap<K, Vec<Edge<K, W>>>, source: K, target:K, weight: W);
    // fn remove_edge(graph: &mut HashMap<K, Vec<Edge<K, W>>>, source: K, target: K);
}

#[derive(Debug)]
pub struct Weighted;
impl<K, W> DirectionStrategy<K, W> for Weighted
where
    K: Eq + Hash + Clone,
    W: Eq + Hash + Clone,
{
    fn add_edge(graph: &mut HashMap<K, Vec<Edge<K, W>>>, source: K, target:K, weight: W) {
        let edge = Edge::new(target.clone(), weight.clone());
        let edge_reverse = Edge::new(source.clone(), weight.clone());
        graph.entry(source).or_default().push(edge);
        graph.entry(target).or_default().push(edge_reverse);
    }
}

pub struct Directed;
impl <K> DirectionStrategy<K, u32> for Directed
where
    K: Eq + Hash + Clone
{
    fn add_edge(graph: &mut HashMap<K, Vec<Edge<K, u32>>>, source: K, target:K, weight: u32) {
        let edge = Edge::new(target.clone(), weight.clone());
        let edge_reverse = Edge::new(source.clone(), weight.clone());
        graph.entry(source).or_default().push(edge);
        graph.entry(target).or_default().push(edge_reverse);
        
    }
}
