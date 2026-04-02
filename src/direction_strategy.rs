use std::collections::HashMap;
use crate::Edge;
use crate::adjacency_list::AdjacencyList;
use crate::graph_errors::GraphErrors;

/// A trait defining how edges are inserted into the graph's adjacency list.
///
/// By implementing this trait, different graph types (Directed, Undirected, etc.) 
/// can share the same core `MultiGraph` structure while maintaining unique behavior.
pub trait DirectionStrategy<W>
where
    W: Clone + std::cmp::PartialEq,
{
    /// Processes the raw source, target, and weight, mutating the `graph` directly.
    /// Returns the edges that were successfully created.
    fn add_edge(graph: &mut AdjacencyList<W>, source: &usize, target: &usize, weight: &W) -> Result<Edge<W>, GraphErrors>;

    fn remove_edge(graph: &mut AdjacencyList<W>, source: &usize, edge: &Edge<W> ) -> Result<Edge<W>, GraphErrors>;
}
