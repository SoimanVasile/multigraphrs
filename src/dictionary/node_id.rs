#[repr(C)]
pub struct NodeId{
    data_len: u64,
    data_offset: u64,
}

impl NodeId{
    pub fn new(data_len: u64, data_offset: u64) -> Self{
        Self{ data_len, data_offset}
    }

    pub fn convert_to_bytes(&self) -> &[u8]{
        unsafe{
            std::slice::from_raw_parts(
                self as *const NodeId as *const u8, 
                std::mem::size_of::<NodeId>())
        }
    }
}
