use bytemuck::Pod;
use bytemuck::Zeroable;

/// On-disk representation of a graph node.
///
/// Stores the node's index, pointers to its forward and reverse edge blocks,
/// and the counts for each. This struct is `#[repr(C)]` and implements
/// `Pod` + `Zeroable` for direct memory-mapping.
#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct DiskNode{
    /// Zero-based node index used for position calculations.
    pub node_idx: u64,
    /// Byte offset of the forward edge block in `structure.bin` (`u64::MAX` = uninitialized).
    pub list_edges_offset: u64,
    /// Number of forward edges stored.
    pub number_of_edges: u64,
    /// Byte offset of the reverse edge block in `reverse_structure.bin` (`u64::MAX` = uninitialized).
    pub list_reverse_edges_offset: u64,
    /// Number of reverse edge entries stored.
    pub number_of_reverse_edges: u64,
}

impl DiskNode{
    /// Constructs a new `DiskNode` with zero edges.
    ///
    /// All arguments are **copied** (`u64` is `Copy`). Pass `u64::MAX` for
    /// the offset parameters to indicate an uninitialized node.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn new(node_idx: u64, list_edges_offset: u64,list_reverse_edges_offset: u64) -> Self{
        Self { node_idx, list_edges_offset, number_of_edges: 0, list_reverse_edges_offset, number_of_reverse_edges: 0,}   
    }
    /// Returns the byte offset of this node's forward edge block.
    ///
    /// # Returns
    /// A **copy** of `list_edges_offset` (`u64` is `Copy`).
    ///
    /// # Panics
    /// This method does not panic.
    pub fn get_edge_offset(&self) -> u64{
        self.list_edges_offset
    }

    /// Returns the number of forward edges for this node.
    ///
    /// # Returns
    /// A **copy** of `number_of_edges` (`u64` is `Copy`).
    ///
    /// # Panics
    /// This method does not panic.
    pub fn get_number_of_edges(&self) -> u64{
        self.number_of_edges
    }

    /// Reinterprets this `DiskNode` as a raw byte slice for disk serialization.
    ///
    /// # Returns
    /// An **immutable reference** (`&[u8]`) into the struct's in-memory layout.
    /// The slice is valid for the lifetime of `self`.
    ///
    /// # Safety
    /// Uses `unsafe` pointer casting. This is sound because `DiskNode` is
    /// `#[repr(C)]` and derives `Pod`.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn convert_to_bytes(&self) -> &[u8]{
        unsafe{
            std::slice::from_raw_parts(self as *const DiskNode as *const u8, std::mem::size_of::<DiskNode>())
        }
    }
}


