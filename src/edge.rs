use std::{hash::Hash};
use std::clone::Clone;

#[derive(Clone, Debug)]
pub struct Edge<K, W>
where
    K: Eq + Hash + Clone,
    W: Eq + Hash + Clone,
{
    pub target: K,
    pub weight: W,
}

impl<K, W> Edge<K, W>
where
    K: Eq + Hash + Clone,
    W: Eq + Hash + Clone ,
{
    pub fn new(target: &K, weight: &W) -> Edge<K, W>{
        Edge { target: target.clone(), weight: weight.clone()}
    }
}

