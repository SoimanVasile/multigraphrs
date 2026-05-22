use bytemuck::Pod;
use bytemuck::Zeroable;

/// On-disk representation of a graph edge.
///
/// Stores the offset and length of the weight data in the data file,
/// and the target node identifier. This struct is `#[repr(C)]` and
/// implements `Pod` + `Zeroable` for direct memory-mapping.
#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct DiskEdge{
    /// Byte offset of the weight data in `data.bin`.
    pub weight_offset: u64,
    /// Length of the weight data in bytes.
    pub weight_len: u64,
    /// Target node identifier.
    pub node: u64,
}

impl DiskEdge{
    /// Constructs a new `DiskEdge`.
    ///
    /// All arguments are **copied** (all are `u64`, which is `Copy`).
    ///
    /// # Panics
    /// This method does not panic.
    pub fn new(weight_offset: u64, weight_len: u64, node: u64) -> DiskEdge{
        DiskEdge { weight_offset, weight_len, node}
    }

    /// Reinterprets this `DiskEdge` as a raw byte slice for disk serialization.
    ///
    /// # Returns
    /// An **immutable reference** (`&[u8]`) into the struct's in-memory layout.
    /// The slice is valid for the lifetime of `self`.
    ///
    /// # Safety
    /// Uses `unsafe` pointer casting. This is sound because `DiskEdge` is
    /// `#[repr(C)]` and derives `Pod`.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn convert_into_bytes(&self) -> &[u8]{
        unsafe{
            std::slice::from_raw_parts(
            self as *const DiskEdge as *const u8, 
            std::mem::size_of::<DiskEdge>()
                )
        }
    }
}

