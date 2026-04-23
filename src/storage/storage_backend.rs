use crate::core::edge::Edge;
use crate::core::graph_errors::GraphErrors;

pub trait StorageBackend<W>
where
    W: Clone + std::cmp::PartialEq,
{
    type EdgeIter<'a>: Iterator<Item=Edge<W>> where Self: 'a, W: 'a;
    fn add_edge_to_node(&mut self, node: u32, edge: &Edge<W>);
    fn add_node(&mut self);
    fn node_len(&self, node: u32) -> usize;
    fn get_edges<'a>(&self, node: u32) -> Self::EdgeIter<'a> where W: 'a;
    fn remove_edge<F>(&mut self, source: u32, edge: &Edge<W>, func: F) -> Result<Edge<W>, GraphErrors>
    where
        F: Fn(&Edge<W>, &Edge<W>) -> bool;
    fn remove_node(&mut self, target: u32);
    fn contains_edge(&self, source: u32, target: u32) -> Result<Edge<W>, GraphErrors>;
    fn node_count(&self) -> usize;
    fn edge_count(&self) -> usize;
    fn increment_node_counter(&mut self);
}
