use crate::DirectionStrategy;
use crate::Edge;
use crate::GraphErrors;
use crate::adjacency_list::AdjacencyList;

/// A strategy for weighted, undirected graphs.
///
/// In a weighted, undirected graph, an edge represents a two-way connection 
/// between nodes, where the connection has an associated cost, distance, or metadata 
/// (the `weight`). 
///
/// Adding an edge between node A and node B will create two internal edge entries:
/// one from A to B, and one from B to A, both sharing the exact same cloned weight.
#[derive(Debug)]
pub struct Weighted;

impl<W> DirectionStrategy<W> for Weighted
where
    W: Clone + std::cmp::PartialEq,
{
    /// Adds two edges (source -> target and target -> source) with the specified `weight`.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if the `source` or `target` node 
    /// is missing from the graph's adjacency list.
    fn add_edge(
        graph: &mut AdjacencyList<W>,
        source: &usize, 
        target: &usize, 
        weight: &W
    ) -> Result<Edge<W>, GraphErrors> {

        let edge = Edge::new(target, weight);
        let edge_reverse = Edge::new(source, weight);
        
        graph.add_edge_to_node(source, &edge);
        graph.add_edge_to_node(target, &edge_reverse);

        // Returns both edges to confirm the bidirectional connection
        Ok(edge)
    }

    fn remove_edge(graph: &mut AdjacencyList<W>, source: &usize, target: &usize, weight: &W) -> Result<Edge<W>, GraphErrors> {
        let edge = Edge::new(target, weight);
        let reverse_edge = Edge::new(source, weight);
        graph.remove_edge(target, &reverse_edge, |edge_1: &Edge<W>, edge_2: &Edge<W>| -> bool 
            {return edge_1.get_weight() == edge_2.get_weight() && edge_1.get_target() == edge_2.get_target()})?;
        graph.remove_edge(source, &edge, |edge_1: &Edge<W>, edge_2: &Edge<W>| -> bool
            {return edge_1.get_weight() == edge_2.get_weight() && edge_1.get_target() == edge_2.get_target()})
    }
}
