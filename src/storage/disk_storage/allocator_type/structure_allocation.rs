use crate::storage::disk_storage::{allocator_type::allocation::AllocationType, super_block::SuperBlock};

struct StructureAllocation<'a>{
    super_block: &'a SuperBlock
}

impl<'a> AllocationType for StructureAllocation<'a>{
    fn get_ith_header_list(&self, index: u64) -> u64 {
        self.super_block.get_ith_header_structure(index)
    }

    fn bump_allocate(&self, size: u64) -> u64 {
        self.super_block.next_free_block_structure(size)
    }

    fn set_ith_header_list(&self, index: u64, offset: u64) {
        self.super_block.set_new_header_structure(index, offset);
    }
}
