use crate::{Edge, GraphErrors};
pub struct AdjacencyList<W>
where
    W: Clone + std::cmp::PartialEq,
{
    adjacency_list: Vec<Vec<Edge<W>>>,
    number_of_nodes: usize,
    number_of_edges: usize,
}

impl<W> AdjacencyList<W>
where
    W: Clone + std::cmp::PartialEq,
{
    pub fn new() -> AdjacencyList<W>{
        AdjacencyList{adjacency_list: Vec::new(), number_of_nodes: 0, number_of_edges: 0}
    }

    pub fn add_edge_to_node(&mut self, node: &usize, edge: &Edge<W>){
        self.number_of_edges+=1;
        self.adjacency_list[*node].push(edge.clone())
    }

    pub fn add_node(&mut self){
        self.number_of_nodes+=1;
        self.adjacency_list.push(Vec::new());
    }

    pub fn node_len(&self, node: &usize) -> usize{
        self.adjacency_list[*node].len()
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
            self.number_of_edges-=1;
            Ok(self.adjacency_list[*source].swap_remove(i))
        } else {
            Err(GraphErrors::EdgeDoesntExists)
        }
    }

    pub fn remove_node(&mut self, target: &usize) {
        self.number_of_nodes -=1;
        self.number_of_edges -=self.adjacency_list[*target].len();
        self.adjacency_list[*target].clear();
        
        for list in &mut self.adjacency_list{
            let initial_len = list.len();
            list.retain(
                |e| e.target != *target
            );
            self.number_of_edges -= initial_len - list.len();
        }
    }

    pub fn contains_edge(&self, source: &usize, target: &usize) ->Result<Edge<W>, GraphErrors>{
        match self.adjacency_list[*source].iter().position(|e| e.get_target() == *target) {
            Some(t) => Ok(self.adjacency_list[*source][t].clone()),
            None => Err(GraphErrors::EdgeDoesntExists),
        }
    }

    pub fn node_count(&self) -> usize {
        self.number_of_nodes
    }

    pub fn edge_count(&self) ->usize{
        self.number_of_edges
    }

    pub fn get_edges_ref(&self, source: &usize) -> &Vec<Edge<W>>{
        &self.adjacency_list[*source]
    }

    pub fn increment_node_counter(&mut self) {
        self.number_of_nodes+=1;
    }
}
