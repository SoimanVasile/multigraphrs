use std::{collections::HashMap, hash::Hash, marker::PhantomData};
mod edge;
mod direction_strategy;
use direction_strategy::{WeightedStrategy, NonWeightedStrategy, Weighted, Directed};
use edge::Edge;


pub struct MultiGraph<K, W, S:WeightedStrategy<K, W>>
where
    K: Eq + Hash + Clone,
    W: Eq + Hash + Clone,
{
    pub adjacency_list: HashMap<K, Vec<Edge<K, W>>>,
    pub _strategy: PhantomData<S>,
}

pub struct MultiGraph<K, W, S:NonWeightedStrategy<K, W>>
{

}

impl<K, W, S> MultiGraph<K, W, S>
where
    K: Eq + Hash + Clone,
    W: Eq + Hash + Clone,
    S: WeightedStrategy<K, W>,
{
    pub fn new() -> MultiGraph<K, W, S>{
        MultiGraph::<K, W, S> { adjacency_list: HashMap::new(), _strategy: PhantomData::<S>}
    }
    pub fn add_edge(&mut self, source: K, target: K, weight: W){
        S::add_edge(&mut self.adjacency_list, source, target, weight);
    }
}

