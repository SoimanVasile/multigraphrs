/// Represents a single connection between nodes in the graph.
///
/// An edge is always stored inside the adjacency list of a "source" node, 
/// so it only needs to keep track of its `target` and `weight`.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge<W>
where
    W: Clone + std::cmp::PartialEq,
{
    /// The destination node this edge points to.
    pub target: usize,
    /// The cost, distance, or metadata associated with this edge.
    pub weight: W,
}

impl<W> Edge<W>
where
    W: Clone + std::cmp::PartialEq,
{
    /// Constructs a new `Edge`.
    pub fn new(target: &usize, weight: &W) -> Edge<W> {
        Edge { target: target.clone(), weight: weight.clone()}
    }

    pub fn get_target(&self) -> usize {
        self.target
    }

    pub fn get_weight(&self) -> W{
        self.weight.clone()
    }
}

