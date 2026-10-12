use crate::storage::disk_storage::{allocator_type::{allocation::AllocationType, reverse_allocation::ReverseAllocation, structure_allocation::StructureAllocation, weight_allocation::WeightAllocation}, super_block::SuperBlock, wal::FileId};

pub enum AllocatorStrategy<'a> {
    Structure(StructureAllocation<'a>),
    Reverse(ReverseAllocation<'a>),
    Weight(WeightAllocation<'a>),
}

impl<'a> AllocationType for AllocatorStrategy<'a> {
    fn get_ith_header_list(&self, index: u64) -> u64 {
        match self {
            Self::Structure(a) => a.get_ith_header_list(index),
            Self::Reverse(a) => a.get_ith_header_list(index),
            Self::Weight(a) => a.get_ith_header_list(index),
        }
    }

    fn set_ith_header_list(&self, index: u64, offset: u64) {
        match self {
            Self::Structure(a) => a.set_ith_header_list(index, offset),
            Self::Reverse(a) => a.set_ith_header_list(index, offset),
            Self::Weight(a) => a.set_ith_header_list(index, offset),
        }
    }

    fn bump_allocate(&self, size: u64) -> u64 {
        match self {
            Self::Structure(a) => a.bump_allocate(size),
            Self::Reverse(a) => a.bump_allocate(size),
            Self::Weight(a) => a.bump_allocate(size),
        }
    }
}

impl<'a> AllocatorStrategy<'a> {
    pub fn new(file_id: FileId, super_block: &'a SuperBlock) -> Self {
        match file_id {
            FileId::Structure => Self::Structure(StructureAllocation::new(super_block)),
            FileId::Reverse => Self::Reverse(ReverseAllocation::new(super_block)),
            FileId::Data => Self::Weight(WeightAllocation::new(super_block)),
            _ => panic!("Unsupported file id"),
        }
    }
}
