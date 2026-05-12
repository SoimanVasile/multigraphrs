use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;
use memmap2::MmapOptions;
use std::fs::OpenOptions;

use crate::GraphErrors;
use crate::storage::disk_storage::disk_edge_iterator::DiskEdgeIterator;
use crate::storage::disk_storage::disk_edge_iterator::DiskReverseEdgeIterator;
use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::super_block::SuperBlock;
use crate::storage::disk_storage::disk_edge::DiskEdge;
use crate::storage::disk_storage::disk_node::DiskNode;
use crate::StorageBackend;
use crate::Edge;

const SUPER_BLOCK_SIZE: usize = 1024;
const DISK_NODE_INITIAL_CAPACITY: usize = 1024;
const FILE_INITIAL_SIZE: u64 = 1024 * 1024;

/// Fill the range [start,end) with the bytes given in the memory map
///
/// This is a convenience wrapper around [`slice::copy_from_slice`]
/// 
/// # Arguments
/// * `mmap` - The mutable memory map to modify
/// * `start` - The starting byte  offset (inclusive)
/// * `end` - The end byte offset (exclusive)
/// * `bytes` - The raw data to be written
///
/// # Panics
/// Panics if `start > end` or if `ends` exceeds the actual length of the memory map or if the length
/// of the bytes is different from the range
pub fn writing_bytes_to_mmap(mmap: &mut memmap2::MmapMut, start: u64, end: u64,  bytes: &[u8]){
    mmap[start as usize .. end as usize].copy_from_slice(bytes);
}

/// Fill the range [start,end) with zeros in the memory map
///
/// This is a convenience wrapper around [`slice::fill`]
/// 
/// # Arguments
/// * `mmap` - The mutable memory map to modify
/// * `start` - The starting byte  offset (inclusive)
/// * `end` - The end byte offset (exclusive)
///
/// # Panics
/// Panics if `start > end` or if `ends` exceeds the actual length of the memory map.
pub fn zeroing_mmap(mmap: &mut memmap2::MmapMut, start: u64, end: u64){
    mmap[start as usize .. end as usize].fill(0);
}

/// A persistent, disk-backed storage engine for `MultiGraphRs`.
///
/// `DiskStorage` utilizes a segmented file architecture to manage graph data with 
/// constant-time random access and memory-mapped performance. It maintains four 
/// distinct binary files:
/// * `node.bin`: Stores the [`SuperBlock`] and a contiguous array of [`DiskNode`] records.
/// * `structure.bin`: Manages adjacency lists (edges) using a block-allocation strategy.
/// * `reverse_structure.bin`: Stores incoming edge indices for optimized node removal.
/// * `data.bin`: A variable-length heap for storing edge weights (`W`).
///
/// All files are memory-mapped to the process's virtual address space, allowing 
/// the Operating System to handle caching and persistence asynchronously.
#[derive(Debug)]
pub struct DiskStorage<W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes
{
    
    //This file stores all the DiskEdges
    pub(crate) file_structure: std::fs::File,
    //This file stores all the weight data for all the edges
    pub(crate) file_data: std::fs::File,
    //This file stores all the DiskNodes
    pub(crate) file_node: std::fs::File,
    //This file stores the nodes that point to that respective target so to remove a node is
    //O(degree(target)) instead of iterating all over the graph
    pub(crate) file_reverse_structure: std::fs::File,

    //All the memory maps to their respective files
    pub(crate) mmap_structure: memmap2::MmapMut,
    pub(crate) mmap_data: memmap2::MmapMut,
    pub(crate) mmap_node: memmap2::MmapMut,
    pub(crate) mmap_reverse_structure: memmap2::MmapMut,
    _marker: PhantomData<W>,
}


