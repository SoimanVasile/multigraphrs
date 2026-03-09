use std::{collections::HashMap, hash::Hash, marker::PhantomData, ops::Mul};
mod edge;
mod direction_strategy;
use direction_strategy::{DirectionStrategy, Weighted, Directed};
use edge::Edge;


pub struct MultiGraph<K, W, S:DirectionStrategy<K, W>>
where
    K: Eq + Hash + Clone,
    W: Eq + Hash + Clone,
{
    pub adjacency_list: HashMap<K, Vec<Edge<K, W>>>,
    pub _strategy: PhantomData<S>,
}

impl<K, W> MultiGraph<K, W, Weighted>
where
    K: Eq + Hash + Clone,
    W: Eq + Hash + Clone,
{
    pub fn new() -> MultiGraph<K, W, Weighted>{
        MultiGraph::<K, W, Weighted> { adjacency_list: HashMap::new(), _strategy: PhantomData}
    }
    pub fn add_edge(&mut self, source: K, target: K, weight: W){
        Weighted::add_edge(&mut self.adjacency_list, source, target, weight);
    }
}

impl<K> MultiGraph<K, u32, Directed>
where
    K: Eq + Hash + Clone,
{
    pub fn new() -> MultiGraph<K, u32, Directed>{
        MultiGraph { adjacency_list: HashMap::new(), _strategy:  PhantomData}
    }
    pub fn add_edge(&mut self, source: K, target: K){
        Directed::add_edge(&mut self.adjacency_list, source, target, 1);
    }
}

