use crate::adjacency_list::AdjacencyList;
use crate::direction_strategy::DirectionStrategy;
use crate::graph_errors::GraphErrors;
use crate::Edge;

/// A strategy for unweighted, directed graphs.
///
/// In a directed graph, an edge from node A to node B does not imply 
/// a connection from node B back to node A. 
/// 
/// Because this is an "unweighted" strategy, the `MultiGraph` will automatically 
/// assign a default weight of `1` (as a `u32`) to every edge created.
pub struct Directed;

impl DirectionStrategy<u32> for Directed
{
    /// Adds a single directed edge from `source` to `target` with a weight of `1`.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if the `source` or `target` node 
    /// is missing from the graph's adjacency list.
    fn add_edge(
        graph: &mut AdjacencyList<u32>,
        source: &usize, 
        target: &usize, 
        weight: &u32
    ) -> Result<Edge<u32>, GraphErrors> {

        let edge = Edge::new(target, weight);
        graph.add_edge_to_node(source, &edge);
        
        // Returns the single edge that was created
        Ok(edge)
    }

    fn remove_edge(graph: &mut AdjacencyList<u32>, source: &usize, edge: &Edge<u32> ) -> Result<Edge<u32>, GraphErrors> {
        return graph.remove_edge(source, edge, |edge_1: &Edge<u32>, edge_2: &Edge<u32>| -> bool {
            return edge_1.get_target() == edge_2.get_target();
        })
    }
}
