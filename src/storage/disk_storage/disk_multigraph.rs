use crate::dictionary::dictionary_strategy::DictionaryStrategy;
use crate::dictionary::disk_dictionary::DiskDictionary;
use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::path::Path;
use std::path::PathBuf;

use crate::core::graph_errors::GraphError;
use crate::core::db_error::DbError;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::storage::disk_storage::allocator::AllocatedStruct;
use crate::storage::disk_storage::disk_edge_iterator::DiskEdgeIterator;
use crate::storage::disk_storage::disk_edge_iterator::DiskReverseEdgeIterator;
use crate::storage::disk_storage::disk_node::DISK_NODE_INITIAL_CAPACITY;
use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::super_block::SuperBlock;
use crate::storage::disk_storage::disk_edge::DiskEdge;
use crate::storage::disk_storage::disk_node::DiskNode;
use crate::StorageBackend;
use crate::Edge;
use crate::storage::disk_storage::file_manager::FileManager;
use crate::storage::disk_storage::wal::{WalManager, WalTransaction, WalRecord, FileId};

const SUPER_BLOCK_SIZE: usize = 1024;
const WAL_BIN_MAX_FILE_SIZE: u64 = 1024 * 1024 * 1024;


/// allocates memory for the edges on the respective file_manager
///
/// # Arguments
/// * `disk_node` - which [`DiskNode`] to allocate for
/// * `file_manager` - which file to be allocated in
/// * `file_id` - the FileId for the file_manager (if its different then the function will have UB)
/// * `super_block` - [`SuperBlock`]
/// * `tx` - the [`WalTransaction`] to save the changes
/// 
///
fn allocated_disk_node(disk_node: &mut DiskNode, file_manager: &mut FileManager, file_id: FileId, super_block: &mut SuperBlock, tx: &mut WalTransaction) -> Result<(), DbError>{

    let (edge_offset, capacity) = match file_id{
        FileId::Structure => (&mut disk_node.list_edges_offset, disk_node.capacity),
        FileId::Reverse => (&mut disk_node.list_reverse_edges_offset, disk_node.reverse_capacity),
        FileId::Node => {return Ok(());},
        FileId::Data => {return Ok(());},
        FileId::NodeId => {return Ok(());},
    };
    *edge_offset = {
        let mut alloc = AllocatedStruct::new(file_manager, super_block, Some(tx), file_id);
        alloc.allocate_structure(&DISK_NODE_INITIAL_CAPACITY)
    };

    while *edge_offset + capacity > file_manager.file_len()?{
        tx.increase_file_size(file_id, file_manager.check_next_size( file_manager.file_len()?)?);
        file_manager.increase_file_size()?;
    }
    tx.zero_mmap(file_id, *edge_offset, *edge_offset + capacity);

    Ok(())
}


/// Checks if the edges for the node was allocated
///
///  # Arguments
///  * `disk_node` - the [`DiskNode`] to check
///  * `file_id` - which file to check if it the [`DiskNode`] was allocated
///
/// # Panics:
/// If file_id is different to Reverse or Structure as these are the only files which contains
/// edges
fn check_node_allocated(disk_node: &DiskNode, file_id: FileId) -> Result<bool, DbError>{
    let edge_offset = match file_id{
        FileId::Reverse => disk_node.list_reverse_edges_offset,
        FileId::Structure => disk_node.list_edges_offset,
        FileId::Data => return Err(DbError::InvalidFileId(3)),
        FileId::Node => return Err(DbError::InvalidFileId(2)),
        FileId::NodeId => return Err(DbError::InvalidFileId(4)),
    };

    Ok(edge_offset == u64::MAX)
}

/// Allocates memory in the file with the respective size
///
/// # Side Effects
/// May increase the size of the underlying file.
///
/// # Errors
/// None (internal operations may unwrap or panic on IO error).
///
/// # Panics
/// Panics if the underlying memory allocation fails.
fn allocate_memory(file_manager: &mut FileManager, super_block: &mut SuperBlock, tx: Option<&mut WalTransaction>, file_id: FileId, size: u64) -> u64{
    
        let mut alloc = AllocatedStruct::new(file_manager, super_block, tx, file_id);
        alloc.allocate_structure(&size)
}

/// Resizes a disk node by allocating a larger edge block and copying data.
///
/// # Side Effects
/// Allocates new memory, copies data, deallocates old memory, and may increase file size.
///
/// # Errors
/// Returns `std::io::Error` if file operations fail.
///
/// # Panics
/// Panics if memory cannot be allocated.
fn resizing_disk_node(file_manager: &mut FileManager, super_block: &mut SuperBlock, disk_node: &mut DiskNode, mut tx: Option<&mut WalTransaction>) -> Result<(), DbError>{
    disk_node.capacity *= 2;
    let free_offset = {
        allocate_memory(file_manager, super_block, tx.as_deref_mut(), FileId::Structure, disk_node.capacity)
    };

    while free_offset + disk_node.capacity > file_manager.file_len()?{
        if let Some(ref mut t) = tx { t.increase_file_size(FileId::Structure, file_manager.check_next_size(file_manager.file_len()?)?); }        file_manager.increase_file_size()?;
    }
    let edge_offset= disk_node.list_edges_offset;
    let edge_offset_end = edge_offset + (disk_node.number_of_edges * size_of::<DiskEdge>() as u64);

    if let Some(t) = tx.as_deref_mut() {
        let bytes = file_manager.reading_bytes(edge_offset, edge_offset_end);
        t.write_bytes(FileId::Structure, free_offset, bytes);
    } else {
        file_manager.copy_within(edge_offset, edge_offset_end, free_offset);
    }
    {
        let mut alloc = AllocatedStruct::new(file_manager, super_block, tx, FileId::Structure);
        let mut old_node = *disk_node;
        old_node.list_edges_offset = edge_offset;
        old_node.capacity /= 2;
        alloc.deallocator(&old_node);
    }
    disk_node.list_edges_offset = free_offset;

    Ok(())
}

