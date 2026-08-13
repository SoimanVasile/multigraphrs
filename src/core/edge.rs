use std::hash::Hash;

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
    /// Constructs a new `Edge` connecting to `target` with the given `weight`.
    ///
    /// The `target` is the node identifier, and `weight` is cloned and stored
    /// to represent the cost or data of the connection.
    pub fn new(target: u64, weight: &W) -> Edge<W> {
        Edge { target, weight: weight.clone()}
    }

    /// Returns the target node identifier for this edge.
    pub fn get_target(&self) -> u64 {
        self.target
    }

    /// Returns a clone of the weight associated with this edge.
    ///
    /// Mutations to the returned weight will not affect the original edge.
    pub fn get_weight(&self) -> W{
        self.weight.clone()
    }

    /// Reinterprets the weight field as a raw byte slice for disk serialization.
    ///
    /// The returned slice is valid for the lifetime of `self`.
    ///
    /// # Safety
    /// Uses `unsafe` pointer casting internally. This is sound only when `W`
    /// is a plain-old-data type with no padding bytes that carry meaning.
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
    K: Clone + Hash + Eq + AsDiskBytes + FromDiskBytes,
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes
{
    target: K,
    weight: W,
}

impl<K, W> EdgeView<K, W>
where
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    K: Eq + Clone + AsDiskBytes + FromDiskBytes + Hash
{
    /// Constructs a new `EdgeView` by cloning the provided `target` and `weight`.
    ///
    /// The returned struct owns independent copies of both values;
    /// mutating them will not affect the original graph data. `target` is the node key, and `weight` is the edge data.
    pub fn new(target: &K, weight: &W) -> EdgeView<K, W>{
        EdgeView { target: target.clone(), weight: weight.clone() }
    }
    /// Returns an immutable reference to the target node key.
    ///
    /// The reference borrows from `self`; no clone is performed.
    pub fn get_target(&self) -> &K{
        &self.target
    }
    /// Returns an immutable reference to the edge weight.
    ///
    /// The reference borrows from `self`; no clone is performed.
    pub fn get_weight(&self) -> &W{
        &self.weight
    }

}
