use std::io::Write;
use std::io::Seek;
use std::marker::PhantomData;
use std::path::Path;
use memmap2::MmapOptions;
use std::fs::OpenOptions;
use std::io::SeekFrom;

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


#[derive(Debug)]
pub struct DiskStorage<W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes
{

    pub(crate) file_structure: std::fs::File,
    pub(crate) file_data: std::fs::File,
    pub(crate) file_node: std::fs::File,
    pub(crate) file_reverse_structure: std::fs::File,

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
    pub fn new<P: AsRef<Path>>(directory: P) -> DiskStorage<W>
    {
        let dir = directory.as_ref();

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

        if file_structure.metadata().unwrap().len() == 0{
            file_structure.set_len(1024 * 1024).unwrap();
        }

        if file_data.metadata().unwrap().len() == 0{
            file_data.set_len(1024 * 1024).unwrap();
        }
        if file_reverse_structure.metadata().unwrap().len() == 0{
            file_reverse_structure.set_len(1024 * 1024).unwrap();
        }

        if file_node.metadata().unwrap().len() == 0{
            file_node.set_len(1024 * 1024).unwrap();
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

    pub fn get_super_block(&self) -> SuperBlock{
        let super_block:SuperBlock = unsafe{
            let raw_ptr = self.mmap_node.as_ptr();
            std::ptr::read(raw_ptr as *const SuperBlock)
        };
        super_block
    }

    pub fn calculate_node_offset(&self, node_id: &u64) -> u64{
        1024 + (node_id * std::mem::size_of::<DiskNode>() as u64)
    }
    pub fn write_disk_node(&mut self, disk_node: &DiskNode, node: &u64){
        let offset = self.calculate_node_offset(node);
        let bytes = disk_node.convert_to_bytes();

        self.mmap_node[offset as usize .. (offset as usize + bytes.len()) as usize].copy_from_slice(bytes);
    }

    pub fn get_disk_node(&self, source: &u64) -> DiskNode{
        let offset = self.calculate_node_offset(source);
        let disk_node_bytes: &[u8]= &self.mmap_node[offset as usize .. offset as usize + std::mem::size_of::<DiskNode>()];
        let disk_node: &DiskNode = bytemuck::from_bytes(disk_node_bytes);
        disk_node.clone()
    }

    pub fn initialize_disk_node(&mut self, disk_node: &DiskNode){

        let offset = disk_node.list_edges_offset;
        let reverse_offset = disk_node.list_reverse_edges_offset;

        let bytes: &[u8] = &[0; 1024];
        self.mmap_node[offset as usize .. (offset as usize + bytes.len()) as usize].copy_from_slice(bytes);


        self.mmap_node[reverse_offset as usize .. (reverse_offset as usize + bytes.len()) as usize].copy_from_slice(bytes);
    }

    pub fn calculate_edge_offset(&mut self, start_offset: &u64,  edge_numbers: &u64) -> u64{
        *start_offset + *edge_numbers * size_of::<DiskEdge>() as u64
    }

    pub fn write_disk_edge(&mut self, disk_node: &DiskNode, disk_edge: &DiskEdge){
        let index = disk_node.number_of_edges;
        let edge_offset = disk_node.get_edge_offset() + index * size_of::<DiskEdge>() as u64;
        
        let disk_edge_bytes: &[u8] = disk_edge.convert_into_bytes();
        self.mmap_structure[edge_offset as usize .. edge_offset as usize + disk_edge_bytes.len()].copy_from_slice(disk_edge_bytes);
    }

    pub fn write_weight(&mut self, weight_data_bytes: &[u8], weight_offset: &u64){

        self.mmap_data[*weight_offset as usize .. (*weight_offset as usize + weight_data_bytes.len())].copy_from_slice(weight_data_bytes);
    }
    pub fn write_superblock(&mut self, superblock: &SuperBlock) {

        let bytes: &[u8] = superblock.convert_to_bytes();
        self.mmap_node[0 .. 1024].copy_from_slice(bytes);
    }

    pub fn remove_edges_from_node(&mut self, disk_node: &mut DiskNode){
        let number_of_edges = disk_node.get_number_of_edges();
        let edges_offset = disk_node.get_edge_offset();

        let start = edges_offset as usize;
        let number_of_bytes = number_of_edges * size_of::<DiskEdge>() as u64;
        let end = start + number_of_bytes as usize;

        self.mmap_structure[start .. end].fill(0);

        disk_node.number_of_edges = 0;
        self.write_disk_node(disk_node, &disk_node.node_idx);
    }

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
        self.write_disk_node(disk_node, &disk_node.node_idx);
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
        self.write_disk_node(&disk_node, &(new_node_id));

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
            superblock.find_next_strcture_free_block(&1024);
            superblock.find_next_reverse_structure_free_block(&1024);
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
        self.write_disk_node(&disk_node, &node);
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
            superblock.find_next_strcture_free_block(&1024);
            superblock.find_next_reverse_structure_free_block(&1024);
            self.write_superblock(&superblock);
        }

        let edge_offset = disk_node.list_reverse_edges_offset + disk_node.number_of_reverse_edges * size_of::<u64>() as u64;


        let bytes = &_source.to_le_bytes();
        self.mmap_reverse_structure[edge_offset as usize .. edge_offset as usize + bytes.len()].copy_from_slice(bytes);

        disk_node.number_of_reverse_edges+=1;
        self.write_disk_node(&disk_node, &_origin);

    }

    fn get_reverse_edges(&self, _node: u64) -> Vec<u64> {
        let disk_node = self.get_disk_node(&_node);
        DiskReverseEdgeIterator::new(& self, &disk_node.list_reverse_edges_offset, &disk_node.number_of_reverse_edges).collect()
    }

    fn clear_reverse_edges(&mut self, _node: u64) {
        let mut disk_node: DiskNode = self.get_disk_node(&_node);

        let start = disk_node.list_reverse_edges_offset as usize;
        let number_of_bytes = size_of::<u64>() as u64 * disk_node.number_of_reverse_edges;
        let end = start + number_of_bytes as usize;
        self.mmap_reverse_structure[start .. end].fill(0);

        disk_node.number_of_reverse_edges = 0;
        self.write_disk_node(&disk_node, &_node);
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
                self.write_disk_node(&disk_node, &source);
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

