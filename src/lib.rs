mod edge;
mod direction_strategy;
mod directed;
mod weighted;
mod undirected;
mod weighted_directed;

use direction_strategy::DirectionStrategy;
use directed::Directed;
use edge::Edge;
use std::{collections::HashMap, hash::Hash, marker::PhantomData};
use weighted::Weighted;

use crate::{graph_errors::GraphErrors, undirected::Undirected, weighted_directed::WeightedDirected};

mod graph_errors;


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
    pub fn add_edge(&mut self, source: &K, target: &K, weight: &W) -> Result<Vec<Edge<K, W>>, GraphErrors>{
        Weighted::add_edge(&mut self.adjacency_list, source, target, weight)
    }


    pub fn add_node(&mut self, node: &K) -> Result<K, GraphErrors>
    where K: Eq + Hash + Clone,{
        if !self.adjacency_list.contains_key(node){
            return Err(GraphErrors::NodeAlreadyExists);
        }

        self.adjacency_list.entry(node.clone()).or_default();
        return Ok(node.clone())
    }
}

impl<K, W> MultiGraph<K, W, WeightedDirected>
where
    K: Eq + Hash + Clone,
    W: Eq + Hash + Clone,
{
    pub fn new() -> MultiGraph<K, W, WeightedDirected>{
        MultiGraph { adjacency_list: HashMap::new(), _strategy: PhantomData}
    }

    pub fn add_edge(&mut self, source: &K, target: &K, weight: &W) -> Result<Vec<Edge<K, W>>, GraphErrors>{
        WeightedDirected::add_edge(&mut self.adjacency_list, source, target, weight)
    }

    pub fn add_node(&mut self, node: &K) -> Result<K, GraphErrors>
    where K: Eq + Hash + Clone,{
        if !self.adjacency_list.contains_key(node){
            return Err(GraphErrors::NodeAlreadyExists);
        }

        self.adjacency_list.entry(node.clone()).or_default();
        return Ok(node.clone())
    }
}

impl<K> MultiGraph<K, u32, Directed>
where
    K: Eq + Hash + Clone,
{
    pub fn new() -> MultiGraph<K, u32, Directed>{
        MultiGraph { adjacency_list: HashMap::new(), _strategy:  PhantomData}
    }

    pub fn add_edge(&mut self, source: &K, target: &K) -> Result<Vec<Edge<K, u32>>, GraphErrors>{
        Directed::add_edge(&mut self.adjacency_list, source, target, &1)
    }

    pub fn add_node(&mut self, node: &K) -> Result<K, GraphErrors>
    where K: Eq + Hash + Clone,{
        if !self.adjacency_list.contains_key(node){
            return Err(GraphErrors::NodeAlreadyExists);
        }

        self.adjacency_list.entry(node.clone()).or_default();
        return Ok(node.clone())
    }
}

impl<K> MultiGraph<K, u32, Undirected>
where
    K: Eq + Hash + Clone,
{
    pub fn new() -> MultiGraph<K, u32, Undirected>{
        MultiGraph { adjacency_list: HashMap::new(), _strategy: PhantomData}
    }
    pub fn add_edge(&mut self, source: &K, target: &K) -> Result<Vec<Edge<K, u32>>, GraphErrors>{
        Undirected::add_edge(&mut self.adjacency_list, source, target, &1)
    }

    pub fn add_node(&mut self, node: &K) -> Result<K, GraphErrors>
    where K: Eq + Hash + Clone,{
        if !self.adjacency_list.contains_key(node){
            return Err(GraphErrors::NodeAlreadyExists);
        }

        self.adjacency_list.entry(node.clone()).or_default();
        return Ok(node.clone())
    }
}
