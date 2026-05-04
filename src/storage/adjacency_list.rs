use crate::core::edge::Edge;
use crate::core::graph_errors::GraphErrors;
use crate::storage::storage_backend::StorageBackend;

pub struct RamStorage<W>
where
    W: Clone + std::cmp::PartialEq,
{
    adjacency_list: Vec<Vec<Edge<W>>>,
    /// Tracks incoming edges: reverse_adjacency_list[node] = list of nodes that have edges TO this node.
    /// Used by directed strategies for O(degree) remove_node.
    reverse_adjacency_list: Vec<Vec<u64>>,
    number_of_nodes: usize,
    number_of_edges: usize,
}

impl<W> RamStorage<W>
where
    W: Clone + std::cmp::PartialEq,
{
    pub fn new() -> RamStorage<W>{
        RamStorage{
            adjacency_list: Vec::new(),
            reverse_adjacency_list: Vec::new(),
            number_of_nodes: 0,
            number_of_edges: 0,
        }
    }

    pub fn get_edges_ref(&self, source: u64) -> &Vec<Edge<W>>{
        &self.adjacency_list[source as usize]
    }
}

impl<W> StorageBackend<W> for RamStorage<W>
where
    W: Clone + std::cmp::PartialEq,
{
    type EdgeIter<'a> = std::vec::IntoIter<Edge<W>> where Self: 'a, W: 'a;

    fn add_edge_to_node(&mut self, node: u64, edge: &Edge<W>){
        self.number_of_edges+=1;
        self.adjacency_list[node as usize].push(edge.clone())
    }

    fn add_node(&mut self){
        self.number_of_nodes+=1;
        self.adjacency_list.push(Vec::new());
        self.reverse_adjacency_list.push(Vec::new());
    }

    fn node_len(&self, node: u64) -> usize{
        self.adjacency_list[node as usize].len()
    }

    fn get_edges<'a>(&self, node: u64) -> Self::EdgeIter<'a> where W: 'a{
        self.adjacency_list[node as usize].clone().into_iter()
    }

    fn remove_edge<F>(&mut self, source: u64, edge: &Edge<W>, func: F) -> Result<Edge<W>, GraphErrors>
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

    fn contains_edge(&self, source: u64, target: u64) ->Result<Edge<W>, GraphErrors>{
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

    // --- New primitives for strategy-driven remove_node ---

    fn clear_node_edges(&mut self, node: u64) {
        let count = self.adjacency_list[node as usize].len();
        self.number_of_edges -= count;
        self.adjacency_list[node as usize].clear();
    }

    fn remove_edge_by_target(&mut self, source: u64, target: u64) {
        let list = &mut self.adjacency_list[source as usize];
        if let Some(pos) = list.iter().position(|e| e.get_target() == target) {
            list.swap_remove(pos);
            self.number_of_edges -= 1;
        }
    }

    fn add_reverse_edge(&mut self, source: u64, origin: u64) {
        self.reverse_adjacency_list[source as usize].push(origin);
    }

    fn get_reverse_edges(&self, node: u64) -> Vec<u64> {
        self.reverse_adjacency_list[node as usize].clone()
    }

    fn clear_reverse_edges(&mut self, node: u64) {
        self.reverse_adjacency_list[node as usize].clear();
    }

    fn remove_reverse_edge(&mut self, source: u64, origin: u64) {
        let list = &mut self.reverse_adjacency_list[source as usize];
        if let Some(pos) = list.iter().position(|&id| id == origin) {
            list.swap_remove(pos);
        }
    }

    fn decrement_node_counter(&mut self) {
        self.number_of_nodes -= 1;
    }
}
