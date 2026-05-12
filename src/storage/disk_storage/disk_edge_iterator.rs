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
pub struct DiskEdgeIterator<'a, W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes,
{
    mmap_ref: &'a DiskStorage<W>,
    current_offset: u64,
    edges_left: u64,
}

impl<'a, W> DiskEdgeIterator<'a, W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes,
{
    /// Creates a new `DiskEdgeIterator` starting at the given offset.
    ///
    /// # Arguments
    /// * `mmap_ref` - **Immutable reference** to the `DiskStorage` (borrows for lifetime `'a`).
    /// * `offset` - Starting byte offset in `structure.bin` (**copied**, `u64` is `Copy`).
    /// * `number_of_edges` - Number of edges to iterate (**copied**).
    ///
    /// # Panics
    /// This method does not panic. Panics may occur later during iteration
    /// if the offset is invalid.
    pub fn new(mmap_ref: &'a DiskStorage<W>, offset: &u64, number_of_edges: &u64) -> DiskEdgeIterator<'a, W>{
        DiskEdgeIterator{mmap_ref, current_offset: offset.clone(), edges_left: number_of_edges.clone()}
    }
}
impl<'a, W> Iterator for DiskEdgeIterator<'a, W>
where
    W: Clone + PartialEq + FromDiskBytes{
    type Item=Edge<W>;

    /// Advances the iterator and returns the next `Edge<W>`.
    ///
    /// # Returns
    /// * `Some(Edge<W>)` — An **owned** edge with its weight deserialized from disk.
    /// * `None` — When all edges have been consumed.
    ///
    /// # Panics
    /// Panics if the current offset exceeds the structure or data memory map bounds.
    fn next(&mut self) -> Option<<Self as Iterator>::Item>{
        if self.edges_left == 0{
            return None;
        }
        
        let struct_bytes = &self.mmap_ref.mmap_structure[
            self.current_offset as usize .. self.current_offset as usize + size_of::<DiskEdge>()
        ];

        let disk_edge: &DiskEdge = bytemuck::from_bytes(struct_bytes);
        self.current_offset += size_of::<DiskEdge>() as u64;
        
        let weight_bytes: &[u8] = &self.mmap_ref.mmap_data[
            disk_edge.weight_offset as usize.. (disk_edge.weight_offset + disk_edge.weight_len) as usize];

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
pub struct DiskReverseEdgeIterator<'a, W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes,
{
    mmap_ref: &'a DiskStorage<W>,
    current_offset: u64,
    edges_left: u64,
}

impl<'a, W> DiskReverseEdgeIterator<'a, W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes,
{
    /// Creates a new `DiskReverseEdgeIterator` starting at the given offset.
    ///
    /// # Arguments
    /// * `mmap_ref` - **Immutable reference** to the `DiskStorage` (borrows for lifetime `'a`).
    /// * `offset` - Starting byte offset in `reverse_structure.bin` (**copied**).
    /// * `number_of_edges` - Number of reverse entries to iterate (**copied**).
    ///
    /// # Panics
    /// This method does not panic. Panics may occur later during iteration
    /// if the offset is invalid.
    pub fn new(mmap_ref: &'a DiskStorage<W>, offset: &u64, number_of_edges: &u64) -> DiskReverseEdgeIterator<'a, W>{
        DiskReverseEdgeIterator{mmap_ref, current_offset: offset.clone(), edges_left: number_of_edges.clone()}
    }
}
impl<'a, W> Iterator for DiskReverseEdgeIterator<'a, W>
where
    W: Clone + PartialEq + FromDiskBytes{
    type Item=u64;

    /// Advances the iterator and returns the next reverse edge node ID.
    ///
    /// # Returns
    /// * `Some(u64)` — The node ID (**copy**, `u64` is `Copy`).
    /// * `None` — When all entries have been consumed.
    ///
    /// # Panics
    /// * Panics if the current offset exceeds the reverse structure memory map bounds.
    /// * Panics (via `unwrap`) if the byte slice cannot be converted to a `[u8; 8]` array.
    fn next(&mut self) -> Option<<Self as Iterator>::Item>{
        if self.edges_left == 0{
            return None;
        }
        
        let struct_bytes = &self.mmap_ref.mmap_reverse_structure[
            self.current_offset as usize .. self.current_offset as usize + size_of::<u64>()
        ];

        let node: u64 = u64::from_le_bytes(struct_bytes.try_into().unwrap());
        self.current_offset += size_of::<u64>() as u64;
        
        self.edges_left-=1;

        Some(node)
    }
}