impl<W> DiskStorage<W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes,
{
    /// Allocates a new [`DiskStorage`] in the specified directory.
    ///
    /// This constructor handles the full initialization of the database storage:
    /// 1. It creates the directory and any missing parent directories.
    /// 2. It opens (or creates) the four required backing files: `structure.bin`, 
    ///    `data.bin`, `node.bin`, and `reverse_structure.bin`.
    /// 3. It initializes new files to [`FILE_INITIAL_SIZE`].
    /// 4. It establishes memory maps for all files.
    ///
    /// # Arguments
    /// * `directory` - The path where the storage files will be managed.
    ///
    /// # Panics
    /// This function will panic if:
    /// * The directory cannot be created due to permission or path errors.
    /// * Any of the required `.bin` files cannot be opened or created.
    /// * The filesystem fails to report file metadata or set the initial file length.
    /// * Memory mapping the files fails (e.g., out of virtual address space).
    pub fn new<P: AsRef<Path>>(directory: P) -> DiskStorage<W>
    {
        let dir = directory.as_ref();

        std::fs::create_dir_all(dir)
            .expect("Failed to create the storage directory!");

        let structure_path = dir.join("structure.bin");
        let data_path = dir.join("data.bin");
        let node_path = dir.join("node.bin");
        let reverse_structure_path = dir.join("reverse_structure.bin");

        let mut file_structure = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(structure_path)
            .expect("Failed to open the structure file!");

        let mut file_data = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(data_path)
            .expect("Failed to open the data file!");

        let mut file_node = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(node_path)
            .expect("Failed to open the node file!");

        let mut file_reverse_structure = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(reverse_structure_path)
            .expect("Failed to open the reverse structure file!");

        if file_structure.metadata().map(|m| m.len()).unwrap_or(0) == 0{
            file_structure.set_len(FILE_INITIAL_SIZE)
                .expect("Failed to set file size!");
        }

        if file_data.metadata().map(|m| m.len()).unwrap_or(0)== 0{
            file_data.set_len(FILE_INITIAL_SIZE)
                .expect("Failed to set file size!");
        }
        if file_reverse_structure.metadata().map(|m| m.len()).unwrap_or(0) == 0{
            file_reverse_structure.set_len(FILE_INITIAL_SIZE)
                .expect("Failed to set file size");
        }

        if file_node.metadata().map(|m| m.len()).unwrap_or(0) == 0{
            file_node.set_len(FILE_INITIAL_SIZE)
                .expect("Failed to set file size");
            let initial_super_block = SuperBlock::new();
            let bytes_to_write: &[u8] = bytemuck::bytes_of(&initial_super_block);
            file_node.write_all(bytes_to_write).unwrap();
        }

        let mmap_structure = unsafe{
            MmapOptions::new()
                .map_mut(&file_structure)
                .unwrap()
        };

        let mmap_data = unsafe{
            MmapOptions::new()
                .map_mut(&file_data)
                .unwrap()
        };

        let mmap_node = unsafe{
            MmapOptions::new()
                .map_mut(&file_node)
                .unwrap()
        };

        let mmap_reverse_structure = unsafe{
            MmapOptions::new()
                .map_mut(&file_reverse_structure)
                .unwrap()
        };

        DiskStorage { file_structure, file_data, mmap_structure, mmap_data, _marker: PhantomData, file_node, mmap_node, file_reverse_structure, mmap_reverse_structure}
    }

    /// Loads a copy of the [`SuperBlock`] from the start of the node memory map.
    ///
    /// This method performs a bitwise copy of the underlying bytes. Note that 
    /// changes made to the returned struct are **not** persisted to disk 
    /// until they are explicitly written back.
    ///
    /// # Safety
    /// This function is safe to call as long as:
    /// * The `mmap_node` has been initialized with at least `size_of::<SuperBlock>()` bytes.
    /// * The underlying memory contains a valid, initialized instance of [`SuperBlock`].
    ///
    /// # Panics
    /// While this function does not explicitly panic, accessing the returned data 
    /// may cause a hardware exception (SIGBUS) if the underlying file is 
    /// truncated or deleted by another process.
    pub fn get_super_block(&self) -> SuperBlock{
        let super_block:SuperBlock = unsafe{
            let raw_ptr = self.mmap_node.as_ptr();
            std::ptr::read(raw_ptr as *const SuperBlock)
        };
        super_block
    }

    /// Calculates the absolute byte offset of a [`DiskNode`] within the node storage file
    ///
    /// The `node.bin` file follows a linear layout where the [SuperBlock] resides at the head of the
    /// file, followed by a contigous, fixed size [`DiskNode`] records
    ///
    /// The offset is calculated:
    /// $$offset = SUPER\_BLOCK\_SIZE + (node\_id \times size\_of::<DiskNode>())$$
    ///
    /// # Arguments
    /// * `node_id` - the zero-based index of the node to locate
    ///
    /// # Return
    /// * The node offset from the start of the memory map where the node's data begins
    pub fn calculate_node_offset(&self, node_id: &u64) -> u64{
        SUPER_BLOCK_SIZE as u64 + (node_id * std::mem::size_of::<DiskNode>() as u64)
    }

