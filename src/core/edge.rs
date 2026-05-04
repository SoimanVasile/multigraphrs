
/// Represents a single connection between nodes in the graph.
///
/// An edge is always stored inside the adjacency list of a "source" node, 
/// so it only needs to keep track of its `target` and `weight`.
#[derive(Clone, Debug, PartialEq)]
#[repr(C)]
pub struct Edge<W>
where
    W: Clone + std::cmp::PartialEq,
{
    /// The destination node this edge points to.
    pub target: u64,
    /// The cost, distance, or metadata associated with this edge.
    pub weight: W,
}

impl<W> Edge<W>
where
    W: Clone + std::cmp::PartialEq,
{
    /// Constructs a new `Edge`.
    pub fn new(target: u64, weight: &W) -> Edge<W> {
        Edge { target, weight: weight.clone()}
    }

    pub fn get_target(&self) -> u64 {
        self.target
    }

    pub fn get_weight(&self) -> W{
        self.weight.clone()
    }

    pub fn convert_to_bytes(&self) -> &[u8]{
        unsafe{
        std::slice::from_raw_parts(
            (&self.weight as *const W) as *const u8, 
            std::mem::size_of::<W>())
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeView<K, W>
where
    K: Clone + Eq,
    W: Clone + std::cmp::PartialEq
{
    target: K,
    weight: W,
}

impl<K, W> EdgeView<K, W>
where
    W: Clone + std::cmp::PartialEq,
    K: Eq + Clone
{
    pub fn new(target: &K, weight: &W) -> EdgeView<K, W>{
        EdgeView { target: target.clone(), weight: weight.clone() }
    }
    pub fn get_target(&self) -> &K{
        &self.target
    }
    pub fn get_weight(&self) -> &W{
        &self.weight
    }

}