/// Resizes a disk node's reverse edge block by allocating a larger block and copying data.
///
/// # Side Effects
/// Allocates new memory, copies data, deallocates old memory, and may increase file size.
///
/// # Errors
/// Returns `std::io::Error` if file operations fail.
///
/// # Panics
/// Panics if file size cannot be increased or memory cannot be allocated.
pub fn resizing_disk_node_reverse(file_manager: &mut FileManager, super_block: &mut SuperBlock, disk_node: &mut DiskNode, mut tx: Option<&mut WalTransaction>) -> Result<(), DbError>{
    let old_offset = disk_node.list_reverse_edges_offset;
    disk_node.reverse_capacity *= 2;
    let free_offset = {
        let mut alloc = AllocatedStruct::new(file_manager, super_block, tx.as_deref_mut(), FileId::Reverse);
        alloc.allocate_structure(&disk_node.reverse_capacity)
    };

    while free_offset + disk_node.reverse_capacity > file_manager.file_len()? {
        if let Some(ref mut t) = tx { t.increase_file_size(FileId::Reverse, file_manager.check_next_size(file_manager.file_len()?)?); }
        file_manager.increase_file_size()?;    }

    let src_end = old_offset + (disk_node.number_of_reverse_edges * size_of::<u64>() as u64);
    if let Some(t) = tx.as_deref_mut() {
        let bytes = file_manager.reading_bytes(old_offset, src_end);
        t.write_bytes(FileId::Reverse, free_offset, bytes);
    } else {
        file_manager.copy_within(old_offset, src_end, free_offset);
    }

    {
        let mut alloc = AllocatedStruct::new(file_manager, super_block, tx, FileId::Reverse);
        let mut old_node = *disk_node;
        old_node.list_reverse_edges_offset = old_offset;
        old_node.reverse_capacity /= 2;
        alloc.deallocator(&old_node);
    }
    disk_node.list_reverse_edges_offset = free_offset;
    Ok(())
}

#[derive(Debug)]
pub struct DiskStorage<K, W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes + AsDiskBytes,
    K: Clone + Eq + Hash + FromDiskBytes + AsDiskBytes,
{
    pub(crate) file_manager_node: FileManager,
    pub(crate) file_manager_edge_structure: FileManager,
    pub(crate) file_manager_reverse_edge: FileManager,
    pub(crate) file_manager_weight_data: FileManager,
    pub(crate) wal_manager: WalManager,
    pub(crate) directory: PathBuf,
    pub(crate) hashed_nodes: DiskDictionary<K>,
    node_count: u64,
    edge_count: u64,
    is_poisoned: AtomicBool,
    _marker: PhantomData<W>,
}