    /// Persists a [`DiskNode`] to its indexed position withing the `node.bin` file.
    ///
    /// This function uses the [`node_idx`] within provided [`DiskNode`] to determine the write
    /// destination via [`Self::calculate_node_offset`]
    /// 
    /// Note that this function writes to the memory-mapped region. The data will be synced to the
    /// physical disk by the Operating System asynchronously unless an explicit flush is triggered
    ///
    /// # Arguments
    /// * `disk_node` - the node record to be serialized and written
    ///
    /// # Panics
    /// Panics if the calculated offset or node size exceeds the current bounds of the memory map.
    /// See [`writing_bytes_to_mmap`]
    pub fn write_disk_node(&mut self, disk_node: &DiskNode){
        let offset = self.calculate_node_offset(&disk_node.node_idx);
        let bytes = disk_node.convert_to_bytes();

        writing_bytes_to_mmap(&mut self.mmap_node, offset, offset + bytes.len() as u64, bytes);
    }

    /// Loads a copy of [`DiskNode`] with the [`node_idx`] equal to `source`
    ///
    /// This function uses the index to determine the read destination via
    /// [`Self::calculate_node_offset`]
    ///
    /// **Important** that this function only gets a copy from the memory-mapped, so the changed made to the
    /// returned [`DiskNode`] will not be seen in the file, until a write has been made
    ///
    /// # Arguments
    /// * `source` - The unique identifier (index) of the node to retrieve
    /// 
    /// # Panics
    /// Panics if the calculated offset or node size exceeds the current bounds of the memory map.
    /// (e.g. `source` is out of bounds)
    pub fn get_disk_node(&self, source: &u64) -> DiskNode{
        let offset = self.calculate_node_offset(source);

        // We slice the mmap and use bytemuck for a zero-copy cast to a reference,
        // then clone it to return an owned struct.
        let disk_node_bytes: &[u8]= &self.mmap_node[offset as usize .. offset as usize + std::mem::size_of::<DiskNode>()];
        let disk_node: &DiskNode = bytemuck::from_bytes(disk_node_bytes);

        disk_node.clone()
    }

    /// Zeroes out the edge and reverse-edge regions for a newly allocated node.
    ///
    /// Called during the first edge insertion for a node to ensure the
    /// backing memory is clean before writing.
    ///
    /// This function uses the [`node_idx`] provided within [`DiskNode`] to determine where to
    /// intialize the block of bytes
    ///
    /// Note that this function zeroes the memory-mapped region. The data will be synced to the 
    /// physical disk by the Operating System asynchronously or if a manual flush is triggered
    ///
    /// # Arguments
    /// * `disk_node` - The node whose edge regions should be initialized.
    ///
    /// # Panics
    /// Panics if the offsets exceed the memory map bounds.
    pub fn initialize_disk_node(&mut self, disk_node: &DiskNode){

        let offset = disk_node.list_edges_offset;
        let reverse_offset = disk_node.list_reverse_edges_offset;

        zeroing_mmap(&mut self.mmap_structure, offset, offset + DISK_NODE_INITIAL_CAPACITY as u64);

        zeroing_mmap(&mut self.mmap_reverse_structure, reverse_offset, reverse_offset + DISK_NODE_INITIAL_CAPACITY as u64);
    }

    /// Calculates the absolute byte offset of a [`DiskEdge`] within the edge storage file
    /// 
    /// The `structure.bin` follows a linear layout, where is a contigous block of bytes, fixed size [`DiskEdge`]
    /// records
    ///
    /// The offset is calculated:
    /// $$offset= start\_offset + (edge\_numbers \times size\_of::<DiskEdge>())$$
    ///
    /// # Returns
    /// A **copy** of the computed offset (`u64` is `Copy`).
    pub fn calculate_edge_offset(&mut self, start_offset: &u64,  edge_numbers: &u64) -> u64{
        *start_offset + *edge_numbers * size_of::<DiskEdge>() as u64
    }

