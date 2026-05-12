
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

    /// Returns the target node identifier for this edge.
    ///
    /// # Returns
    /// A **copy** of the `target` field (`u64` is `Copy`).
    ///
    /// # Panics
    /// This method does not panic.
    pub fn get_target(&self) -> u64 {
        self.target
    }

    /// Returns the weight associated with this edge.
    ///
    /// # Returns
    /// A **clone** of the stored weight. The caller receives an owned copy;
    /// mutations to it will **not** affect the original edge.
    ///
    /// # Panics
    /// This method does not panic (assuming `W::clone()` does not panic).
    pub fn get_weight(&self) -> W{
        self.weight.clone()
    }

    /// Reinterprets the weight field as a raw byte slice for disk serialization.
    ///
    /// # Returns
    /// An **immutable reference** (`&[u8]`) into the weight's in-memory
    /// representation. The slice is valid for the lifetime of `self`.
    ///
    /// # Safety
    /// Uses `unsafe` pointer casting internally. This is sound only when `W`
    /// is a plain-old-data type with no padding bytes that carry meaning.
    ///
    /// # Panics
    /// This method does not panic.
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
    /// Constructs a new `EdgeView` by **cloning** the provided target and weight.
    ///
    /// The returned struct owns independent copies of both values;
    /// mutating them will **not** affect the original graph data.
    ///
    /// # Panics
    /// This method does not panic (assuming `K::clone()` and `W::clone()` do not panic).
    pub fn new(target: &K, weight: &W) -> EdgeView<K, W>{
        EdgeView { target: target.clone(), weight: weight.clone() }
    }
    /// Returns an **immutable reference** to the target node key.
    ///
    /// The reference borrows from `self`; no clone is performed.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn get_target(&self) -> &K{
        &self.target
    }
    /// Returns an **immutable reference** to the edge weight.
    ///
    /// The reference borrows from `self`; no clone is performed.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn get_weight(&self) -> &W{
        &self.weight
    }

}
