use crate::core::edge::Edge;
use crate::core::graph_errors::GraphErrors;
use crate::storage::storage_backend::StorageBackend;
pub struct RamStorage<W>
where
    W: Clone + std::cmp::PartialEq,
{
    adjacency_list: Vec<Vec<Edge<W>>>,
    number_of_nodes: usize,
    number_of_edges: usize,
}

impl<W> RamStorage<W>
where
    W: Clone + std::cmp::PartialEq,
{
    pub fn new() -> RamStorage<W>{
        RamStorage{adjacency_list: Vec::new(), number_of_nodes: 0, number_of_edges: 0}
    }

    pub fn get_edges_ref(&self, source: u32) -> &Vec<Edge<W>>{
        &self.adjacency_list[source as usize]
    }
}

impl<W> StorageBackend<W> for RamStorage<W>
where
    W: Clone + std::cmp::PartialEq,
{
    type EdgeIter<'a> = std::vec::IntoIter<Edge<W>> where Self: 'a, W: 'a;

    fn add_edge_to_node(&mut self, node: u32, edge: &Edge<W>){
        self.number_of_edges+=1;
        self.adjacency_list[node as usize].push(edge.clone())
    }

    fn add_node(&mut self){
        self.number_of_nodes+=1;
        self.adjacency_list.push(Vec::new());
    }

    fn node_len(&self, node: u32) -> usize{
        self.adjacency_list[node as usize].len()
    }

    fn get_edges<'a>(&self, node: u32) -> Self::EdgeIter<'a> where W: 'a {
        self.adjacency_list[node as usize].clone().into_iter()
    }

    fn remove_edge<F>(&mut self, source: u32, edge: &Edge<W>, func: F) -> Result<Edge<W>, GraphErrors>
    where
        F: Fn(&Edge<W>, &Edge<W>) -> bool
    {
        let index = self.adjacency_list[source as usize]
            .iter()
            .position(|e| func(edge, e));
        if let Some(i) = index {
            self.number_of_edges-=1;
            Ok(self.adjacency_list[source as usize].swap_remove(i))
        } else {
            Err(GraphErrors::EdgeDoesntExists)
        }
    }

    fn remove_node(&mut self, target: u32) {
        self.number_of_nodes -=1;
        self.number_of_edges -=self.adjacency_list[target as usize].len();
        self.adjacency_list[target as usize].clear();
        
        for list in &mut self.adjacency_list{
            let initial_len = list.len();
            list.retain(
                |e| e.target != target
            );
            self.number_of_edges -= initial_len - list.len();
        }
    }

    fn contains_edge(&self, source: u32, target: u32) ->Result<Edge<W>, GraphErrors>{
        match self.adjacency_list[source as usize].iter().position(|e| e.get_target() == target) {
            Some(t) => Ok(self.adjacency_list[source as usize][t].clone()),
            None => Err(GraphErrors::EdgeDoesntExists),
        }
    }

    fn node_count(&self) -> usize {
        self.number_of_nodes
    }

    fn edge_count(&self) ->usize{
        self.number_of_edges
    }

    fn increment_node_counter(&mut self) {
        self.number_of_nodes+=1;
    }
}
