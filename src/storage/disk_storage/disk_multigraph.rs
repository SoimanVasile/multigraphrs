use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;
use memmap2::MmapOptions;
use std::fs::OpenOptions;

use crate::GraphErrors;
use crate::storage::disk_storage::disk_edge_iterator::DiskEdgeIterator;
use crate::storage::disk_storage::disk_edge_iterator::DiskReverseEdgeIterator;
use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::super_block;
use crate::storage::disk_storage::super_block::SuperBlock;
use crate::storage::disk_storage::disk_edge::DiskEdge;
use crate::storage::disk_storage::disk_node::DiskNode;
use crate::StorageBackend;
use crate::Edge;
use crate::storage::disk_storage::file_manager::FileManager;

const SUPER_BLOCK_SIZE: usize = 1024;


#[derive(Debug)]
pub struct DiskStorage<W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes
{
    pub(crate) file_manager_node: FileManager,
    pub(crate) file_manager_edge_structure: FileManager,
    pub(crate) file_manager_reverse_edge: FileManager,
    pub(crate) file_manager_weight_data: FileManager,
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

        let (mut file_node, node_file_created) = FileManager::new(node_path)
            .expect("Failed to open the file_node");
        let (file_structure, _)= FileManager::new(structure_path)
            .expect("Failed to open the file_structure");
        let (file_reverse, _)= FileManager::new(reverse_structure_path)
            .expect("Failed to open the file_reverse");
        let (file_data, _) = FileManager::new(data_path)
            .expect("Failed to open the file_data");


        if node_file_created{
            let initial_super_block = SuperBlock::new();
            let bytes_superblock: &[u8] = bytemuck::bytes_of(&initial_super_block);
            file_node.writing_bytes_to_mmap(0, SUPER_BLOCK_SIZE as u64, bytes_superblock);
        }

