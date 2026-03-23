/// Represents a single connection between nodes in the graph.
///
/// An edge is always stored inside the adjacency list of a "source" node, 
/// so it only needs to keep track of its `target` and `weight`.
#[derive(Clone, Debug)]
pub struct Edge<W>
where
    W: Clone,
{
    /// The destination node this edge points to.
    pub target: usize,
    /// The cost, distance, or metadata associated with this edge.
    pub weight: W,
}

impl<W> Edge<W>
where
    W: Clone,
{
    /// Constructs a new `Edge`.
    pub fn new(target: &usize, weight: &W) -> Edge<W> {
        Edge { target: target.clone(), weight: weight.clone()}
    }
}
