use crate::storage::disk_storage::{allocator_type::allocation::AllocationType, super_block::SuperBlock};

pub struct ReverseAllocation<'a>{
    super_block: &'a SuperBlock
}

impl<'a> ReverseAllocation<'a>{
    pub fn new(super_block: &'a SuperBlock) -> Self{
        Self{
            super_block
        }
    }
}

impl<'a> AllocationType for ReverseAllocation<'a>{
    fn set_ith_header_list(&self, index: u64, offset: u64) {
        self.super_block.set_new_header_reverse_structure(index, offset);
    }

    fn get_ith_header_list(&self, index: u64) -> u64 {
        self.super_block.get_ith_header_reverse_structure(index)
    }

    fn bump_allocate(&self, size: u64) -> u64{
        self.super_block.next_free_block_reverse_structure(size)
    }
}
