/// Represents a single connection between nodes in the graph.
///
/// An edge is always stored inside the adjacency list of a "source" node, 
/// so it only needs to keep track of its `target` and `weight`.
#[derive(Clone, Debug)]
pub struct Edge<K, W>
where
    K: Eq + std::hash::Hash + Clone,
    W: Clone,
{
    /// The destination node this edge points to.
    pub target: K,
    /// The cost, distance, or metadata associated with this edge.
    pub weight: W,
}

impl<K, W> Edge<K, W>
where
    K: Eq + std::hash::Hash + Clone,
    W: Clone,
{
    /// Constructs a new `Edge`.
    pub fn new(target: &K, weight: &W) -> Edge<K, W> {
        Edge { target: target.clone(), weight: weight.clone()}
    }
}