    /// Writes a [`DiskEdge`] to the structure memory map at the position
    /// determined by the node's current edge count.
    ///
    /// The edge is appended at the end of the node's edge block.
    ///
    /// # Arguments
    /// * `disk_node` - The node to which this edge belongs (used for offset calculation).
    /// * `disk_edge` - The edge record to write.
    ///
    /// # Panics
    ///  * Panics if the computed write region exceeds the structure memory map bounds.
    ///  * If it cannot resize the edge_block when its full
    ///
    pub fn write_disk_edge(&mut self, disk_node: &DiskNode, disk_edge: &DiskEdge){
        let index = disk_node.number_of_edges;
        let edge_offset = disk_node.get_edge_offset() + index * size_of::<DiskEdge>() as u64;
        
        let disk_edge_bytes: &[u8] = disk_edge.convert_into_bytes();
        writing_bytes_to_mmap(&mut self.mmap_structure, edge_offset, edge_offset + size_of::<DiskEdge>() as u64, disk_edge_bytes);
    }

    /// Writes raw weight bytes to the data memory map at the given offset.
    ///
    /// # Arguments
    /// * `weight_data_bytes` - The serialized weight data (**immutable reference**, not cloned).
    /// * `weight_offset` - The byte position in the data file.
    ///
    /// # Panics
    /// Panics if the write region exceeds the data memory map bounds or if the function
    /// `writing_bytes_to_mmap` panics. See more [`writing_bytes_to_mmap`]
    pub fn write_weight(&mut self, weight_data_bytes: &[u8], weight_offset: &u64){

        writing_bytes_to_mmap(&mut self.mmap_data, *weight_offset, *weight_offset + weight_data_bytes.len() as u64, weight_data_bytes);
    }
    /// Persists the [`SuperBlock`] to the beginning of the node memory map.
    ///
    /// Writes a **copy** of the provided superblock; the caller's struct
    /// is not moved or consumed (passed by reference).
    pub fn write_superblock(&mut self, superblock: &SuperBlock) {

        let bytes: &[u8] = superblock.convert_to_bytes();
        writing_bytes_to_mmap(&mut self.mmap_node, 0, SUPER_BLOCK_SIZE as u64, bytes);
    }

    /// Clears all edges from a node by zeroing its edge region on disk
    /// and resetting the edge count to 0.
    ///
    /// The node is **mutated in place** and then persisted.
    ///
    /// Note that this function writes to the memory-mapped region. The data will be synced to the
    /// physical disk by the Operating System asynchronously unless an explicit flush is triggered
    ///
    /// # Panics
    /// Panics if the edge region exceeds the structure memory map bounds.
    pub fn remove_edges_from_node(&mut self, disk_node: &mut DiskNode){
        let number_of_edges = disk_node.get_number_of_edges();
        let edges_offset = disk_node.get_edge_offset();

        let start = edges_offset;
        let number_of_bytes = number_of_edges * size_of::<DiskEdge>() as u64;
        let end = start + number_of_bytes;

        zeroing_mmap(&mut self.mmap_structure, start, end);

        disk_node.number_of_edges = 0;
        self.write_disk_node(disk_node);
    }
    /// Removes an edge using the "Swap-and-Pop" strategy to maintain 
    /// O(1) performance.
    ///
    /// The edge at `edge_number` is overwritten by the last edge in the 
    /// node's contiguous block. This avoids shifting all subsequent edges 
    /// (which would be O(N)) at the cost of changing the iteration order.
    ///
    /// # Safety
    /// This directly manipulates the memory-mapped bytes of `structure.bin` 
    /// via [`MmapMut::copy_within`].
    pub fn swap_remove_disk_edge(&mut self, disk_node: &mut DiskNode, edge_number: &u64){
        let last_index = disk_node.number_of_edges - 1;

        // only copy if we're not already removing the last edge
        if *edge_number != last_index {
            let edge_offset_removed = self.calculate_edge_offset(&disk_node.get_edge_offset(), edge_number);
            let last_edge_offset = self.calculate_edge_offset(&disk_node.get_edge_offset(), &last_index);
            let src_start = last_edge_offset as usize;
            let src_end = src_start + size_of::<DiskEdge>();
            let dest_start = edge_offset_removed as usize;

            self.mmap_structure.copy_within(src_start..src_end, dest_start);
        }

        disk_node.number_of_edges -= 1;
        let mut super_block = self.get_super_block();
        super_block.edge_count -=1;
        self.write_superblock(&super_block);
        self.write_disk_node(disk_node,);
    }
}

