use std::hash::Hash;

use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;
use crate::storage::disk_storage::disk_edge::DiskEdge;
use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::disk_multigraph::DiskStorage;
use crate::Edge;

/// Iterator over a node's forward edges on disk.
///
/// Reads [`DiskEdge`] records sequentially from the structure memory map
/// and reconstructs full `Edge<W>` values by fetching weight data from
/// the data memory map.
///
/// Each call to [`next()`](Iterator::next) returns an **owned** `Edge<W>`
/// (weight is deserialized via [`FromDiskBytes`]).
#[derive(Clone, Debug)]
pub struct DiskEdgeIterator<'a, K, W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes + AsDiskBytes,
    K: Clone + Hash + Eq + AsDiskBytes + FromDiskBytes,
{
    mmap_ref: &'a DiskStorage<K, W>,
    current_offset: u64,
    edges_left: u64,
}

impl<'a, K, W> DiskEdgeIterator<'a, K, W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes + AsDiskBytes,
    K: Clone + Eq + Hash + FromDiskBytes + AsDiskBytes,
{
    /// Creates a new `DiskEdgeIterator` starting at the given offset.
    ///
    /// # Arguments
    /// * `mmap_ref` - Provides access to the memory-mapped files needed for reading structure and weight data.
    /// * `offset` - Determines where in `structure.bin` the iteration should begin.
    /// * `number_of_edges` - Specifies the exact number of edges to read before iteration stops.
    pub fn new(mmap_ref: &'a DiskStorage<K, W>, offset: &u64, number_of_edges: &u64) -> DiskEdgeIterator<'a, K, W>{
        DiskEdgeIterator{mmap_ref, current_offset: *offset, edges_left: *number_of_edges}
    }
}
impl<'a, K, W> Iterator for DiskEdgeIterator<'a, K, W>
where
    W: Clone + PartialEq + FromDiskBytes + AsDiskBytes,
    K: Clone + Eq + Hash + FromDiskBytes + AsDiskBytes,
{
    type Item=Edge<W>;

    /// Advances the iterator and returns the next `Edge<W>`. Returns `None` when all edges have been consumed.
    ///
    /// # Panics
    /// Panics if the current offset exceeds the structure or data memory map bounds.
    fn next(&mut self) -> Option<<Self as Iterator>::Item>{
        if self.edges_left == 0{
            return None;
        }
        
        let struct_bytes = &self.mmap_ref.file_manager_edge_structure.reading_bytes(self.current_offset,self.current_offset + size_of::<DiskEdge>() as u64);

        let disk_edge: &DiskEdge = bytemuck::from_bytes(struct_bytes);
        self.current_offset += size_of::<DiskEdge>() as u64;
        
        let weight_bytes: &[u8] = self.mmap_ref.file_manager_weight_data.reading_bytes(disk_edge.weight_offset, disk_edge.weight_offset + disk_edge.weight_len);

        let weight: W = FromDiskBytes::from_bytes(weight_bytes);

        self.edges_left-=1;

        Some(Edge::new(disk_edge.node, &weight))
    }
}


/// Iterator over a node's reverse edge entries on disk.
///
/// Reads `u64` node IDs sequentially from the reverse structure memory map.
/// Each call to [`next()`](Iterator::next) returns an **owned** `u64`
/// (`u64` is `Copy`).
pub struct DiskReverseEdgeIterator<'a, K, W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes + AsDiskBytes,
    K: Clone + Eq + Hash + FromDiskBytes + AsDiskBytes,
{
    mmap_ref: &'a DiskStorage<K, W>,
    current_offset: u64,
    edges_left: u64,
}

impl<'a, K, W> DiskReverseEdgeIterator<'a, K, W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes + AsDiskBytes,
    K: Clone + Eq + Hash + FromDiskBytes + AsDiskBytes,
{
    /// Creates a new `DiskReverseEdgeIterator` starting at the given offset.
    ///
    /// # Arguments
    /// * `mmap_ref` - Provides access to the memory-mapped files needed for reading reverse structure data.
    /// * `offset` - Determines where in `reverse_structure.bin` the iteration should begin.
    /// * `number_of_edges` - Specifies the exact number of reverse entries to read before iteration stops.
    pub fn new(mmap_ref: &'a DiskStorage<K, W>, offset: &u64, number_of_edges: &u64) -> DiskReverseEdgeIterator<'a, K, W>{
        DiskReverseEdgeIterator{mmap_ref, current_offset: *offset, edges_left: *number_of_edges}
    }
}
impl<'a, K, W> Iterator for DiskReverseEdgeIterator<'a, K, W>
where
    W: Clone + PartialEq + FromDiskBytes + AsDiskBytes,
    K: Clone + Eq + Hash + FromDiskBytes + AsDiskBytes,
{
    type Item=u64;

    /// Advances the iterator and returns the next reverse edge node ID. Returns `None` when all entries have been consumed.
    ///
    /// # Panics
    /// * Panics if the current offset exceeds the reverse structure memory map bounds.
    /// * Panics (via `unwrap`) if the byte slice cannot be converted to a `[u8; 8]` array.
    fn next(&mut self) -> Option<<Self as Iterator>::Item>{
        if self.edges_left == 0{
            return None;
        }
        
        let struct_bytes = self.mmap_ref.file_manager_reverse_edge.reading_bytes(self.current_offset, self.current_offset + size_of::<u64>() as u64);

        let node: u64 = u64::from_le_bytes(struct_bytes.try_into().unwrap());
        self.current_offset += size_of::<u64>() as u64;
        
        self.edges_left-=1;

        Some(node)
    }
}

