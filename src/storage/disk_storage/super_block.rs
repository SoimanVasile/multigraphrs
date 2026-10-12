use std::sync::atomic::{AtomicU64, Ordering};
use bytemuck::{Pod, Zeroable};

pub const SUPER_BLOCK_SIZE: usize = 1024;
pub const NUMBER_OF_BUCKETS: usize = 7;
pub const NUMBER_OF_WEIGHT_BUCKETS: usize = 3;

/// A plain-old-data (POD) representation of the [`SuperBlock`] for zero-copy
/// serialization and deserialization from disk.
///
/// Because types with interior mutability (`AtomicU64`) cannot safely implement
/// `bytemuck::Pod`, `SuperBlockRaw` serves as the serializable snapshot.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct SuperBlockRaw {
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

    /// Head of the free node linked list.
    pub head_linked_list_node: u64,

    /// Segregated free-list bucket headers for `structure.bin`.
    pub header_structure: [u64; NUMBER_OF_BUCKETS],
    /// Segregated free-list bucket headers for `reverse_structure.bin`.
    pub header_reverse_structure: [u64; NUMBER_OF_BUCKETS],
    /// Segregated free-list bucket headers for `data.bin`.
    pub header_weight_data: [u64; NUMBER_OF_WEIGHT_BUCKETS],

    /// Reserved padding to ensure the superblock is exactly 1024 bytes.
    pub _padding: [u8; 832],
}

unsafe impl Pod for SuperBlockRaw {}
unsafe impl Zeroable for SuperBlockRaw {}

// Compile-time assertion that SuperBlockRaw is exactly 1024 bytes
const _: () = assert!(std::mem::size_of::<SuperBlockRaw>() == SUPER_BLOCK_SIZE);

/// The in-memory metadata header stored at the beginning of `node.bin`.
///
/// Contains global counters (nodes, edges) and free-block pointers for
/// each of the data files. Provides thread-safe **interior mutability**
/// via atomic primitives (`AtomicU64`), allowing methods to mutate fields
/// concurrently through a shared reference (`&self`).
#[derive(Debug)]
#[repr(C)]
pub struct SuperBlock {
    /// Magic number identifying the file format (`"MGRF"` in little-endian).
    pub magic_number: u32,
    /// Format version (currently `1`).
    pub version: u32,

    /// Total number of live nodes.
    pub node_count: AtomicU64,
    /// Total number of edges across all nodes.
    pub edge_count: AtomicU64,

    /// Next free byte offset in `structure.bin`.
    pub next_structure_free_block: AtomicU64,
    /// Next free byte offset in `data.bin`.
    pub next_data_free_block: AtomicU64,
    /// Next free byte offset in `reverse_structure.bin`.
    pub next_reverse_structure_free_block: AtomicU64,

    /// Head of the free node linked list.
    pub head_linked_list_node: AtomicU64,

    /// Segregated free-list bucket headers for `structure.bin`.
    pub header_structure: [AtomicU64; NUMBER_OF_BUCKETS],
    /// Segregated free-list bucket headers for `reverse_structure.bin`.
    pub header_reverse_structure: [AtomicU64; NUMBER_OF_BUCKETS],
    /// Segregated free-list bucket headers for `data.bin`.
    pub header_weight_data: [AtomicU64; NUMBER_OF_WEIGHT_BUCKETS],

    /// Reserved padding to ensure the superblock is exactly 1024 bytes.
    pub _padding: [u8; 832],
}

// Compile-time assertion that SuperBlock has the exact same layout size
const _: () = assert!(std::mem::size_of::<SuperBlock>() == SUPER_BLOCK_SIZE);

impl Default for SuperBlock {
    /// Returns a default instance of `SuperBlock`.
    fn default() -> Self {
        Self::new()
    }
}

