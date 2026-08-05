use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;

/// Represents a single connection between nodes in the graph.
///
/// An edge is always stored inside the adjacency list of a "source" node, 
/// so it only needs to keep track of its `target` and `weight`.
#[derive(Clone, Debug, PartialEq)]
#[repr(C)]
pub struct Edge<W>
where
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
{
    /// The destination node this edge points to.
    pub target: u64,
    /// The cost, distance, or metadata associated with this edge.
    pub weight: W,
}

impl<W> Edge<W>
where
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
{
    /// Constructs a new `Edge`.
    ///
    /// # Errors
    /// This function does not return an error.
    pub fn new(target: u64, weight: &W) -> Edge<W> {
        Edge { target, weight: weight.clone()}
    }

    /// Returns the target node identifier for this edge.
    ///
    /// # Returns
    /// A **copy** of the `target` field (`u64` is `Copy`).
    ///
    /// # Errors
    /// This function does not return an error.
    pub fn get_target(&self) -> u64 {
        self.target
    }

    /// Returns the weight associated with this edge.
    ///
    /// # Returns
    /// A **clone** of the stored weight. The caller receives an owned copy;
    /// mutations to it will **not** affect the original edge.
    ///
    /// # Errors
    /// This function does not return an error.
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
    /// # Errors
    /// This function does not return an error.
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
    K: Clone + Eq + AsDiskBytes + FromDiskBytes,
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes
{
    target: K,
    weight: W,
}

impl<K, W> EdgeView<K, W>
where
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    K: Eq + Clone + AsDiskBytes + FromDiskBytes
{
    /// Constructs a new `EdgeView` by **cloning** the provided target and weight.
    ///
    /// The returned struct owns independent copies of both values;
    /// mutating them will **not** affect the original graph data.
    ///
    /// # Errors
    /// This function does not return an error.
    pub fn new(target: &K, weight: &W) -> EdgeView<K, W>{
        EdgeView { target: target.clone(), weight: weight.clone() }
    }
    /// Returns an **immutable reference** to the target node key.
    ///
    /// The reference borrows from `self`; no clone is performed.
    ///
    /// # Errors
    /// This function does not return an error.
    pub fn get_target(&self) -> &K{
        &self.target
    }
    /// Returns an **immutable reference** to the edge weight.
    ///
    /// The reference borrows from `self`; no clone is performed.
    ///
    /// # Errors
    /// This function does not return an error.
    pub fn get_weight(&self) -> &W{
        &self.weight
    }

}
