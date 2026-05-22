use bytemuck::Pod;
use bytemuck::Zeroable;

/// The metadata header stored at the beginning of `node.bin`.
///
/// Contains global counters (nodes, edges) and free-block pointers for
/// each of the three data files. Padded to exactly 1024 bytes so that
/// node records start at a fixed, aligned offset.
///
/// This struct is `#[repr(C)]` and implements `Pod` + `Zeroable`
/// for zero-copy serialization.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct SuperBlock{
    /// Magic number identifying the file format (`"MGRF"` in little-endian).
    pub magic_number: u32,
    /// Format version (currently `1`).
    pub version: u32,

    /// Total number of live nodes.
    pub node_count: u64,
    /// Total number of edges across all nodes.
    pub edge_count: u64,

    /// Next free byte offset in `structure.bin`.
    pub next_structure_free_block: u64,
    /// Next free byte offset in `data.bin`.
    pub next_data_free_block: u64,
    /// Next free byte offset in `reverse_structure.bin`.
    pub next_reverse_structure_free_block: u64,

    pub _padding: [u8; 976],
}

unsafe impl Pod for SuperBlock{}
unsafe impl Zeroable for SuperBlock {}

impl SuperBlock {
    /// Creates a new `SuperBlock` with the `"MGRF"` magic number, version `1`,
    /// and all counters and pointers initialized to zero.
    ///
    /// # Returns
    /// An **owned** `SuperBlock`.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn new() -> Self{
        Self{
            magic_number: u32::from_le_bytes(*b"MGRF"),
            version: 1,
            node_count: 0,
            edge_count: 0,

            next_structure_free_block: 0,
            next_data_free_block: 0,
            next_reverse_structure_free_block: 0,

            _padding: [0; 976],

        }
    }

    /// Returns the current node count.
    ///
    /// # Returns
    /// A **copy** of `node_count` (`u64` is `Copy`).
    ///
    /// # Panics
    /// This method does not panic.
    pub fn get_node_count(&self) -> u64{
        self.node_count
    }

    /// Increments the node count by one.
    ///
    /// Mutates `self` in place.
    ///
    /// # Panics
    /// This method does not panic (may overflow on `u64::MAX`, which is unreachable in practice).
    pub fn increment_node_counter(&mut self){
        self.node_count+=1;
    }

    /// Reinterprets this `SuperBlock` as a raw byte slice for disk serialization.
    ///
    /// # Returns
    /// An **immutable reference** (`&[u8]`) into the struct's memory layout.
    ///
    /// # Safety
    /// Uses `unsafe` pointer casting. Sound because `SuperBlock` is `#[repr(C)]` and `Pod`.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn convert_to_bytes(&self) -> &[u8]{
        unsafe{
            std::slice::from_raw_parts(
                self as *const SuperBlock as *const u8,
                std::mem::size_of::<SuperBlock>()
            )
        }
    }

    /// Returns the next free byte offset in `structure.bin`.
    ///
    /// # Returns
    /// A **copy** of `next_structure_free_block` (`u64` is `Copy`).
    ///
    /// # Panics
    /// This method does not panic.
    pub fn get_free_block_structure(&self) -> u64{
        self.next_structure_free_block
    }

    /// Advances the structure free-block pointer by `size` bytes.
    ///
    /// Mutates `self` in place.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn find_next_strcture_free_block(&mut self, size: &u64){
        self.next_structure_free_block += *size;
    }

    /// Returns the next free byte offset in `data.bin`.
    ///
    /// # Returns
    /// A **copy** of `next_data_free_block` (`u64` is `Copy`).
    ///
    /// # Panics
    /// This method does not panic.
    pub fn get_free_block_data(&self) -> u64{
        self.next_data_free_block
    }
    /// Advances the data free-block pointer by `size` bytes.
    ///
    /// Mutates `self` in place.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn find_next_data_free_block(&mut self, size: &u64){
        self.next_data_free_block += *size;
    }

    /// Returns the next free byte offset in `reverse_structure.bin`.
    ///
    /// # Returns
    /// A **copy** of `next_reverse_structure_free_block` (`u64` is `Copy`).
    ///
    /// # Panics
    /// This method does not panic.
    pub fn get_free_block_reverse_structure(&self) -> u64{
        self.next_reverse_structure_free_block
    }

    /// Advances the reverse-structure free-block pointer by `size` bytes.
    ///
    /// Mutates `self` in place.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn find_next_reverse_structure_free_block(&mut self, size: &u64){
        self.next_reverse_structure_free_block += *size;
    }
}