impl SuperBlock {
    /// Creates a new `SuperBlock` with the `"MGRF"` magic number, version `1`,
    /// counters set to 0, and all free lists initialized to `u64::MAX` (empty).
    pub fn new() -> Self {
        Self {
            magic_number: u32::from_le_bytes(*b"MGRF"),
            version: 1,
            node_count: AtomicU64::new(0),
            edge_count: AtomicU64::new(0),

            next_structure_free_block: AtomicU64::new(0),
            next_data_free_block: AtomicU64::new(0),
            next_reverse_structure_free_block: AtomicU64::new(0),

            head_linked_list_node: AtomicU64::new(u64::MAX),

            header_structure: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            header_reverse_structure: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            header_weight_data: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),

            _padding: [0; 832],
        }
    }

    /// Converts this `SuperBlock` to a raw snapshot for disk serialization.
    pub fn to_raw(&self) -> SuperBlockRaw {
        let mut header_structure = [0u64; NUMBER_OF_BUCKETS];
        for (i, h) in self.header_structure.iter().enumerate() {
            header_structure[i] = h.load(Ordering::Acquire);
        }

        let mut header_reverse_structure = [0u64; NUMBER_OF_BUCKETS];
        for (i, h) in self.header_reverse_structure.iter().enumerate() {
            header_reverse_structure[i] = h.load(Ordering::Acquire);
        }

        let mut header_weight_data = [0u64; NUMBER_OF_WEIGHT_BUCKETS];
        for (i, h) in self.header_weight_data.iter().enumerate() {
            header_weight_data[i] = h.load(Ordering::Acquire);
        }

        SuperBlockRaw {
            magic_number: self.magic_number,
            version: self.version,
            node_count: self.node_count.load(Ordering::Acquire),
            edge_count: self.edge_count.load(Ordering::Acquire),
            next_structure_free_block: self.next_structure_free_block.load(Ordering::Acquire),
            next_data_free_block: self.next_data_free_block.load(Ordering::Acquire),
            next_reverse_structure_free_block: self.next_reverse_structure_free_block.load(Ordering::Acquire),
            head_linked_list_node: self.head_linked_list_node.load(Ordering::Acquire),
            header_structure,
            header_reverse_structure,
            header_weight_data,
            _padding: self._padding,
        }
    }

    /// Reconstructs an in-memory `SuperBlock` from a raw snapshot.
    pub fn from_raw(raw: SuperBlockRaw) -> Self {
        Self {
            magic_number: raw.magic_number,
            version: raw.version,
            node_count: AtomicU64::new(raw.node_count),
            edge_count: AtomicU64::new(raw.edge_count),
            next_structure_free_block: AtomicU64::new(raw.next_structure_free_block),
            next_data_free_block: AtomicU64::new(raw.next_data_free_block),
            next_reverse_structure_free_block: AtomicU64::new(raw.next_reverse_structure_free_block),
            head_linked_list_node: AtomicU64::new(raw.head_linked_list_node),
            header_structure: std::array::from_fn(|i| AtomicU64::new(raw.header_structure[i])),
            header_reverse_structure: std::array::from_fn(|i| AtomicU64::new(raw.header_reverse_structure[i])),
            header_weight_data: std::array::from_fn(|i| AtomicU64::new(raw.header_weight_data[i])),
            _padding: raw._padding,
        }
    }

    /// Serializes this `SuperBlock` into an owned byte array of 1024 bytes.
    pub fn convert_to_bytes(&self) -> [u8; SUPER_BLOCK_SIZE] {
        bytemuck::cast(self.to_raw())
    }

    /// Deserializes a `SuperBlock` from a 1024-byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bytemuck::PodCastError> {
        let raw: &SuperBlockRaw = bytemuck::try_from_bytes(bytes)?;
        Ok(Self::from_raw(*raw))
    }

    // --- Node and Edge Counters ---

    /// Returns the current live node count.
    pub fn get_node_count(&self) -> u64 {
        self.node_count.load(Ordering::Acquire)
    }

    /// Sets the node count.
    pub fn set_node_count(&self, count: u64) {
        self.node_count.store(count, Ordering::Release);
    }

    /// Atomically increments the node count by one and returns the previous value.
    pub fn increment_node_counter(&self) -> u64 {
        self.node_count.fetch_add(1, Ordering::AcqRel)
    }

    pub fn sub_node_counter(&self, sub: u64){
        self.node_count.fetch_sub(sub, Ordering::AcqRel);
    }

    /// Atomically decrements the node count by one and returns the previous value.
    pub fn decrement_node_counter(&self) -> u64 {
        self.node_count.fetch_sub(1, Ordering::AcqRel)
    }

    /// Returns the current total edge count.
    pub fn get_edge_count(&self) -> u64 {
        self.edge_count.load(Ordering::Acquire)
    }

    /// Sets the edge count.
    pub fn set_edge_count(&self, count: u64) {
        self.edge_count.store(count, Ordering::Release);
    }

    /// Atomically increments the edge count by one and returns the previous value.
    pub fn increment_edge_counter(&self) -> u64 {
        self.edge_count.fetch_add(1, Ordering::AcqRel)
    }

    pub fn sub_edge_counter(&self, sub: u64){
        self.edge_count.fetch_sub(sub, Ordering::AcqRel);
    }

    /// Atomically decrements the edge count by `count` and returns the previous value.
    pub fn decrement_edge_counter(&self) -> u64 {
        self.edge_count.fetch_sub(1, Ordering::AcqRel)
    }

    // --- Bump Allocation ---

    /// Returns the next free byte offset in `structure.bin` and advances the pointer by `size`.
    pub fn next_free_block_structure(&self, size: u64) -> u64 {
        self.next_structure_free_block.fetch_add(size, Ordering::AcqRel)
    }

    /// Returns the next free byte offset in `reverse_structure.bin` and advances the pointer by `size`.
    pub fn next_free_block_reverse_structure(&self, size: u64) -> u64 {
        self.next_reverse_structure_free_block.fetch_add(size, Ordering::AcqRel)
    }

    /// Returns the current next free byte offset in `data.bin`.
    pub fn next_free_block_data(&self, size: u64) -> u64 {
        self.next_data_free_block.fetch_add(size, Ordering::AcqRel)
    }

    // --- Node Free List Header ---

    /// Returns the index of the next free node from the linked list (`u64::MAX` if empty).
    pub fn next_free_node(&self) -> u64 {
        self.head_linked_list_node.load(Ordering::Acquire)
    }

    /// Updates the head of the free node linked list.
    pub fn change_header(&self, next_id: u64) {
        self.head_linked_list_node.store(next_id, Ordering::Release);
    }

    // --- Segregated Free-List Headers: structure.bin ---

    /// Returns the `i`-th header value for `structure.bin`.
    pub fn get_ith_header_structure(&self, i: u64) -> u64 {
        self.header_structure[i as usize].load(Ordering::Acquire)
    }

    /// Sets the `i`-th header value for `structure.bin`.
    pub fn set_new_header_structure(&self, index: u64, next_id: u64) {
        self.header_structure[index as usize].store(next_id, Ordering::Release);
    }

    /// Compatibility wrapper for `set_new_header_structure`.
    pub fn next_header_structure(&self, index: &u64, next_id: &u64) {
        self.set_new_header_structure(*index, *next_id);
    }

    // --- Segregated Free-List Headers: reverse_structure.bin ---

    /// Returns the `i`-th header value for `reverse_structure.bin`.
    pub fn get_ith_header_reverse_structure(&self, i: u64) -> u64 {
        self.header_reverse_structure[i as usize].load(Ordering::Acquire)
    }

    /// Sets the `i`-th header value for `reverse_structure.bin`.
    pub fn set_new_header_reverse_structure(&self, index: u64, next_id: u64) {
        self.header_reverse_structure[index as usize].store(next_id, Ordering::Release);
    }

    /// Compatibility wrapper for `set_new_header_reverse_structure`.
    pub fn next_header_reverse_structure(&self, index: &u64, next_id: &u64) {
        self.set_new_header_reverse_structure(*index, *next_id);
    }

    // --- Segregated Free-List Headers: data.bin ---

    /// Returns the `i`-th header value for `data.bin`.
    pub fn get_ith_header_weight_data(&self, i: u64) -> u64 {
        self.header_weight_data[i as usize].load(Ordering::Acquire)
    }

    /// Sets the `i`-th header value for `data.bin`.
    pub fn set_new_header_weight_data(&self, index: u64, next_id: u64) {
        self.header_weight_data[index as usize].store(next_id, Ordering::Release);
    }

    /// Compatibility wrapper for `set_new_header_weight_data`.
    pub fn next_header_weight_data(&self, index: &u64, next_id: &u64) {
        self.set_new_header_weight_data(*index, *next_id);
    }
}
