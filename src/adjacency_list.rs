
use crate::{Edge, adjacency_list};
pub struct AdjacencyList<W>
where
    W: Clone
{
    adjacency_list: Vec<Vec<Edge<W>>>,
}

impl<W> AdjacencyList<W>
where
    W: Clone
{
    pub fn new() -> AdjacencyList<W>{
        AdjacencyList{adjacency_list: Vec::new() }
    }

    pub fn add_edge_to_node(&mut self, node: &usize, edge: &Edge<W>){
        self.adjacency_list[*node].push(edge.clone())
    }

    pub fn len(&self) -> usize{
        self.adjacency_list.len()
    }

    pub fn add_node(&mut self){
        self.adjacency_list.push(Vec::new());
    }

    pub fn node_len(&mut self, node: usize) -> usize{
        self.adjacency_list[node].len()
    }
}