impl<K, W> DiskStorage<K, W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes + AsDiskBytes,
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes,
{
    /// Marks the database as poisoned. Once poisoned, all write operations
    /// will return `DbError::Poisoned`.
    pub fn poison(&self) {
        self.is_poisoned.store(true, Ordering::SeqCst);
    }

    /// Returns `true` if the database has been poisoned by a previous I/O error.
    pub fn is_poisoned(&self) -> bool {
        self.is_poisoned.load(Ordering::SeqCst)
    }

    /// Checks if the database is poisoned and returns an error if so.
    fn check_poisoned(&self) -> Result<(), DbError> {
        if self.is_poisoned() {
            Err(DbError::Poisoned)
        } else {
            Ok(())
        }
    }

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
    /// # Side Effects
    /// Creates directories, files, and memory maps. Replays WAL if present.
    ///
    /// # Errors
    /// None (panics instead).
    ///
    /// # Panics
    /// This function will panic if:
    /// * The directory cannot be created due to permission or path errors.
    /// * Any of the required `.bin` files cannot be opened or created.
    /// * The filesystem fails to report file metadata or set the initial file length.
    /// * Memory mapping the files fails (e.g., out of virtual address space).
    pub fn new<P: AsRef<Path>>(directory: P) -> DiskStorage<K, W>
    {
        let dir = directory.as_ref();

        std::fs::create_dir_all(dir)
            .expect("Failed to create the storage directory!");

        let structure_path = dir.join("structure.bin");
        let data_path = dir.join("data.bin");
        let node_path = dir.join("node.bin");
        let reverse_structure_path = dir.join("reverse_structure.bin");
        let node_id_path = dir.join("node_id.bin");
        let node_id_value_path = dir.join("node_id_data.bin");

        let (mut file_node, node_file_created) = FileManager::new(node_path)
            .expect("Failed to open the file_node");
        let (mut file_structure, _)= FileManager::new(structure_path)
            .expect("Failed to open the file_structure");
        let (mut file_reverse, _)= FileManager::new(reverse_structure_path)
            .expect("Failed to open the file_reverse");
        let (mut file_data, _) = FileManager::new(data_path)
            .expect("Failed to open the file_data");
        let (mut file_node_id, _) = FileManager::new(node_id_path)
            .expect("Failed to open the node_id");

        let (mut file_node_value_id, _) = FileManager::new(node_id_value_path)
            .expect("Failed to open the node_id_value");

        let mut wal_manager = WalManager::new(dir.to_path_buf())
            .expect("Failed to initialize WalManager");
            
        wal_manager.replay(&mut file_node, &mut file_structure, &mut file_reverse, &mut file_data, &mut file_node_id, &mut file_node_value_id)
            .expect("Failed to replay WAL transactions on startup");

        wal_manager.start(WAL_BIN_MAX_FILE_SIZE)
            .expect("Failed to start WAL background thread");

        if node_file_created{
            let initial_super_block = SuperBlock::new();
            let bytes_superblock: &[u8] = bytemuck::bytes_of(&initial_super_block);
            file_node.writing_bytes_to_mmap(0, SUPER_BLOCK_SIZE as u64, bytes_superblock);
        }


        let super_block_bytes = file_node.reading_bytes(0, 1024);
        let super_block: SuperBlock = *bytemuck::from_bytes(super_block_bytes)  ;

        let node_count = super_block.node_count;
        let edge_count = super_block.edge_count;
        let dictionary = DiskDictionary::new::<K, &Path>(dir, &wal_manager);

        DiskStorage {
            file_manager_node: file_node,
            file_manager_edge_structure: file_structure,
            file_manager_reverse_edge: file_reverse,
            file_manager_weight_data: file_data,
            node_count,
            edge_count,
            wal_manager,
            directory: dir.to_path_buf(),
            is_poisoned: AtomicBool::new(false),
            hashed_nodes: dictionary,
            _marker: PhantomData::<W>
        }
    }

    /// Commits a WAL transaction and flushes graph data files if the WAL was rotated.
    ///
    /// When the background thread rotates the WAL file (because it exceeded
    /// `max_file_size`), `commit` returns `rotated = true`. This method then
    /// flushes all four memory-mapped data files so the operations in the now
    /// `old_wal.bin` are persisted. On the *next* rotation the background thread
    /// deletes `old_wal.bin` — by that point this flush is guaranteed complete.
    fn commit_and_flush(&mut self, tx: &WalTransaction) -> Result<(), DbError> {
        let rotated = self.wal_manager.commit(tx).map_err(DbError::Io)?;
        if rotated {
            self.file_manager_node.flush()?;
            self.file_manager_edge_structure.flush()?;
            self.file_manager_reverse_edge.flush()?;
            self.file_manager_weight_data.flush()?;
        }
        Ok(())
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
    /// # Side Effects
    /// Writes the superblock to the memory map or WAL transaction.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// While this function does not explicitly panic, accessing the returned data 
    /// may cause a hardware exception (SIGBUS) if the underlying file is 
    /// truncated or deleted by another process.
    pub fn write_superblock(&mut self, superblock: &SuperBlock, tx: Option<&mut WalTransaction>) {
        let bytes = bytemuck::bytes_of(superblock);
        if let Some(t) = tx {
            t.write_bytes(FileId::Node, 0, bytes);
        } else {
            self.file_manager_node.writing_bytes_to_mmap(0, SUPER_BLOCK_SIZE as u64, bytes);
        }
    }

    /// Retrieves the superblock from the memory map.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if reading from the memory map fails.
    pub fn get_super_block(&self) -> SuperBlock{
        let superblock_bytes:&[u8] = self.file_manager_node.reading_bytes(0, SUPER_BLOCK_SIZE as u64);
        let super_block: &SuperBlock = bytemuck::from_bytes(superblock_bytes);
        *super_block
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
    ///
    /// # Errors
    /// None.
    ///
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
    /// # Side Effects
    /// Writes the node data to the memory map or WAL transaction. May increase file size.
    ///
    /// # Errors
    /// Returns `std::io::Error` if file size cannot be increased.
    ///
    /// # Panics
    /// Panics if the calculated offset or node size exceeds the current bounds of the memory map.
    /// See [`writing_bytes_to_mmap`]
    pub fn write_disk_node(&mut self, disk_node: &DiskNode, mut tx: Option<&mut WalTransaction>) -> Result<(), DbError>{
        let offset = self.calculate_node_offset(&disk_node.node_idx);
        let bytes = disk_node.convert_to_bytes();

        while offset + bytes.len() as u64 > self.file_manager_node.file_len()?{
            if let Some(ref mut t) = tx { t.increase_file_size(FileId::Node, self.file_manager_node.check_next_size(self.file_manager_node.file_len()?)?); }            self.file_manager_node.increase_file_size()?;
        }
        if let Some(t) = tx {
            t.write_bytes(FileId::Node, offset, bytes);
        } else {
            self.file_manager_node.writing_bytes_to_mmap(offset, offset + bytes.len() as u64, bytes);
        }
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
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if the calculated offset or node size exceeds the current bounds of the memory map.
    /// (e.g. `source` is out of bounds)
    pub fn get_disk_node(&self, source: &u64) -> DiskNode{
        let offset = self.calculate_node_offset(source);

        let disk_node_bytes: &[u8] = self.file_manager_node.reading_bytes(offset, offset + std::mem::size_of::<DiskNode>() as u64);
        let disk_node: &DiskNode = bytemuck::from_bytes(disk_node_bytes);

        *disk_node
    }

    /// Computes the byte offset of the `edge_numbers`-th edge within a node's
    /// edge block in the structure file.
    ///
    /// # Returns
    /// A **copy** of the computed offset (`u64` is `Copy`).
    ///
    /// # Errors
    /// None.
    ///
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
    /// # Side Effects
    /// Resizes the node if capacity is reached. Writes the edge data to the memory map or WAL.
    /// Modifies the `disk_node` edge count.
    ///
    /// # Errors
    /// Returns `std::io::Error` if file operations fail during resizing or writing.
    ///
    /// # Panics
    /// Panics if the computed write region exceeds the structure memory map bounds.
    pub fn write_disk_edge(&mut self, disk_node: &mut DiskNode, disk_edge: &DiskEdge, superblock: &mut SuperBlock, mut tx: Option<&mut WalTransaction>) -> Result<(), DbError>{

        if  !disk_node.verify_enough_capacity(){
            resizing_disk_node(&mut self.file_manager_edge_structure, superblock, disk_node, tx.as_deref_mut())?;
        }

        let index = disk_node.number_of_edges;
        let edge_offset = disk_node.get_edge_offset() + index * size_of::<DiskEdge>() as u64;
        let disk_edge_bytes: &[u8] = disk_edge.convert_into_bytes();

        if let Some(ref mut t) = tx {
            t.write_bytes(FileId::Structure, edge_offset, disk_edge_bytes);
        } else {
            self.file_manager_edge_structure.writing_bytes_to_mmap(edge_offset, edge_offset + size_of::<DiskEdge>() as u64, disk_edge_bytes);
        }

        disk_node.number_of_edges+=1;
        self.write_disk_node(disk_node, tx)?;
        Ok(())
    }

    /// Writes a reverse edge to the node's reverse edge block.
    ///
    /// # Side Effects
    /// Resizes the reverse edge block if necessary. Writes data to memory map or WAL.
    /// Modifies the `disk_node` reverse edge count.
    ///
    /// # Errors
    /// Returns `std::io::Error` if file operations fail during resizing or writing.
    ///
    /// # Panics
    /// Panics if writing exceeds the memory map bounds.
    pub fn write_reverse_edge(&mut self, disk_node: &mut DiskNode, source: &u64, super_block: &mut SuperBlock, mut tx: Option<&mut WalTransaction>) -> Result<(), DbError>{
        if !disk_node.verify_enough_reverse_capacity(){
            resizing_disk_node_reverse(&mut self.file_manager_reverse_edge, super_block, disk_node, tx.as_deref_mut())?;
        }

        let edge_offset = disk_node.list_reverse_edges_offset + disk_node.number_of_reverse_edges * size_of::<u64>() as u64;
        let bytes = source.to_le_bytes();

        if let Some(ref mut t) = tx{
            t.write_bytes(FileId::Reverse, edge_offset, &bytes);
        } else{
            self.file_manager_reverse_edge.writing_bytes_to_mmap(edge_offset, edge_offset + size_of::<u64>() as u64, &bytes);
        }

        disk_node.number_of_reverse_edges+=1;
        self.write_disk_node(disk_node, tx)?;

        Ok(())
    }

    /// Writes raw weight bytes to the data memory map at the given offset.
    ///
    /// # Arguments
    /// * `weight_data_bytes` - The serialized weight data (**immutable reference**, not cloned).
    /// * `weight_offset` - The byte position in the data file.
    ///
    /// # Side Effects
    /// May increase the size of the data file. Writes weight data to memory map or WAL.
    ///
    /// # Errors
    /// Returns `std::io::Error` if the data file size cannot be increased.
    ///
    /// # Panics
    /// Panics if the write region exceeds the data memory map bounds.
    pub fn write_weight(&mut self, weight_data_bytes: &[u8], weight_offset: &u64, mut tx: Option<&mut WalTransaction>) -> Result<(), DbError>{

        while *weight_offset + weight_data_bytes.len() as u64 > self.file_manager_weight_data.file_len()?{
            if let Some(ref mut t) = tx { t.increase_file_size(FileId::Data, self.file_manager_weight_data.check_next_size(self.file_manager_weight_data.file_len()?)?); }            self.file_manager_weight_data.increase_file_size()?;
        }

        if let Some(t) = tx {
            t.write_bytes(FileId::Data, *weight_offset, weight_data_bytes);
        } else {
            self.file_manager_weight_data.writing_bytes_to_mmap(*weight_offset, *weight_offset + weight_data_bytes.len() as u64, weight_data_bytes);
        }

        Ok(())
    }
    /// Persists the [`SuperBlock`] to the beginning of the node memory map.
    ///
    /// Writes a **copy** of the provided superblock; the caller's struct
    /// is not moved or consumed (passed by reference).
    ///
    /// # Panics
    /// Panics if the superblock bytes exceed the node memory map bounds.
    /// Clears all edges from a node by zeroing its edge region on disk
    /// and resetting the edge count to 0.
    ///
    /// The node is **mutated in place** and then persisted.
    ///
    /// # Side Effects
    /// Zeroes out the memory-mapped region for the edges. Modifies the global superblock
    /// and writes back the updated node and superblock to disk/WAL.
    ///
    /// # Errors
    /// Returns `std::io::Error` if updating the node fails.
    ///
    /// # Panics
    /// Panics if the edge region exceeds the structure memory map bounds.
    pub fn remove_edges_from_node(&mut self, disk_node: &mut DiskNode, mut tx: Option<&mut WalTransaction>)-> Result<(), DbError>{
        let mut super_block = self.get_super_block();
        let mut node_changed = false;

        if disk_node.number_of_edges > 0 {
            let start = disk_node.get_edge_offset();
            let end = start + (disk_node.number_of_edges * size_of::<DiskEdge>() as u64);

            if let Some(ref mut t) = tx {
                t.zero_mmap(FileId::Structure, start, end);
            } else {
                self.file_manager_edge_structure.zeroing_mmap(start, end);
            }

            super_block.edge_count -= disk_node.number_of_edges;
            self.edge_count -= disk_node.number_of_edges;
            
            {
                let mut alloc = AllocatedStruct::new(&mut self.file_manager_edge_structure, &mut super_block, tx.as_deref_mut(), FileId::Structure);
                alloc.deallocator(disk_node);
            }

            disk_node.number_of_edges = 0;
            disk_node.list_edges_offset = u64::MAX;
            disk_node.capacity = DISK_NODE_INITIAL_CAPACITY;
            node_changed = true;
        }

        if disk_node.number_of_reverse_edges > 0 {
            let start = disk_node.list_reverse_edges_offset;
            let end = start + (disk_node.number_of_reverse_edges * size_of::<u64>() as u64);

            if let Some(ref mut t) = tx {
                t.zero_mmap(FileId::Reverse, start, end);
            } else {
                self.file_manager_reverse_edge.zeroing_mmap(start, end);
            }

            {
                let mut alloc = AllocatedStruct::new(&mut self.file_manager_reverse_edge, &mut super_block, tx.as_deref_mut(), FileId::Reverse);
                alloc.deallocator(disk_node);
            }

            disk_node.number_of_reverse_edges = 0;
            disk_node.list_reverse_edges_offset = u64::MAX;
            disk_node.reverse_capacity = DISK_NODE_INITIAL_CAPACITY;
            node_changed = true;
        }

        if node_changed {
            self.write_superblock(&super_block, tx.as_deref_mut());
            self.write_disk_node(disk_node, tx)?;
        }
        
        Ok(())
    }

    /// Removes an edge at index `edge_number` from a node's edge block using
    /// swap-remove semantics: the last edge is copied over the removed one.
    ///
    /// Decrements both the node's edge count and the global edge counter
    /// in the superblock. The node is **mutated in place** and persisted.
    ///
    /// # Side Effects
    /// Copies memory within the edge block. Modifies the node and superblock, writing them to disk/WAL.
    ///
    /// # Errors
    /// Returns `std::io::Error` if updating the node fails.
    ///
    /// # Panics
    /// * Panics if `edge_number >= disk_node.number_of_edges`
    ///   (causes underflow on `number_of_edges - 1`).
    /// * Panics if any computed offsets exceed the structure memory map bounds.
    pub fn swap_remove_disk_edge(&mut self, disk_node: &mut DiskNode, edge_number: &u64, super_block: &mut SuperBlock, mut tx: Option<&mut WalTransaction>) -> Result<(), DbError>{

        if disk_node.number_of_edges == 0{
            return Ok(())
        }
        let last_index = disk_node.number_of_edges - 1;

        // only copy if we're not already removing the last edge
        if *edge_number != last_index {
            let edge_offset_removed = self.calculate_edge_offset(&disk_node.get_edge_offset(), edge_number);
            let last_edge_offset = self.calculate_edge_offset(&disk_node.get_edge_offset(), &last_index);
            let src_start = last_edge_offset;
            let src_end = src_start + size_of::<DiskEdge>() as u64;
            let dest_start = edge_offset_removed;

            if let Some(ref mut t) = tx {
                let bytes = self.file_manager_edge_structure.reading_bytes(src_start, src_end);
                t.write_bytes(FileId::Structure, dest_start, bytes);
            } else {
                self.file_manager_edge_structure.copy_within(src_start, src_end, dest_start);
            }
        }

        disk_node.number_of_edges -= 1;
        super_block.edge_count -=1;
        self.edge_count -= 1;
        self.write_disk_node(disk_node, tx)?;
        Ok(())
    }

    /// Allocates or reuses a node ID.
    ///
    /// # Side Effects
    /// May modify the `superblock` by updating the node count or free list header.
    ///
    /// # Errors
    /// None.
    ///
    pub fn next_node_id(&self, superblock: &mut SuperBlock) -> u64 {
        let node_id = superblock.next_free_node();

        if node_id == u64::MAX {
            superblock.node_count += 1;
            return superblock.node_count - 1;
        }

        let disk_node = self.get_disk_node(&node_id);
        let next_id = disk_node.get_edge_offset(); // We store the ID directly
        superblock.change_header(&next_id);

        node_id
    }

    /// Applies a WAL transaction directly to the memory maps.
    ///
    /// # Side Effects
    /// Modifies the memory maps according to the records in the WAL transaction.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if a write, zero, or copy within operation goes out of bounds.
    pub fn apply_wal_transaction(&mut self, tx: &WalTransaction) {
        for record in &tx.records {
            match record {
                WalRecord::Write { file_id, offset, bytes } => {
                    let fm = match file_id {
                        FileId::Node => &mut self.file_manager_node,
                        FileId::Structure => &mut self.file_manager_edge_structure,
                        FileId::Reverse => &mut self.file_manager_reverse_edge,
                        FileId::Data => &mut self.file_manager_weight_data,
                        _ => continue,
                    };
                    fm.writing_bytes_to_mmap(*offset, *offset + bytes.len() as u64, bytes);
                }
                WalRecord::Zero { file_id, offset, end } => {
                    let fm = match file_id {
                        FileId::Node => &mut self.file_manager_node,
                        FileId::Structure => &mut self.file_manager_edge_structure,
                        FileId::Reverse => &mut self.file_manager_reverse_edge,
                        FileId::Data => &mut self.file_manager_weight_data,
                        _ => continue,
                    };
                    fm.zeroing_mmap(*offset, *end);
                }
                WalRecord::CopyWithin { file_id, src_start, src_end, dest_start } => {
                    let fm = match file_id {
                        FileId::Node => &mut self.file_manager_node,
                        FileId::Structure => &mut self.file_manager_edge_structure,
                        FileId::Reverse => &mut self.file_manager_reverse_edge,
                        FileId::Data => &mut self.file_manager_weight_data,
                        _ => continue,
                    };
                    fm.copy_within(*src_start, *src_end, *dest_start);
                }
                WalRecord::IncreaseFileSize { file_id: _, size: _} => {                    // Already applied directly to satisfy length checks during buffer phase.
                }
            }
        }
    }

    pub fn swap_remove_disk_reverse_edge(&mut self, disk_node: &mut DiskNode, edge_number: &u64, mut tx: Option<&mut WalTransaction>) -> Result<(), DbError> {

        if disk_node.number_of_reverse_edges == 0{
            return Ok(());
        }
        let last_index = disk_node.number_of_reverse_edges - 1;

        if *edge_number != last_index {
            let start = disk_node.list_reverse_edges_offset + edge_number * std::mem::size_of::<u64>() as u64;
            let last_start = disk_node.list_reverse_edges_offset + last_index * std::mem::size_of::<u64>() as u64;
            let last_end = last_start + std::mem::size_of::<u64>() as u64;
            
            if let Some(ref mut t) = tx {
                let bytes = self.file_manager_reverse_edge.reading_bytes(last_start, last_end);
                t.write_bytes(FileId::Reverse, start, bytes);
            } else {
                self.file_manager_reverse_edge.copy_within(last_start, last_end, start);
            }
        }

        disk_node.number_of_reverse_edges -= 1;
        self.write_disk_node(disk_node, tx)?;
        Ok(())
    }
}

impl<K, W> StorageBackend<K, W> for DiskStorage<K, W>
where
    K: Clone + Eq + Hash + FromDiskBytes + AsDiskBytes,
    W: Clone + PartialEq + FromDiskBytes + AsDiskBytes,
{
    type EdgeIter<'a> = DiskEdgeIterator<'a, K, W> where Self: 'a, W: 'a;
    /// Adds a new node to the storage.
    ///
    /// # Side Effects
    /// Persists a new `DiskNode` and updates the `SuperBlock`. May increase file sizes.
    /// Commits and applies a WAL transaction.
    ///
    /// # Errors
    /// None (panics instead).
    ///
    /// # Panics
    /// Panics if disk writing or WAL commit fails.
    fn add_node(&mut self) -> Result<u64, GraphError> {
        self.check_poisoned()?;
        let mut tx = WalTransaction::new();
        let mut superblock: SuperBlock = self.get_super_block();

        let new_node_id = self.next_node_id(&mut superblock);
        self.node_count = superblock.node_count;
        let disk_node: DiskNode = DiskNode::new(new_node_id, u64::MAX, u64::MAX);
        
        self.write_disk_node(&disk_node, Some(&mut tx))
            .map_err(|e| {
                self.poison();
                GraphError::Db(e)
            })?;
        self.write_superblock(&superblock, Some(&mut tx));

        // Commit to WAL and then flush to actual mmap
        self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;        self.apply_wal_transaction(&tx);

        Ok(new_node_id)
    }

    /// Adds multiple nodes to the storage.
    ///
    /// # Side Effects
    /// Persists multiple `DiskNode`s and updates the `SuperBlock`. May increase file sizes.
    /// Commits and applies a WAL transaction.
    ///
    /// # Errors
    /// None (panics instead).
    ///
    /// # Panics
    /// Panics if disk writing or WAL commit fails.
    fn bulk_add_node(&mut self, number_of_nodes: &u64) -> Result<Vec<u64>, GraphError> {
        self.check_poisoned()?;
        let mut tx = WalTransaction::new();
        let mut super_block: SuperBlock = self.get_super_block();

        let mut new_ids: Vec<u64> = Vec::with_capacity(*number_of_nodes as usize);
        for i in 0..*number_of_nodes{
            let id = self.next_node_id(&mut super_block);
            self.node_count = super_block.node_count;
            new_ids.push(id);
            let disk_node: DiskNode = DiskNode::new(new_ids[i as usize], u64::MAX, u64::MAX);
            self.write_disk_node(&disk_node, Some(&mut tx)).map_err(|e| {
                self.poison();
                GraphError::Db(e)
            })?;
        }

        self.write_superblock(&super_block, Some(&mut tx));

        self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;        self.apply_wal_transaction(&tx);
        Ok(new_ids)
    }

    /// Adds an edge to a given node.
    ///
    /// # Side Effects
    /// Writes the edge structure and weight data to disk. May increase file sizes.
    /// Commits and applies a WAL transaction.
    ///
    /// # Errors
    /// None (panics instead).
    ///
    /// # Panics
    /// Panics if disk writing or WAL commit fails.
    fn add_edge_to_node(&mut self, node: &u64, edge: &Edge<W>) -> Result<(), GraphError> {
        self.check_poisoned()?;
        let mut tx = WalTransaction::new();
        let mut disk_node = self.get_disk_node(node);
        let mut superblock = self.get_super_block();

        if check_node_allocated(&disk_node, FileId::Structure).map_err(|e| { self.poison(); GraphError::from(e) })? {
                allocated_disk_node(&mut disk_node, &mut self.file_manager_edge_structure, FileId::Structure, &mut superblock, &mut tx).map_err(|e| { self.poison(); GraphError::from(e) })?;            }

        let data_offset = superblock.get_free_block_data();
        let disk_edge: DiskEdge = DiskEdge::new(data_offset, std::mem::size_of::<W>() as u64, edge.get_target());

        self.write_disk_edge(&mut disk_node, &disk_edge, &mut superblock, Some(&mut tx)).map_err(|e| {
            self.poison();
            GraphError::Db(e)
        })?;
        
        let weight_data_bytes: &[u8] = edge.convert_to_bytes();
        self.write_weight(weight_data_bytes, &data_offset, Some(&mut tx)).map_err(|e| {
            self.poison();
            GraphError::Db(e)
        })?;

        superblock.next_data_free_block += weight_data_bytes.len() as u64;
        superblock.edge_count += 1;
        self.edge_count += 1;
        
        self.write_superblock(&superblock, Some(&mut tx));

        self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;
        self.apply_wal_transaction(&tx);
        Ok(())
    }

    /// Adds multiple edges to nodes in bulk.
    ///
    /// # Side Effects
    /// Writes edges and weight data to disk, potentially increasing file sizes.
    /// Groups writes into WAL transactions to minimize overhead.
    ///
    /// # Errors
    /// Returns `std::io::Error` if file operations fail.
    ///
    /// # Panics
    /// Panics if disk writing or WAL commit fails.
    fn bulk_add_edge_to_node(&mut self, edges: &[(u64, Edge<W>)]) -> Result<(), GraphError> {
        self.check_poisoned()?;
        let mut tx = WalTransaction::new();
        let mut super_block = self.get_super_block();

        let mut seen_disk_node: HashMap<u64, DiskNode> = HashMap::new();
        for (node, edge) in edges{
            let mut disk_node = *seen_disk_node
                .entry(*node)
                .or_insert_with(|| self.get_disk_node(node));
            if check_node_allocated(&disk_node, FileId::Structure).map_err(|e| { self.poison(); GraphError::from(e) })? {
                allocated_disk_node(&mut disk_node, &mut self.file_manager_edge_structure, FileId::Structure, &mut super_block, &mut tx).map_err(|e| { self.poison(); GraphError::from(e) })?;            }

            let data_offset = super_block.get_free_block_data();
            let disk_edge: DiskEdge = DiskEdge::new(data_offset, std::mem::size_of::<W>() as u64, edge.get_target());

            self.write_disk_edge(&mut disk_node, &disk_edge, &mut super_block, Some(&mut tx)).map_err(|e| {
                self.poison();
                GraphError::Db(e)
            })?;
            
            let weight_data_bytes: &[u8] = edge.convert_to_bytes();
            self.write_weight(weight_data_bytes, &data_offset, Some(&mut tx)).map_err(|e| {
                self.poison();
                GraphError::Db(e)
            })?;

            super_block.next_data_free_block += weight_data_bytes.len() as u64;
            super_block.edge_count += 1;
            self.edge_count += 1;
            
            // Update the cached disk node so subsequent edges use the correct edge count!
            seen_disk_node.insert(*node, disk_node);

        }

        if !tx.records.is_empty() {
            self.write_superblock(&super_block, Some(&mut tx));

            self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;            self.apply_wal_transaction(&tx);
        }
        Ok(())
    }

    /// Retrieves the number of edges for a specific node.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if the node does not exist or read goes out of bounds.
    fn node_len(&self, node: &u64) -> usize {
        let disk_node: DiskNode = self.get_disk_node(node);
        disk_node.get_number_of_edges() as usize
    }

    /// Retrieves an iterator over the edges of a specific node.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if the node does not exist or read goes out of bounds.
    fn get_edges<'a>(&'a self, node: &u64) -> Self::EdgeIter<'a> where W: 'a, K: 'a{
        let disk_node: DiskNode = self.get_disk_node(node);
        DiskEdgeIterator::new(self, &disk_node.get_edge_offset(), &disk_node.get_number_of_edges())
    }

    /// Removes the first edge from `source` which the target and weight match
    ///
    /// # Arguments
    /// * `source` - the source node
    /// * `edge` - which edge should be removed
    ///
    /// # Returns
    /// The removed [`Edge`] (**owned**) on succes
    ///
    /// # Error
    /// If the edge doesnt exist it will return [`GraphError::EdgeDoesntExist`]
    ///
    /// # Panics
    /// Panics if `source` is out of bounds
    fn remove_edge(&mut self, source: &u64, edge: &Edge<W>) -> Result<Edge<W>, GraphError> {
        self.check_poisoned()?;
        let edges = self.get_edges(source);
        let mut super_block: SuperBlock = self.get_super_block();

        if let Some((idk, found_edge)) = edges.enumerate().find(|(_, e)| e.get_target()== edge.get_target() && edge.get_weight() == e.get_weight()){
            let mut disk_node: DiskNode = self.get_disk_node(source);
            let mut tx = WalTransaction::new();
            self.swap_remove_disk_edge(&mut disk_node, &(idk as u64), &mut super_block, Some(&mut tx))
                .map_err(|e| { self.poison(); GraphError::from(e) })?;
            self.write_superblock(&super_block, Some(&mut tx));
            self.commit_and_flush(&tx)
                .map_err(|e| { self.poison(); GraphError::from(e) })?;            self.apply_wal_transaction(&tx);
            return Ok(found_edge);
        }
        Err(GraphError::EdgeDoesntExist)
    }


    /// Removes the edges in the `edges` array which the target and weight match
    ///
    /// # Arguments
    /// * `edges` - an array with the strucutre [(source, target, weight)]
    ///
    /// # Panics
    /// Panics if `source` is out of bounds
    fn bulk_remove_edge(&mut self, edges: &[(u64, Edge<W>)]) -> Result<(), GraphError> {
        self.check_poisoned()?;
        let mut seen_disk_node: HashMap<u64, DiskNode> = HashMap::with_capacity(edges.len()/2);
        let mut super_block: SuperBlock = self.get_super_block();
        let mut tx = WalTransaction::new();

        for (source, edge) in edges{
            let edges = self.get_edges(source);

            if let Some((index, _)) = edges.enumerate().find(|(_, e)| e.get_target() == edge.get_target() && e.get_weight() == edge.get_weight()){
                let mut disk_node = *seen_disk_node
                    .entry(*source)
                    .or_insert_with(|| self.get_disk_node(source));                
                self.swap_remove_disk_edge(&mut disk_node, &(index as u64),&mut super_block, Some(&mut tx))
                    .map_err(|e| {
                        self.poison();
                        GraphError::Db(e)
                    })?;
                seen_disk_node.insert(*source, disk_node);
            }
        }
        self.write_superblock(&super_block, Some(&mut tx));
        self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;        self.apply_wal_transaction(&tx);
        Ok(())
    }

    /// Removes an edge matching a specific condition.
    ///
    /// # Side Effects
    /// Uses swap-remove to delete the edge on disk, writes to the WAL, and commits.
    ///
    /// # Errors
    /// Returns `GraphError::EdgeDoesntExist` if the edge is not found.
    ///
    /// # Panics
    /// Panics on file I/O or WAL commit failure.
    fn remove_edge_by_property<F>(&mut self, source: &u64, edge: &Edge<W>, func: F) -> Result<Edge<W>, GraphError>
        where
           F: Fn(&Edge<W>, &Edge<W>) -> bool {
        self.check_poisoned()?;
        let edges = self.get_edges(source);
        let mut super_block: SuperBlock = self.get_super_block();

        if let Some((idk, found_edge)) = edges.enumerate().find(|(_,e)| func(e, edge)){
            let mut disk_node: DiskNode = self.get_disk_node(source);
            let mut tx = WalTransaction::new();
            self.swap_remove_disk_edge(&mut disk_node, &(idk as u64), &mut super_block, Some(&mut tx))
                .map_err(|e| { self.poison(); GraphError::from(e) })?;
            self.write_superblock(&super_block, Some(&mut tx));
            self.commit_and_flush(&tx)
                .map_err(|e| { self.poison(); GraphError::from(e) })?;            self.apply_wal_transaction(&tx);
            return Ok(found_edge);
        }
        Err(GraphError::EdgeDoesntExist)
    }

    /// Checks if a directed edge exists from `source` to `target`.
    ///
    /// # Errors
    /// Returns `GraphError::EdgeDoesntExist` if no such edge exists.
    ///
    /// # Panics
    /// Panics if memory reads go out of bounds.
    fn contains_edge(&self, source: &u64, target: &u64) -> Result<Edge<W>, GraphError> {
        let _disk_node: DiskNode = self.get_disk_node(source);

        let edges = self.get_edges(source);

        for edge in edges{
            if edge.get_target() == *target{
                return Ok(edge);
            }
        }

        Err(crate::core::graph_errors::GraphError::EdgeDoesntExist)
    }

    /// Returns the global count of nodes.
    ///
    /// # Errors
    /// None.
    ///
    fn node_count(&self) -> usize {
        self.node_count as usize
    }

    /// Returns the global count of edges.
    ///
    /// # Errors
    /// None.
    ///
    fn edge_count(&self) -> usize {
        self.edge_count as usize
    }

    /// Increments the node counter manually.
    ///
    /// # Side Effects
    /// Modifies the `SuperBlock` and persists it via WAL.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if disk writing or WAL commit fails.
    fn increment_node_counter(&mut self) -> Result<(), GraphError> {
        self.check_poisoned()?;
        let mut tx = WalTransaction::new();
        let mut super_block = self.get_super_block();
        super_block.increment_node_counter();
        self.node_count = super_block.node_count;
        self.write_superblock(&super_block, Some(&mut tx));
        self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;        self.apply_wal_transaction(&tx);
        Ok(())
    }

    /// Clears all edges from a specific node.
    ///
    /// # Side Effects
    /// Zeroes the edge regions, resets capacities, and writes via WAL.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics on file I/O or WAL commit failure.
    fn clear_node_edges(&mut self, node: &u64) -> Result<(), GraphError> {
        self.check_poisoned()?;
        let mut tx = WalTransaction::new();
        let mut disk_node = self.get_disk_node(node);
        self.remove_edges_from_node(&mut disk_node, Some(&mut tx)).map_err(|e| { self.poison();  GraphError::Db(e)})?;
        self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;        self.apply_wal_transaction(&tx);
        Ok(())
    }

    /// Removes an edge based on its target node.
    ///
    /// # Side Effects
    /// Uses swap-remove to delete the edge on disk, writes to WAL, and commits.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics on file I/O or WAL commit failure.
    fn remove_edge_by_target(&mut self, source: &u64, target: &u64) -> Result<(), GraphError> {
        self.check_poisoned()?;
        let mut disk_node: DiskNode = self.get_disk_node(source);
        let mut super_block: SuperBlock = self.get_super_block();

        for edge_number in 0..disk_node.get_number_of_edges(){
            let edge_offset = self.calculate_edge_offset(&disk_node.get_edge_offset(), &(edge_number));

            let struct_bytes = self.file_manager_edge_structure
                .reading_bytes(edge_offset, edge_offset + std::mem::size_of::<DiskEdge>() as u64);
            let disk_edge: &DiskEdge = bytemuck::from_bytes(struct_bytes);

            if disk_edge.node == *target{
                let mut tx = WalTransaction::new();
                self.swap_remove_disk_edge(&mut disk_node, &(edge_number), &mut super_block, Some(&mut tx)).map_err(|e| {
                        self.poison();
                        GraphError::Db(e)
                    })?;
                self.write_superblock(&super_block, Some(&mut tx));
                self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;                self.apply_wal_transaction(&tx);
                return Ok(());
            }
        }
        Ok(())
    }

    /// Adds a reverse edge pointing back to the origin node.
    ///
    /// # Side Effects
    /// May increase file size and allocate reverse edge block. Writes to WAL and commits.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if disk writing or WAL commit fails.
    fn add_reverse_edge(&mut self, source: &u64, origin: &u64) -> Result<(), GraphError> {
        self.check_poisoned()?;
        let mut tx = WalTransaction::new();
        let mut disk_node: DiskNode = self.get_disk_node(source);
        let mut superblock: SuperBlock = self.get_super_block();

        // First-time initialization: allocate a reverse edge block for this node
        if check_node_allocated(&disk_node, FileId::Reverse).map_err(|e| { self.poison(); GraphError::from(e) })? {
            allocated_disk_node(&mut disk_node, &mut self.file_manager_reverse_edge, FileId::Reverse, &mut superblock, &mut tx).map_err(|e| { self.poison(); GraphError::from(e) })?;        }

        // Check if adding this reverse edge would overflow the allocated capacity
        self.write_reverse_edge(&mut disk_node, origin, &mut superblock, Some(&mut tx)).map_err(|e| { self.poison(); GraphError::Db(e) })?;
        self.write_superblock(&superblock, Some(&mut tx));
        
        self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;        self.apply_wal_transaction(&tx);
        Ok(())
    }

    /// Adds multiple reverse edges in bulk.
    ///
    /// # Side Effects
    /// Uses a single WAL transaction to write multiple reverse edges. May allocate reverse edge blocks.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if disk writing or WAL commit fails.
    fn bulk_add_reverse_edge(&mut self, edges: &[(u64, u64, W)]) -> Result<(), GraphError> {
        self.check_poisoned()?;
        let mut tx = WalTransaction::new();
        let mut super_block = self.get_super_block();
        let mut seen_disk_node: HashMap<u64, DiskNode> = HashMap::with_capacity(edges.len());

        for (source, target, _) in edges{
            let mut disk_node = *seen_disk_node
                .entry(*source)
                .or_insert_with(|| self.get_disk_node(source));
            if check_node_allocated(&disk_node, FileId::Reverse).map_err(|e| { self.poison(); GraphError::from(e) })? {
                allocated_disk_node(&mut disk_node, &mut self.file_manager_reverse_edge, FileId::Reverse, &mut super_block, &mut tx).map_err(|e| { self.poison(); GraphError::from(e) })?;
            }

            self.write_reverse_edge(&mut disk_node, source, &mut super_block, Some(&mut tx))
                .map_err(|e|{ 
                    self.poison(); 
                    GraphError::Db(e)
                })?;            
            seen_disk_node.insert(*target, disk_node);
        };
        self.write_superblock(&super_block, Some(&mut tx));

        self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;        self.apply_wal_transaction(&tx);
        Ok(())
    }

    /// Retrieves all reverse edges (origins) for a specific node.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if reading from the memory map goes out of bounds.
    fn get_reverse_edges(&self, node: &u64) -> Vec<u64> {
        let disk_node = self.get_disk_node(node);
        DiskReverseEdgeIterator::new(self, &disk_node.list_reverse_edges_offset, &disk_node.number_of_reverse_edges).collect()
    }

    /// Clears all reverse edges for a specific node.
    ///
    /// # Side Effects
    /// Zeroes out the reverse edge region on disk, resets count to 0, writes to WAL, and commits.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics on file I/O or WAL commit failure.
    fn clear_reverse_edges(&mut self, node: &u64) -> Result<(), GraphError> {
        self.check_poisoned()?;
        let mut disk_node: DiskNode = self.get_disk_node(node);
        if disk_node.number_of_reverse_edges == 0{
            return Ok(());
        }

        let mut tx = WalTransaction::new();
        let start = disk_node.list_reverse_edges_offset;
        let number_of_bytes = size_of::<u64>() as u64 * disk_node.number_of_reverse_edges;
        let end = start + number_of_bytes;
        tx.zero_mmap(FileId::Reverse, start, end);

        disk_node.number_of_reverse_edges = 0;
        self.write_disk_node(&disk_node, Some(&mut tx)).map_err(|e| { self.poison(); GraphError::Db(e)})?;
        self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;        self.apply_wal_transaction(&tx);
        Ok(())
    }

    /// Removes a specific reverse edge from a node.
    ///
    /// # Side Effects
    /// Uses swap-remove to overwrite the reverse edge on disk, writes to WAL, and commits.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics on file I/O or WAL commit failure, or if byte conversion fails.
    fn remove_reverse_edge(&mut self, source: &u64, origin: &u64) -> Result<(), GraphError> {
        self.check_poisoned()?;
        let mut disk_node: DiskNode = self.get_disk_node(source);

        if check_node_allocated(&disk_node, FileId::Reverse).map_err(|e| { self.poison(); GraphError::from(e) })? {
            return Ok(());
        }

        for i in 0..disk_node.number_of_reverse_edges {
            let edge_offset = disk_node.list_reverse_edges_offset + i * std::mem::size_of::<u64>() as u64;
            let bytes = self.file_manager_reverse_edge.reading_bytes(edge_offset, edge_offset + std::mem::size_of::<u64>() as u64);
            let current_origin: u64 = u64::from_le_bytes(bytes.try_into().unwrap());

            if current_origin == *origin {
                let mut tx = WalTransaction::new();
                self.swap_remove_disk_reverse_edge(&mut disk_node, &i, Some(&mut tx))
                    .map_err(|e| { 
                        self.poison();
                        GraphError::Db(e)
                    })?;
                self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;                self.apply_wal_transaction(&tx);
                return Ok(());
            }
        }
        Ok(())
    }

    fn bulk_remove_reverse_edge(&mut self, edges: &[(u64, u64)]) -> Result<(), GraphError> {
        self.check_poisoned()?;
        if edges.is_empty() { return Ok(()); }

        let mut sorted_edges = edges.to_vec();
        sorted_edges.sort_unstable_by_key(|&(source, _)| source);

        let mut tx = WalTransaction::new();

        for chunk in sorted_edges.chunk_by(|a, b| a.0 == b.0) {
            let source = chunk[0].0;
            
            // Extract just the origins we want to remove for this source
            let mut origins_to_remove: Vec<u64> = chunk.iter().map(|&(_, o)| o).collect();
            let mut indices_to_remove = Vec::new();
            
            let mut disk_node = self.get_disk_node(&source);
            
            let total_bytes = disk_node.number_of_reverse_edges * size_of::<u64>() as u64;
            let start_offset = disk_node.list_reverse_edges_offset;
            
            let all_edges_bytes = self.file_manager_reverse_edge
                .reading_bytes(start_offset, start_offset + total_bytes);

            for i in 0..disk_node.number_of_reverse_edges {
                let byte_start = (i * size_of::<u64>() as u64) as usize;
                let byte_end = byte_start + size_of::<u64>();
                
                let current_origin = u64::from_le_bytes(
                    all_edges_bytes[byte_start..byte_end].try_into().unwrap()
                );

                if let Some(pos) = origins_to_remove.iter().position(|r| *r == current_origin) {
                    indices_to_remove.push(i);
                    origins_to_remove.swap_remove(pos);
                }
                if origins_to_remove.is_empty() {
                    break;
                }
            }

            indices_to_remove.sort_unstable_by(|a, b| b.cmp(a));

            for index in indices_to_remove {
                self.swap_remove_disk_reverse_edge(&mut disk_node, &index, Some(&mut tx))
                    .map_err(|e|{
                        self.poison();
                        GraphError::Db(e)
                    })?;
            }
        }

        self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;        self.apply_wal_transaction(&tx);
        Ok(())
    }

    /// Marks a node ID as free, adding it to the free list.
    ///
    /// # Side Effects
    /// Updates the node's disk record to point to the current head, and updates the superblock head.
    /// Writes to WAL and commits.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics on file I/O or WAL commit failure.
    fn free_node_id(&mut self, node_id: &u64) -> Result<(), GraphError> {
        self.check_poisoned()?;
        let mut tx = WalTransaction::new();
        let mut superblock = self.get_super_block();
        let head = superblock.next_free_node();
        
        let disk_node = DiskNode::new(u64::MAX, head, u64::MAX);
        let offset = self.calculate_node_offset(node_id);
        let bytes = disk_node.convert_to_bytes();
        tx.write_bytes(FileId::Node, offset, bytes);

        superblock.change_header(node_id);
        superblock.node_count -= 1;
        self.node_count -= 1;
        self.write_superblock(&superblock, Some(&mut tx));
        
        self.commit_and_flush(&tx).map_err(|e| { self.poison(); GraphError::from(e) })?;        self.apply_wal_transaction(&tx);
        Ok(())
    }

    fn hashed_nodes_contains_key(&self, key: &K) -> Result<bool, GraphError> {
        Ok(self.hashed_nodes.contains_key(key))
    }

    fn hashed_nodes_insert(&mut self, key: K, node_id: u64) -> Result<(), GraphError> {
        Ok(self.hashed_nodes.insert(key, node_id).map_err(|e| {
            GraphError::Db(e)
        })?)
    }

    fn hashed_nodes_get(&self,  key: &K) -> Result<Option<u64>, GraphError> {
        Ok(self.hashed_nodes.get(key))
    }

    fn hashed_nodes_remove(&mut self, key: &K) -> Result<Option<u64>, GraphError> {
        Ok(self.hashed_nodes.remove(key).map_err(|e| {
            GraphError::Db(e)
        })?)
    }
}
