use bytemuck::Pod;
use bytemuck::Zeroable;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct DiskNode{
    pub node_idx: u64,
    pub list_edges_offset: u64,
    pub number_of_edges: u64,
}

impl DiskNode{
    pub fn new(node_idx: u64, list_edges_offset: u64, number_of_edges: u64) -> Self{
        Self { node_idx, list_edges_offset, number_of_edges }   
    }
    pub fn get_edge_offset(&self) -> u64{
        self.list_edges_offset
    }

    pub fn get_number_of_edges(&self) -> u64{
        self.number_of_edges
    }

    pub fn convert_to_bytes(&self) -> &[u8]{
        unsafe{
            std::slice::from_raw_parts(self as *const DiskNode as *const u8, std::mem::size_of::<DiskNode>())
        }
    }
}


