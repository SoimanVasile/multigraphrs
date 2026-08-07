use bytemuck::{Pod, Zeroable};

use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;

/// Represents an internal node ID including metadata for disk storage location and size.
#[repr(C)]
#[derive(Pod, Clone, Copy, Zeroable)]
pub struct NodeId{
    pub(crate) data_len: u64,
    pub(crate) data_offset: u64,
}

impl NodeId{
    /// Creates a new `NodeId` representing the location and size of node data on disk.
    ///
    /// # Arguments
    /// * `data_len` - The length of the node data in bytes.
    /// * `data_offset` - The offset in bytes where the node data is stored.
    pub fn new(data_len: u64, data_offset: u64) -> Self{
        Self{ data_len, data_offset}
    }

    /// Converts the `NodeId` into a byte slice for disk storage.
    ///
    /// # Safety
    /// This method uses `unsafe` to cast the struct pointer to a byte slice pointer, which is valid because `NodeId` implements `Pod` and `Zeroable`.
    pub fn convert_to_bytes(&self) -> &[u8]{
        unsafe{
            std::slice::from_raw_parts(
                self as *const NodeId as *const u8, 
                std::mem::size_of::<NodeId>())
        }
    }
}

impl FromDiskBytes for NodeId{
    /// Reconstructs a `NodeId` from a byte slice read from disk.
    ///
    /// # Arguments
    /// * `bytes` - The byte slice containing the serialized `NodeId`.
    fn from_bytes(bytes: &[u8]) -> Self {
        *bytemuck::from_bytes(bytes)
    }
}
