#[repr(C)]
struct NodeId{
    data_len: u64,
    data_offset: u64,
}

impl NodeId{
    pub fn new(data_len: u64, data_offset: u64) -> Self{
        Self{ data_len, data_offset}
    }
}
