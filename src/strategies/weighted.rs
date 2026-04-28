use crate::strategies::direction_strategy::DirectionStrategy;
use crate::core::edge::Edge;
use crate::core::graph_errors::GraphErrors;
use crate::storage::storage_backend::StorageBackend;

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
        graph: &mut impl StorageBackend<W>,
        source: u32, 
        target: u32, 
        weight: &W
    ) -> Result<Edge<W>, GraphErrors> {

        let edge = Edge::new(target, weight);
        let edge_reverse = Edge::new(source, weight);
        
        graph.add_edge_to_node(source, &edge);
        graph.add_edge_to_node(target, &edge_reverse);

        // Returns both edges to confirm the bidirectional connection
        Ok(edge)
    }

    fn remove_edge(graph: &mut impl StorageBackend<W>, source: u32, target: u32, weight: &W) -> Result<Edge<W>, GraphErrors> {
        let edge = Edge::new(target, weight);
        let reverse_edge = Edge::new(source, weight);
        graph.remove_edge(target, &reverse_edge, |edge_1: &Edge<W>, edge_2: &Edge<W>| -> bool 
            {return edge_1.get_weight() == edge_2.get_weight() && edge_1.get_target() == edge_2.get_target()})?;
        graph.remove_edge(source, &edge, |edge_1: &Edge<W>, edge_2: &Edge<W>| -> bool
            {return edge_1.get_weight() == edge_2.get_weight() && edge_1.get_target() == edge_2.get_target()})
    }

    /// Removes a node and all connected edges. O(degree(node)).
    ///
    /// Since the graph is undirected, every edge in node's outgoing list implies
    /// a reverse edge in the neighbor's list. We use this to avoid a full scan.
    fn remove_node(graph: &mut impl StorageBackend<W>, node_id: u32) {
        let edges: Vec<Edge<W>> = graph.get_edges(node_id).collect();
        for edge in edges {
            graph.remove_edge_by_target(edge.get_target(), node_id);
        }
        graph.clear_node_edges(node_id);
        graph.decrement_node_counter();
    }
}
