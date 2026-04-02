use crate::{Edge, GraphErrors};
pub struct AdjacencyList<W>
where
    W: Clone + std::cmp::PartialEq,
{
    adjacency_list: Vec<Vec<Edge<W>>>,
}

impl<W> AdjacencyList<W>
where
    W: Clone + std::cmp::PartialEq,
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

    pub fn node_len(&self, node: &usize) -> usize{
        self.adjacency_list[*node].len()
    }
    pub fn iter_node(&self, node: &usize) -> std::slice::Iter<'_, Edge<W>>{
        return self.adjacency_list[*node].iter()
    }

    pub fn get_edges(&self, node: &usize) -> Vec<Edge<W>>{
        self.adjacency_list[*node].clone()
    }

    pub fn remove_edge<F>(&mut self, source: &usize, edge: &Edge<W>, func: F) -> Result<Edge<W>, GraphErrors>
    where
        F: Fn(&Edge<W>, &Edge<W>) -> bool
    {
        let index = self.adjacency_list[*source]
            .iter()
            .position(|e| func(edge, e));
        if let Some(i) = index {
            Ok(self.adjacency_list[*source].swap_remove(i))
        } else {
            Err(GraphErrors::EdgeDoesntExists)
        }
    }
}