        DiskStorage {
            file_manager_node: file_node,
            file_manager_edge_structure: file_structure,
            file_manager_reverse_edge: file_reverse,
            file_manager_weight_data: file_data,
            _marker: PhantomData::<W>
        }
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
        let superblock_bytes:&[u8] = self.file_manager_node.reading_bytes(0, SUPER_BLOCK_SIZE as u64);
        let super_block: &SuperBlock = bytemuck::from_bytes(superblock_bytes);
        super_block.clone()
    }

    /// Calculates the absolute byte offset of a [`DiskNode`] within the node storage file
    ///
    /// The `node.bin` file follow a linear layout where the [SuperBlock] resides at the head of the
    /// file, follow by a contigous, fixed size [`DiskNode`] records
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
    pub fn write_disk_node(&mut self, disk_node: &DiskNode) -> Result<(), std::io::Error>{
        let offset = self.calculate_node_offset(&disk_node.node_idx);
        let bytes = disk_node.convert_to_bytes();
        
        if offset + bytes.len() as u64 > self.file_manager_node.file_len()?{
            self.file_manager_node.increase_file_size()?;
        }
        self.file_manager_node.writing_bytes_to_mmap(offset, offset + bytes.len() as u64, bytes);
        Ok(())
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

        let disk_node_bytes: &[u8] = self.file_manager_node.reading_bytes(offset, offset + std::mem::size_of::<DiskNode>() as u64);
        let disk_node: &DiskNode = bytemuck::from_bytes(disk_node_bytes);

        disk_node.clone()
    }

    /// Zeroes out the edge and reverse-edge regions for a newly allocated node.
    ///
    /// Called during the first edge insertion for a node to ensure the
    /// backing memory is clean before writing.
    ///
    /// # Arguments
    /// * `disk_node` - The node whose edge regions should be initialized.
    ///
    /// # Panics
    /// Panics if the offsets exceed the memory map bounds.
    pub fn initialize_disk_node(&mut self, disk_node: &DiskNode){

        let offset = disk_node.list_edges_offset;
        let reverse_offset = disk_node.list_reverse_edges_offset;

        self.file_manager_edge_structure.zeroing_mmap(offset, offset + &disk_node.capacity);

        self.file_manager_reverse_edge.zeroing_mmap(reverse_offset, reverse_offset + &disk_node.reverse_capacity);
    }

    /// Computes the byte offset of the `edge_numbers`-th edge within a node's
    /// edge block in the structure file.
    ///
    /// # Returns
    /// A **copy** of the computed offset (`u64` is `Copy`).
    ///
    /// # Panics
    /// This method does not panic.
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
    /// Panics if the computed write region exceeds the structure memory map bounds.
    pub fn write_disk_edge(&mut self, disk_node: &mut DiskNode, disk_edge: &DiskEdge) -> Result<(), std::io::Error>{

        // Checks if we add this edge, it will overflow the already allocated memory and allocates
        // more if its full
        if  (disk_node.number_of_edges + 1) * size_of::<DiskEdge>() as u64 > disk_node.capacity{
            let mut super_block = self.get_super_block();
            disk_node.capacity *= 2;
            let free_offset = super_block.get_free_block_structure();
            super_block.find_next_strcture_free_block(&disk_node.capacity);

            if free_offset + disk_node.capacity > self.file_manager_edge_structure.file_len()?{
                self.file_manager_edge_structure.increase_file_size()?;
            }
            let edge_offset= disk_node.list_edges_offset;
            let edge_offset_end = edge_offset + (disk_node.number_of_edges * size_of::<DiskEdge>() as u64);

            self.file_manager_edge_structure.copy_within(edge_offset, edge_offset_end, free_offset);
            disk_node.list_edges_offset = free_offset;
            self.write_superblock(&super_block);
        }

        let index = disk_node.number_of_edges;
        let edge_offset = disk_node.get_edge_offset() + index * size_of::<DiskEdge>() as u64;
        let disk_edge_bytes: &[u8] = disk_edge.convert_into_bytes();
        self.file_manager_edge_structure.writing_bytes_to_mmap(edge_offset, edge_offset + size_of::<DiskEdge>() as u64, disk_edge_bytes);

        disk_node.number_of_edges+=1;
        self.write_disk_node(&disk_node)?;
        Ok(())
    }

    /// Writes raw weight bytes to the data memory map at the given offset.
    ///
    /// # Arguments
    /// * `weight_data_bytes` - The serialized weight data (**immutable reference**, not cloned).
    /// * `weight_offset` - The byte position in the data file.
    ///
    /// # Panics
    /// Panics if the write region exceeds the data memory map bounds.
    pub fn write_weight(&mut self, weight_data_bytes: &[u8], weight_offset: &u64) -> Result<(), std::io::Error>{

        if weight_offset + weight_data_bytes.len() as u64 > self.file_manager_weight_data.file_len()?{
            self.file_manager_weight_data.increase_file_size()?;
        }

        self.file_manager_weight_data.writing_bytes_to_mmap(*weight_offset, *weight_offset + weight_data_bytes.len() as u64, weight_data_bytes);

        Ok(())
    }
    /// Persists the [`SuperBlock`] to the beginning of the node memory map.
    ///
    /// Writes a **copy** of the provided superblock; the caller's struct
    /// is not moved or consumed (passed by reference).
    ///
    /// # Panics
    /// Panics if the superblock bytes exceed the node memory map bounds.
    pub fn write_superblock(&mut self, superblock: &SuperBlock) {

        let bytes: &[u8] = superblock.convert_to_bytes();
        self.file_manager_node.writing_bytes_to_mmap(0, SUPER_BLOCK_SIZE as u64, bytes);
    }

    /// Clears all edges from a node by zeroing its edge region on disk
    /// and resetting the edge count to 0.
    ///
    /// The node is **mutated in place** and then persisted.
    ///
    /// # Panics
    /// Panics if the edge region exceeds the structure memory map bounds.
    pub fn remove_edges_from_node(&mut self, disk_node: &mut DiskNode)-> Result<(), std::io::Error>{
        let number_of_edges = disk_node.get_number_of_edges();
        let edges_offset = disk_node.get_edge_offset();

        let start = edges_offset;
        let number_of_bytes = number_of_edges * size_of::<DiskEdge>() as u64;
        let end = start + number_of_bytes;

        self.file_manager_edge_structure.zeroing_mmap(start, end);

        let mut super_block = self.get_super_block();
        super_block.edge_count -= disk_node.number_of_edges;
        self.write_superblock(&super_block);

        disk_node.number_of_edges = 0;
        self.write_disk_node(disk_node)?;
        Ok(())
    }

    /// Removes an edge at index `edge_number` from a node's edge block using
    /// swap-remove semantics: the last edge is copied over the removed one.
    ///
    /// Decrements both the node's edge count and the global edge counter
    /// in the superblock. The node is **mutated in place** and persisted.
    ///
    /// # Panics
    /// * Panics if `edge_number >= disk_node.number_of_edges`
    ///   (causes underflow on `number_of_edges - 1`).
    /// * Panics if any computed offsets exceed the structure memory map bounds.
    pub fn swap_remove_disk_edge(&mut self, disk_node: &mut DiskNode, edge_number: &u64) -> Result<(), std::io::Error>{
        let last_index = disk_node.number_of_edges - 1;

        // only copy if we're not already removing the last edge
        if *edge_number != last_index {
            let edge_offset_removed = self.calculate_edge_offset(&disk_node.get_edge_offset(), edge_number);
            let last_edge_offset = self.calculate_edge_offset(&disk_node.get_edge_offset(), &last_index);
            let src_start = last_edge_offset;
            let src_end = src_start + size_of::<DiskEdge>() as u64;
            let dest_start = edge_offset_removed;

            self.file_manager_edge_structure.copy_within(src_start, src_end, dest_start);
        }

        disk_node.number_of_edges -= 1;
        let mut super_block = self.get_super_block();
        super_block.edge_count -=1;
        self.write_superblock(&super_block);
        self.write_disk_node(disk_node)?;
        Ok(())
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
            superblock.find_next_strcture_free_block(&disk_node.capacity);
            self.file_manager_edge_structure.zeroing_mmap(disk_node.list_edges_offset, disk_node.list_edges_offset + disk_node.capacity);
        }

        //TODO implement the check if the edge block is full

        let data_offset = superblock.get_free_block_data();
        
        // creates the disk edge and then conversts it into bytes and then writes it into
        // file_structure
        let disk_edge: DiskEdge = DiskEdge::new(data_offset, std::mem::size_of::<W>() as u64, edge.get_target());

        self.write_disk_edge(&mut disk_node, &disk_edge);
        
        //converts the weight of the edge into bytes and then writes into 
        let weight_data_bytes: &[u8] = edge.convert_to_bytes();
        self.write_weight(weight_data_bytes, &data_offset);

        superblock.next_data_free_block += weight_data_bytes.len() as u64;

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

        let struct_bytes = self.file_manager_edge_structure.reading_bytes(edge_offset, edge_offset + std::mem::size_of::<DiskEdge>() as u64);
        let disk_edge: &DiskEdge = bytemuck::from_bytes(struct_bytes);

            if disk_edge.node == target{
                self.swap_remove_disk_edge(&mut disk_node, &(edge_number));
                return;
            }
        }
        return;
    }

    fn add_reverse_edge(&mut self, source: u64, origin: u64) {
        let mut disk_node: DiskNode = self.get_disk_node(&origin);
        let mut superblock: SuperBlock = self.get_super_block();

        // First-time initialization: allocate a reverse edge block for this node
        if disk_node.list_reverse_edges_offset == u64::MAX {
            disk_node.list_reverse_edges_offset = superblock.get_free_block_reverse_structure();
            superblock.find_next_reverse_structure_free_block(&disk_node.reverse_capacity);

            if disk_node.list_reverse_edges_offset + disk_node.reverse_capacity > self.file_manager_reverse_edge.file_len().unwrap() {
                self.file_manager_reverse_edge.increase_file_size().unwrap();
            }
            self.file_manager_reverse_edge.zeroing_mmap(
                disk_node.list_reverse_edges_offset,
                disk_node.list_reverse_edges_offset + disk_node.reverse_capacity,
            );
        }

        // Check if adding this reverse edge would overflow the allocated capacity
        if (disk_node.number_of_reverse_edges + 1) * size_of::<u64>() as u64 > disk_node.reverse_capacity {
            let old_offset = disk_node.list_reverse_edges_offset;
            disk_node.reverse_capacity *= 2;
            let free_offset = superblock.get_free_block_reverse_structure();
            superblock.find_next_reverse_structure_free_block(&disk_node.reverse_capacity);

            if free_offset + disk_node.reverse_capacity > self.file_manager_reverse_edge.file_len().unwrap() {
                self.file_manager_reverse_edge.increase_file_size().unwrap();
            }

            let src_end = old_offset + (disk_node.number_of_reverse_edges * size_of::<u64>() as u64);
            self.file_manager_reverse_edge.copy_within(old_offset, src_end, free_offset);
            disk_node.list_reverse_edges_offset = free_offset;
        }

        let edge_offset = disk_node.list_reverse_edges_offset + disk_node.number_of_reverse_edges * size_of::<u64>() as u64;

        let bytes = &source.to_le_bytes();
        self.file_manager_reverse_edge.writing_bytes_to_mmap(edge_offset, edge_offset + bytes.len() as u64, bytes);

        disk_node.number_of_reverse_edges += 1;
        self.write_disk_node(&disk_node);
        self.write_superblock(&superblock);
    }

    fn get_reverse_edges(&self, node: u64) -> Vec<u64> {
        let disk_node = self.get_disk_node(&node);
        DiskReverseEdgeIterator::new(&self, &disk_node.list_reverse_edges_offset, &disk_node.number_of_reverse_edges).collect()
    }

    fn clear_reverse_edges(&mut self, _node: u64) {
        let mut disk_node: DiskNode = self.get_disk_node(&_node);

        let start = disk_node.list_reverse_edges_offset;
        let number_of_bytes = size_of::<u64>() as u64 * disk_node.number_of_reverse_edges;
        let end = start + number_of_bytes;
        self.file_manager_reverse_edge.zeroing_mmap(start, end);

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
            
            let start = edge_offset;
            let end = start + size_of::<u64>() as u64;
            let bytes = self.file_manager_reverse_edge.reading_bytes(start, end);
            let current_origin: u64 = u64::from_le_bytes(bytes.try_into().unwrap());

            if current_origin == origin {
                let last_index = disk_node.number_of_reverse_edges - 1;
                
                if i != last_index {
                    let last_offset = disk_node.list_reverse_edges_offset + last_index * size_of::<u64>() as u64;
                    let last_start = last_offset;
                    let last_end = last_start + size_of::<u64>() as u64;
                    
                    self.file_manager_reverse_edge.copy_within(last_start, last_end, start);
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

