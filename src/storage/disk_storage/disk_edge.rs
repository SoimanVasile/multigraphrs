use bytemuck::Pod;
use bytemuck::Zeroable;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct DiskEdge{
    pub weight_offset: u64,
    pub weight_len: u64,
    pub node: u64,
}

impl DiskEdge{
    pub fn new(weight_offset: u64, weight_len: u64, node: u64) -> DiskEdge{
        DiskEdge { weight_offset, weight_len, node}
    }

    pub fn convert_into_bytes(&self) -> &[u8]{
        unsafe{
            std::slice::from_raw_parts(
            self as *const DiskEdge as *const u8, 
            std::mem::size_of::<DiskEdge>()
                )
        }
    }
}

