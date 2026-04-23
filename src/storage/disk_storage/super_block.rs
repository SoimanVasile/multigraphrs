use bytemuck::Pod;
use bytemuck::Zeroable;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct SuperBlock{
    pub magic_number: u32,
    pub version: u32,

    pub node_count: u64,
    pub edge_count: u64,

    pub next_structure_free_block: u64,
    pub next_data_free_block: u64,

    pub _padding: [u8; 984],
}

unsafe impl Pod for SuperBlock{}
unsafe impl Zeroable for SuperBlock {}

impl SuperBlock {
    pub fn new() -> Self{
        Self{
            magic_number: u32::from_le_bytes(*b"MGRF"),
            version: 1,
            node_count: 0,
            edge_count: 0,

            next_structure_free_block: 0,
            next_data_free_block: 0,

            _padding: [0; 984],

        }
    }

    pub fn get_node_count(&self) -> u64{
        self.node_count
    }

    pub fn increment_node_counter(&mut self){
        self.node_count+=1;
    }

    pub fn get_super_block_bytes(&self) -> &[u8]{
        unsafe{
            std::slice::from_raw_parts(
                self as *const SuperBlock as *const u8,
                std::mem::size_of::<SuperBlock>()
            )
        }
    }

    pub fn get_free_block_structure(&self) -> u64{
        self.next_structure_free_block
    }

    pub fn find_next_strcture_free_block(&mut self, size: &u64){
        self.next_structure_free_block += *size;
    }

    pub fn get_free_block_data(&self) -> u64{
        self.next_data_free_block
    }
}
