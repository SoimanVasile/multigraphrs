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

    pub head_linked_list_node: u64,


    pub header_structure: [u64; 7],

    pub header_reverse_structure: [u64; 7],

    pub _padding: [u8; 856],
}

impl Default for SuperBlock{
    /// Returns a default instance of `SuperBlock`.
    ///
    /// # Errors
    /// None.
    ///
    fn default() -> Self{
        Self::new()
    }
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
    /// # Errors
    /// None.
    ///
    pub fn new() -> Self{
        Self{
            magic_number: u32::from_le_bytes(*b"MGRF"),
            version: 1,
            node_count: 0,
            edge_count: 0,

            next_structure_free_block: 0,
            next_data_free_block: 0,
            next_reverse_structure_free_block: 0,

            head_linked_list_node: u64::MAX,
            header_structure: [u64::MAX; 7],

            header_reverse_structure: [u64::MAX; 7],
            _padding: [0; 856],

        }
    }

    /// Returns the current node count.
    ///
    /// # Returns
    /// A **copy** of `node_count` (`u64` is `Copy`).
    ///
    /// # Errors
    /// None.
    ///
    pub fn get_node_count(&self) -> u64{
        self.node_count
    }

    /// Increments the node count by one.
    ///
    /// Mutates `self` in place.
    ///
    /// # Side Effects
    /// Modifies the `node_count` field of `self`.
    ///
    /// # Errors
    /// None.
    ///
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
    /// # Errors
    /// None.
    ///
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
    /// # Side Effects
    /// Advances `next_structure_free_block` by `size`.
    ///
    /// # Errors
    /// None.
    ///
    pub fn get_free_block_structure(&mut self, size: &u64) -> u64{
        self.next_structure_free_block += *size;

        self.next_structure_free_block - *size
    }

    /// Returns the next free byte offset in `reverse_structure.bin`.
    ///
    /// # Side Effects
    /// Advances `next_reverse_structure_free_block` by `size`.
    ///
    /// # Errors
    /// None.
    ///
    pub fn get_free_block_reverse_structure(&mut self, size: &u64) -> u64{
        self.next_reverse_structure_free_block += *size;

        self.next_reverse_structure_free_block - *size
    }

    /// Returns the next free byte offset in `data.bin`.
    ///
    /// # Returns
    /// A **copy** of `next_data_free_block` (`u64` is `Copy`).
    ///
    /// # Errors
    /// None.
    ///
    pub fn get_free_block_data(&self) -> u64{
        self.next_data_free_block
    }
    /// Advances the data free-block pointer by `size` bytes.
    ///
    /// Mutates `self` in place.
    ///
    /// # Side Effects
    /// Modifies the `next_data_free_block` field of `self`.
    ///
    /// # Errors
    /// None.
    ///
    pub fn find_next_data_free_block(&mut self, size: &u64){
        self.next_data_free_block += *size;
    }

    /// Returns the `i`-th header value for `reverse_structure.bin`.
    ///
    /// # Returns
    /// A **copy** of the requested header (`u64` is `Copy`).
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if `i` is out of bounds for the `header_reverse_structure` array.
    pub fn get_ith_header_reverse_structure(&self, i: &u64) -> u64{
        self.header_reverse_structure[*i as usize]
    }

    /// Returns the `i`-th header value for `structure.bin`.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if `i` is out of bounds for the `header_structure` array.
    pub fn get_ith_header_structure(&self, i: &u64) -> u64{
        self.header_structure[*i as usize]
    }

    /// Returns the index of the next free node from the linked list.
    ///
    /// # Errors
    /// None.
    ///
    pub fn next_free_node(&self) -> u64{
        self.head_linked_list_node
    }

    /// Updates the head of the free node linked list.
    ///
    /// # Side Effects
    /// Modifies `head_linked_list_node`.
    ///
    /// # Errors
    /// None.
    ///
    pub fn change_header(&mut self, next_id: &u64){
        self.head_linked_list_node = *next_id;
    }

    /// Sets the `i`-th header value for `structure.bin`.
    ///
    /// # Side Effects
    /// Modifies `header_structure`.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    pub fn next_header_structure(&mut self, index: &u64, next_id: &u64){
        self.header_structure[*index as usize] = *next_id;
    }

    /// Sets the `i`-th header value for `reverse_structure.bin`.
    ///
    /// # Side Effects
    /// Modifies `header_reverse_structure`.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    pub fn next_header_reverse_structure(&mut self, index: &u64, next_id: &u64){
        self.header_reverse_structure[*index as usize] = *next_id;
    }
        
}