impl<W> StorageBackend<W> for DiskStorage<W>
where
    W: Clone + PartialEq + FromDiskBytes
{
    type EdgeIter<'a> = DiskEdgeIterator<'a, W> where Self: 'a, W: 'a;
    fn add_node(&mut self){
        let mut superblock: SuperBlock = self.get_super_block();

        let new_node_id = superblock.get_node_count();
        let disk_node: DiskNode = DiskNode::new(new_node_id, u64::MAX, u64::MAX);
        self.write_disk_node(&disk_node);

        superblock.increment_node_counter();

        self.write_superblock(&superblock);
    }

    fn add_edge_to_node(&mut self, node: u64, edge: &Edge<W>) {

        // gets the superblock from node
        let mut superblock: SuperBlock = self.get_super_block();

        let mut disk_node = self.get_disk_node(&node);

        if disk_node.list_edges_offset == u64::MAX{

            //initialize the disk node and puts the padding for space and the next free edge block
            disk_node.list_edges_offset = superblock.get_free_block_structure();
            disk_node.list_reverse_edges_offset = superblock.get_free_block_reverse_structure();
            self.initialize_disk_node(&disk_node);
            superblock.find_next_strcture_free_block(&(DISK_NODE_INITIAL_CAPACITY as u64));
            superblock.find_next_reverse_structure_free_block(&(DISK_NODE_INITIAL_CAPACITY as u64));
        }

        //TODO implement the check if the edge block is full

        let data_offset = superblock.get_free_block_data();
        
        // creates the disk edge and then conversts it into bytes and then writes it into
        // file_structure
        let disk_edge: DiskEdge = DiskEdge::new(data_offset, std::mem::size_of::<W>() as u64, edge.get_target());
        self.write_disk_edge(&disk_node, &disk_edge);
        
        //converts the weight of the edge into bytes and then writes into 
        let weight_data_bytes: &[u8] = edge.convert_to_bytes();
        self.write_weight(weight_data_bytes, &data_offset);

        superblock.next_data_free_block += weight_data_bytes.len() as u64;

        disk_node.number_of_edges+=1;
        self.write_disk_node(&disk_node);
        superblock.edge_count+=1;
        self.write_superblock(&superblock);
    }

    fn node_len(&self, node: u64) -> usize {
        let disk_node: DiskNode = self.get_disk_node(&node);
        disk_node.get_number_of_edges() as usize
    }

    fn get_edges<'a>(&'a self, node: u64) -> Self::EdgeIter<'a> where W: 'a {
        let disk_node: DiskNode = self.get_disk_node(&node);
        DiskEdgeIterator::new(self, &disk_node.get_edge_offset(), &disk_node.get_number_of_edges())
    }

    fn remove_edge<F>(&mut self, source: u64, edge: &Edge<W>, func: F) -> Result<Edge<W>, crate::GraphErrors>
        where
           F: Fn(&Edge<W>, &Edge<W>) -> bool {

        let edges = self.get_edges(source.clone());

        if let Some((idk, found_edge)) = edges.enumerate().find(|(_,e)| func(&e, edge)){
            let mut disk_node: DiskNode = self.get_disk_node(&source);
            self.swap_remove_disk_edge(&mut disk_node, &(idk as u64));
            return Ok(found_edge);
        }
        Err(GraphErrors::EdgeDoesntExists)
    }

    fn contains_edge(&self, source: u64, target: u64) -> Result<Edge<W>, crate::GraphErrors> {
        let _disk_node: DiskNode = self.get_disk_node(&source);

        let edges = self.get_edges(source);

        for edge in edges{
            if edge.get_target() == target{
                return Ok(edge);
            }
        }

        Err(crate::GraphErrors::EdgeDoesntExists)
    }

    fn node_count(&self) -> usize {
        let superblock = self.get_super_block();
        superblock.node_count as usize
    }

    fn edge_count(&self) -> usize {
        let superblock = self.get_super_block();
        superblock.edge_count as usize
    }

    fn increment_node_counter(&mut self) {
        let mut super_block = self.get_super_block();
        super_block.increment_node_counter();
        self.write_superblock(&super_block);
    }

    fn clear_node_edges(&mut self, node: u64) {
        let mut disk_node = self.get_disk_node(&node);
        self.remove_edges_from_node(&mut disk_node);

    }

    fn remove_edge_by_target(&mut self, source: u64, target: u64) {
        let mut disk_node: DiskNode = self.get_disk_node(&source);

        for edge_number in 0..disk_node.get_number_of_edges(){
            let edge_offset = self.calculate_edge_offset(&disk_node.get_edge_offset(), &(edge_number as u64));

        let struct_bytes = &self.mmap_structure[
            edge_offset as usize .. edge_offset as usize + size_of::<DiskEdge>()
        ];
            let disk_edge: &DiskEdge = bytemuck::from_bytes(struct_bytes);

            if disk_edge.node == target{
                self.swap_remove_disk_edge(&mut disk_node, &(edge_number));
                return;
            }
        }
        return;
    }

    fn add_reverse_edge(&mut self, _source: u64, _origin: u64) {
        let mut disk_node: DiskNode = self.get_disk_node(&_origin);
        
        if disk_node.list_reverse_edges_offset == u64::MAX {
            let mut superblock: SuperBlock = self.get_super_block();
            disk_node.list_edges_offset = superblock.get_free_block_structure();
            disk_node.list_reverse_edges_offset = superblock.get_free_block_reverse_structure();
            self.initialize_disk_node(&disk_node);
            superblock.find_next_strcture_free_block(&(DISK_NODE_INITIAL_CAPACITY as u64));
            superblock.find_next_reverse_structure_free_block(&(DISK_NODE_INITIAL_CAPACITY as u64));
            self.write_superblock(&superblock);
        }

        let edge_offset = disk_node.list_reverse_edges_offset + disk_node.number_of_reverse_edges * size_of::<u64>() as u64;


        let bytes = &_source.to_le_bytes();
        writing_bytes_to_mmap(&mut self.mmap_reverse_structure, edge_offset, edge_offset + bytes.len() as u64, bytes);

        disk_node.number_of_reverse_edges+=1;
        self.write_disk_node(&disk_node);

    }

    fn get_reverse_edges(&self, _node: u64) -> Vec<u64> {
        let disk_node = self.get_disk_node(&_node);
        DiskReverseEdgeIterator::new(& self, &disk_node.list_reverse_edges_offset, &disk_node.number_of_reverse_edges).collect()
    }

    fn clear_reverse_edges(&mut self, _node: u64) {
        let mut disk_node: DiskNode = self.get_disk_node(&_node);

        let start = disk_node.list_reverse_edges_offset;
        let number_of_bytes = size_of::<u64>() as u64 * disk_node.number_of_reverse_edges;
        let end = start + number_of_bytes;
        zeroing_mmap(&mut self.mmap_reverse_structure, start, end);

        disk_node.number_of_reverse_edges = 0;
        self.write_disk_node(&disk_node);
    }

    fn remove_reverse_edge(&mut self, source: u64, origin: u64) {
        let mut disk_node: DiskNode = self.get_disk_node(&source);

        if disk_node.list_reverse_edges_offset == u64::MAX {
            return;
        }

        for i in 0..disk_node.number_of_reverse_edges {
            let edge_offset = disk_node.list_reverse_edges_offset + i * size_of::<u64>() as u64;
            
            let start = edge_offset as usize;
            let end = start + size_of::<u64>();
            let bytes = &self.mmap_reverse_structure[start..end];
            let current_origin: u64 = u64::from_le_bytes(bytes.try_into().unwrap());

            if current_origin == origin {
                let last_index = disk_node.number_of_reverse_edges - 1;
                
                if i != last_index {
                    let last_offset = disk_node.list_reverse_edges_offset + last_index * size_of::<u64>() as u64;
                    let last_start = last_offset as usize;
                    let last_end = last_start + size_of::<u64>();
                    
                    self.mmap_reverse_structure.copy_within(last_start..last_end, start);
                }

                disk_node.number_of_reverse_edges -= 1;
                self.write_disk_node(&disk_node);
                return;
            }
        }
    }

    fn decrement_node_counter(&mut self) {
        let mut super_block = self.get_super_block();
        super_block.node_count -= 1;
        self.write_superblock(&super_block);
    }
}

