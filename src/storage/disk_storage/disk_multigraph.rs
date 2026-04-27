use std::io::Write;
use std::io::Seek;
use std::marker::PhantomData;
use std::path::Path;
use memmap2::MmapOptions;
use std::fs::OpenOptions;
use std::io::SeekFrom;

use crate::storage::disk_storage::disk_edge_iterator::DiskEdgeIterator;
use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
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

    pub(crate) mmap_structure: memmap2::MmapMut,
    pub(crate) mmap_data: memmap2::MmapMut,
    pub(crate) mmap_node: memmap2::MmapMut,
    _marker: PhantomData<W>,
}


impl<W> DiskStorage<W>
where
    W: Clone + std::cmp::PartialEq + FromDiskBytes,
{
    pub fn new<P1, P2, P3>(structure_path: P1, data_path: P2, node_path: P3) -> DiskStorage<W>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
        P3: AsRef<Path>,
    {
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

        if file_structure.metadata().unwrap().len() == 0{
            file_structure.set_len(1024 * 1024).unwrap();
        }

        if file_data.metadata().unwrap().len() == 0{
            file_data.set_len(1024 * 1024).unwrap();
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

        DiskStorage { file_structure, file_data, mmap_structure, mmap_data, _marker: PhantomData, file_node, mmap_node}
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
    pub fn write_disk_node(&mut self, disk_node: &DiskNode, offset: &u64){
        self.file_node.seek(SeekFrom::Start(*offset));
        self.file_node.write_all(disk_node.convert_to_bytes());
        self.file_node.seek(SeekFrom::Start(0));
    }

    pub fn get_disk_node(&self, offset: &u64) -> DiskNode{
        let disk_node_bytes: &[u8]= &self.mmap_node[*offset as usize .. *offset as usize + std::mem::size_of::<DiskNode>()];
        let disk_node: &DiskNode = bytemuck::from_bytes(disk_node_bytes);
        disk_node.clone()
    }

    pub fn initialize_disk_node(&mut self, offset: &u64){
        self.file_structure.
            seek(SeekFrom::Start(*offset));
        self.file_structure.
            write_all(&[0; 1024]);
        self.file_structure
            .seek(SeekFrom::Start(0));
    }

    pub fn calculate_edge_offset(&mut self, start_offset: &u64,  edge_numbers: &u64) -> u64{
        *start_offset + *edge_numbers * size_of::<DiskEdge>() as u64
    }

    pub fn write_disk_edge(&mut self, disk_edge_bytes: &[u8], edge_offset: &u64){
        self.file_structure.seek(SeekFrom::Start(*edge_offset)).unwrap();
        self.file_structure.write_all(disk_edge_bytes);
        self.file_structure.seek(SeekFrom::Start(0)).unwrap();
    }

    pub fn write_weight(&mut self, weight_data_bytes: &[u8], weight_offset: &u64){
        self.file_data.seek(SeekFrom::Start(*weight_offset));
        self.file_data.write_all(weight_data_bytes);
        self.file_data.seek(SeekFrom::Start(0));
    }
    pub fn write_superblock(&mut self, superblock: &SuperBlock) {
        self.file_node.write_all(superblock.convert_to_bytes());
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

        let disk_node: DiskNode = DiskNode::new(new_node_id, u64::MAX, 0);

        let node_offset = self.calculate_node_offset(&new_node_id);

        self.write_disk_node(&disk_node, &node_offset);

        superblock.increment_node_counter();

        self.write_superblock(&superblock);
    }

    fn add_edge_to_node(&mut self, node: u32, edge: &Edge<W>) {

        // gets the superblock from node
        let mut superblock: SuperBlock = self.get_super_block();

        let node_offset = self.calculate_node_offset(&(node as u64));
        let mut disk_node = self.get_disk_node(&node_offset);

        if disk_node.list_edges_offset == u64::MAX{

            //initialize the disk node and puts the padding for space and the next free edge block
            disk_node.list_edges_offset = superblock.get_free_block_structure();
            self.initialize_disk_node(&disk_node.list_edges_offset);
            superblock.find_next_strcture_free_block(&1024);
        }

        //TODO implement the check if the edge block is full

        let edge_offset = self.calculate_edge_offset(&disk_node.get_edge_offset(), &disk_node.get_number_of_edges());
        let data_offset = superblock.get_free_block_data();
        
        // creates the disk edge and then conversts it into bytes and then writes it into
        // file_structure
        let disk_edge: DiskEdge = DiskEdge::new(data_offset, std::mem::size_of::<W>() as u64, edge.get_target());
        let disk_edge_bytes: &[u8] = disk_edge.convert_into_bytes();
        self.write_disk_edge(disk_edge_bytes, &edge_offset);
        
        //converts the weight of the edge into bytes and then writes into 
        let weight_data_bytes: &[u8] = edge.convert_to_bytes();
        self.write_weight(weight_data_bytes, &data_offset);

        superblock.next_data_free_block += weight_data_bytes.len() as u64;

        disk_node.number_of_edges+=1;
        self.write_disk_node(&disk_node, &node_offset);
        superblock.edge_count+=1;
        self.write_superblock(&superblock);
    }

    fn node_len(&self, node: u32) -> usize {
        let node_offset = self.calculate_node_offset(&(node as u64));
        let disk_node: DiskNode = self.get_disk_node(&node_offset);
        disk_node.get_number_of_edges() as usize
    }

    fn get_edges<'a>(&'a self, node: u32) -> Self::EdgeIter<'a> where W: 'a {
        let node_offset = self.calculate_node_offset(&(node as u64));
        let disk_node: DiskNode = self.get_disk_node(&node_offset);
        DiskEdgeIterator::new(self, &disk_node.get_edge_offset(), &disk_node.get_number_of_edges())
    }

    fn remove_edge<F>(&mut self, source: u32, edge: &Edge<W>, func: F) -> Result<Edge<W>, crate::GraphErrors>
        where
            F: Fn(&Edge<W>, &Edge<W>) -> bool {
        unimplemented!();
    }

    fn remove_node(&mut self, target: u32) {
        unimplemented!();
    }

    fn contains_edge(&self, source: u32, target: u32) -> Result<Edge<W>, crate::GraphErrors> {
        unimplemented!();
    }

    fn node_count(&self) -> usize {
        let superblock: SuperBlock = unsafe {
            let raw_ptr = self.mmap_node.as_ptr();
            std::ptr::read(raw_ptr as *const SuperBlock)
        };
        superblock.node_count as usize
    }

    fn edge_count(&self) -> usize {
        let super_block: SuperBlock = unsafe{
            let raw_ptr = self.mmap_node.as_ptr();
            std::ptr::read(raw_ptr as *const SuperBlock)
        };
        super_block.edge_count as usize
    }

    fn increment_node_counter(&mut self) {
        unimplemented!()
    }
}

