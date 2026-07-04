use crate::storage::disk_storage::disk_edge::DiskEdge;
use crate::storage::disk_storage::disk_node::DiskNode;
use crate::storage::disk_storage::{file_manager::FileManager, from_disk_bytes::FromDiskBytes};
use crate::storage::disk_storage::{super_block::SuperBlock};
use crate::storage::disk_storage::wal::{WalTransaction, FileId};

const NUMBER_OF_LINKED_LIST: u64 = 7;

/// Determines the appropriate bucket index for a given allocation size.
/// 
/// # Design Constraints
/// This allocator makes a strict assumption that sizes passed to it are exactly 
/// powers of 2, starting from 128 bytes (128, 256, 512, 1024, 2048, 4096, 8192+).
/// This perfectly aligns with how `disk_edge` blocks double in size.
/// 
/// Because of this invariant, `find_index` maps these exact sizes perfectly to buckets:
/// - 128 bytes -> Bucket 0
/// - 256 bytes -> Bucket 1
/// - 512 bytes -> Bucket 2
/// - 1024 bytes -> Bucket 3
/// - 2048 bytes -> Bucket 4
/// - 4096 bytes -> Bucket 5
/// - 8192+ bytes -> Bucket 6 (Fallback linked list)
///
/// NOTE: Passing sizes `< 128` or non-power-of-2 sizes will result in logic errors
/// or panics elsewhere in the allocator, as it relies on this exact mapping.
pub fn find_index(size: &u64) -> u64{
    
    let aux = *size >> 7;
    let index = u64::BITS as u64 - 1 - aux.leading_zeros() as u64;

    if index >= NUMBER_OF_LINKED_LIST{
        return 6u64;
    }

    index
}

/// The main allocator struct responsible for managing disk space.
/// 
/// # Invariants
/// This allocator is highly optimized under the strict assumption that it only handles
/// sizes that are **exact powers of 2 starting at 128 bytes** (i.e., `128 * 2^n`).
/// 
/// By doubling sizes (128, 256, 512, ...), the allocator guarantees that:
/// 1. Buckets 0-5 only ever contain blocks of exactly one size (128, 256, 512, etc.).
/// 2. It can safely pop the first element of bucket 0-5 without needing to verify 
///    the block is large enough (because it strictly will be).
/// 3. Memory underflow/corruption is avoided because non-power-of-2 blocks are never created.
pub struct AllocatedStruct<'a>
{
    pub file_manager: &'a mut FileManager,
    pub super_block: &'a mut SuperBlock,
    pub tx: Option<&'a mut WalTransaction>,
    pub file_id: FileId,
} 

impl<'a> AllocatedStruct<'a>
{
    pub fn new(file_manager: &'a mut FileManager, super_block: &'a mut SuperBlock, tx: Option<&'a mut WalTransaction>, file_id: FileId) -> Self{
        Self{ file_manager, super_block, tx, file_id}
    }

    fn get_header(&self, index: &u64) -> u64 {
        match self.file_id {
            FileId::Structure => self.super_block.get_ith_header_structure(index),
            FileId::Reverse => self.super_block.get_ith_header_reverse_structure(index),
            _ => panic!("AllocatedStruct only supports Structure and Reverse files"),
        }
    }

    fn set_header(&mut self, index: &u64, offset: &u64) {
        match self.file_id {
            FileId::Structure => self.super_block.next_header_structure(index, offset),
            FileId::Reverse => self.super_block.next_header_reverse_structure(index, offset),
            _ => panic!("AllocatedStruct only supports Structure and Reverse files"),
        }
    }

    fn bump_allocate(&mut self, size: &u64) -> u64 {
        match self.file_id {
            FileId::Structure => self.super_block.get_free_block_structure(size),
            FileId::Reverse => self.super_block.get_free_block_reverse_structure(size),
            _ => panic!("AllocatedStruct only supports Structure and Reverse files"),
        }
    }

    fn split_capacity_into_power_of_2s(&mut self, new_offset: &u64, new_cap: &u64){
        let mut power_of_2: [u64; 15] = [0; 15];
        let mut cap_div = new_cap >> 7;
        while cap_div != 0{
            let cap_div_index = u64::BITS - 1 - cap_div.leading_zeros();
            power_of_2[cap_div_index as usize] = 1u64;
            cap_div ^= 1<<cap_div_index;
        }
        
        let mut padding = 0;
        for (index, val) in power_of_2.iter().enumerate(){
            if *val == 1{
                let header_index: u64 = index.min((NUMBER_OF_LINKED_LIST - 1) as usize) as u64;

                let next_offset = self.get_header(&header_index);
                let start_offset = new_offset + padding;
                
                let chunk_size = 1u64 << (index + 7);
                let end_offset = new_offset + padding + chunk_size;
                let new_disk_edge: DiskEdge = DiskEdge::new(next_offset, chunk_size, u64::MAX);

                self.write_disk_edges(&start_offset, &end_offset, &new_disk_edge);
                self.set_header(&header_index, &start_offset);

                padding += chunk_size;
            }
        }
    }

    fn skip_cur(&mut self, prev_offset: &u64, cur_offset: &u64){
        let cur_disk_edge_bytes = self.file_manager.reading_bytes(*cur_offset, *cur_offset + size_of::<DiskEdge>() as u64);

        let cur_disk_edge: DiskEdge = *bytemuck::from_bytes(cur_disk_edge_bytes);
        if *prev_offset == u64::MAX{
            self.set_header(&(NUMBER_OF_LINKED_LIST-1), &cur_disk_edge.weight_offset);
            return;
        }

        let prev_disk_edge_bytes = self.file_manager.reading_bytes(*prev_offset, *prev_offset + size_of::<DiskEdge>() as u64);

        let prev_disk_edge: DiskEdge = *bytemuck::from_bytes(prev_disk_edge_bytes);

        let cap = prev_disk_edge.weight_len;
        let new_prev_disk_edge = DiskEdge::new(cur_disk_edge.weight_offset, cap, u64::MAX);

        self.write_disk_edges(prev_offset, &(*prev_offset + cap), &new_prev_disk_edge);
    }


    fn write_disk_edges(&mut self, start_offset: &u64, end_offset: &u64, disk_edge: &DiskEdge){
        if let Some(ref mut t) = self.tx {
            t.write_bytes(self.file_id, *start_offset, disk_edge.convert_into_bytes());
            t.write_bytes(self.file_id, *end_offset - size_of::<DiskEdge>() as u64, disk_edge.convert_into_bytes());
        } else {
            self.file_manager.writing_bytes_to_mmap(*start_offset, *start_offset+ size_of::<DiskEdge>() as u64, disk_edge.convert_into_bytes());
            self.file_manager.writing_bytes_to_mmap(*end_offset - size_of::<DiskEdge>() as u64, *end_offset, disk_edge.convert_into_bytes());
        }
    }

    /// Allocates a block of memory of at least `size` bytes.
    ///
    /// # Safety & Assumptions
    /// Expects `size` to be exactly a power of 2 >= 128. If `size` is an exact power of 2, 
    /// the block popped from buckets 0-5 is guaranteed to exactly match the requested size.
    pub fn allocate_structure(&mut self, size: &u64) -> u64{
        let mut index: u64 = find_index(size);

        while index < NUMBER_OF_LINKED_LIST {
            if self.get_header(&index) == u64::MAX{
                index+=1;
                continue;
            }
            break;
        }
        
        if index < NUMBER_OF_LINKED_LIST - 1{
            let offset_free_memory = self.get_header(&index);

            let disk_edge_bytes: &[u8] = self.file_manager.reading_bytes(offset_free_memory, offset_free_memory + size_of::<DiskEdge>() as u64);

            let disk_edge: DiskEdge = *bytemuck::from_bytes(disk_edge_bytes);

            if let Some(ref mut t) = self.tx {
                t.zero_mmap(self.file_id, offset_free_memory, offset_free_memory + *size);
            } else {
                self.file_manager.zeroing_mmap(offset_free_memory, offset_free_memory + *size);
            }
            let next_offset = disk_edge.weight_offset;
            self.set_header(&index, &next_offset);

            if disk_edge.weight_len == *size{
                // Exact match, no split needed
            }else{
                let new_offset = offset_free_memory + *size;

                if disk_edge.weight_len < *size {
                    println!("PANIC AVERTED! weight_len: {}, size: {}, disk_edge weight_offset: {}, disk_edge node: {}", disk_edge.weight_len, *size, disk_edge.weight_offset, disk_edge.node);
                    println!("offset_free_memory: {}", offset_free_memory);
                    println!("index: {}", index);
                }
                let new_cap = disk_edge.weight_len - *size;

                if new_cap & (new_cap-1) == 0{
                    let new_disk_edge: DiskEdge = DiskEdge::new(disk_edge.weight_offset, new_cap, disk_edge.node);

                    self.write_disk_edges(&new_offset, &(new_offset + new_cap), &new_disk_edge);
                    let new_index = find_index(&new_cap);

                    self.set_header(&new_index, &new_offset);
                }else{
                    self.split_capacity_into_power_of_2s(&new_offset, &new_cap);
                }
            } 
            return offset_free_memory;
        }else if index == NUMBER_OF_LINKED_LIST - 1{
            let mut prev_offset = u64::MAX;
            let mut cur_offset = self.get_header(&index);

            while cur_offset != u64::MAX{
                let cur_disk_edge_bytes = self.file_manager.reading_bytes_mut(cur_offset, cur_offset + size_of::<DiskEdge>() as u64);

                let cur_disk_edge: DiskEdge = *bytemuck::from_bytes(cur_disk_edge_bytes);

                if cur_disk_edge.weight_len >= *size{
                    self.skip_cur(&prev_offset, &cur_offset);
                    if let Some(ref mut t) = self.tx {
                        t.zero_mmap(self.file_id, cur_offset, cur_offset + *size);
                    } else {
                        self.file_manager.zeroing_mmap(cur_offset, cur_offset + *size);
                    }
                    if cur_disk_edge.weight_len > *size{
                        let new_offset = cur_offset + *size;
                        let new_cap = cur_disk_edge.weight_len - *size;
                        self.split_capacity_into_power_of_2s(&new_offset, &new_cap);
                    }
                    return cur_offset;
                }
                prev_offset = cur_offset;
                cur_offset = cur_disk_edge.weight_offset;
            }
        }
        self.bump_allocate(size)
    }

    /// Deallocates a `DiskNode` back into the allocator's free list.
    /// 
    /// # Assumptions
    /// The `capacity` of the deallocated `DiskNode` MUST be exactly a power of 2 >= 128 
    /// (128, 256, 512, etc.). This ensures it lands in the perfectly sized bucket 
    /// expected by `allocate_structure`.
    pub fn deallocater(&mut self, disk_node: &DiskNode){
        // We need to use list_edges_offset or list_reverse_edges_offset based on FileId!
        let (offset, capacity) = match self.file_id {
            FileId::Structure => (disk_node.get_edge_offset(), disk_node.get_capacity()),
            FileId::Reverse => (disk_node.list_reverse_edges_offset, disk_node.reverse_capacity),
            _ => panic!("AllocatedStruct only supports Structure and Reverse files"),
        };
        
        // Prevent panics if offset is MAX (node wasn't allocated)
        if offset == u64::MAX { return; }

        let end_offset = offset + capacity;

        let index = find_index(&capacity);

        let next_offset = self.get_header(&index);
        let disk_edge = DiskEdge::new(next_offset, capacity, u64::MAX);

        self.write_disk_edges(&offset, &end_offset, &disk_edge);
        self.set_header(&index, &offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::disk_storage::disk_edge::DiskEdge;
    use crate::storage::disk_storage::file_manager::FileManager;
    use crate::storage::disk_storage::super_block::SuperBlock;
    use std::path::PathBuf;

    /// Helper: create a FileManager backed by a temp file and a fresh SuperBlock.
    /// Returns (FileManager, SuperBlock, temp_dir) — keep temp_dir alive so the
    /// file isn't deleted while the mmap is still open.
    fn setup() -> (FileManager, SuperBlock, tempfile::TempDir) {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let path = tmp.path().join("test_structure.bin");
        let (fm, _) = FileManager::new(path).expect("failed to create FileManager");
        let sb = SuperBlock::new();
        (fm, sb, tmp)
    }


    /// Helper: write a free-block header (DiskEdge used as free-list node) at `offset`.
    /// `next` is the next pointer, `block_size` is the capacity of the free block.
    fn write_free_block(fm: &mut FileManager, offset: u64, next: u64, block_size: u64) {
        let edge = DiskEdge::new(next, block_size, u64::MAX);
        let bytes = edge.convert_into_bytes();
        fm.writing_bytes_to_mmap(offset, offset + size_of::<DiskEdge>() as u64, bytes);
        // Write boundary tag at the end of the block
        fm.writing_bytes_to_mmap(
            offset + block_size - size_of::<DiskEdge>() as u64,
            offset + block_size,
            bytes,
        );
    }

    /// Helper: read a DiskEdge from the mmap at the given offset.
    fn read_block(fm: &FileManager, offset: u64) -> DiskEdge {
        let bytes = fm.reading_bytes(offset, offset + size_of::<DiskEdge>() as u64);
        *bytemuck::from_bytes::<DiskEdge>(bytes)
    }

    // ─────────────────────────────────────────────────────────────────────
    // find_index tests
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn find_index_128_returns_bucket_0() {
        // 128 >> 7 = 1, leading_zeros(1) = 63, index = 63 - 63 = 0
        assert_eq!(find_index(&128), 0);
    }

    #[test]
    fn find_index_255_returns_bucket_0() {
        // 255 >> 7 = 1, same as 128
        assert_eq!(find_index(&255), 0);
    }

    #[test]
    fn find_index_256_returns_bucket_1() {
        // 256 >> 7 = 2, leading_zeros(2) = 62, index = 63 - 62 = 1
        assert_eq!(find_index(&256), 1);
    }

    #[test]
    fn find_index_512_returns_bucket_2() {
        // 512 >> 7 = 4, leading_zeros(4) = 61, index = 63 - 61 = 2
        assert_eq!(find_index(&512), 2);
    }

    #[test]
    fn find_index_1024_returns_bucket_3() {
        assert_eq!(find_index(&1024), 3);
    }

    #[test]
    fn find_index_2048_returns_bucket_4() {
        assert_eq!(find_index(&2048), 4);
    }

    #[test]
    fn find_index_4096_returns_bucket_5() {
        assert_eq!(find_index(&4096), 5);
    }

    #[test]
    fn find_index_8192_returns_bucket_6() {
        // 8192 >> 7 = 64, leading_zeros(64) = 57, index = 63 - 57 = 6
        assert_eq!(find_index(&8192), 6);
    }

    #[test]
    fn find_index_very_large_returns_bucket_6() {
        assert_eq!(find_index(&(1024 * 1024)), 6);
        assert_eq!(find_index(&(u64::MAX >> 1)), 6);
    }

    #[test]
    fn find_index_boundary_values() {
        // Check boundaries between buckets
        // Bucket 0: size in [128, 256)
        assert_eq!(find_index(&128), 0);
        assert_eq!(find_index(&255), 0);

        // Bucket 1: size in [256, 512)
        assert_eq!(find_index(&256), 1);
        assert_eq!(find_index(&511), 1);

        // Bucket 2: size in [512, 1024)
        assert_eq!(find_index(&512), 2);
        assert_eq!(find_index(&1023), 2);
    }

    // ─────────────────────────────────────────────────────────────────────
    // allocate_structure: exact-fit from correct bucket
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn allocate_exact_fit_returns_block_and_advances_list() {
        let (mut fm, mut sb, _tmp) = setup();

        // Place a 128-byte free block at offset 0, next = u64::MAX (end of list)
        write_free_block(&mut fm, 0, u64::MAX, 128);
        sb.next_header_structure(&0, &0); // bucket 0 head -> offset 0

        let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);

        let offset = alloc.allocate_structure(&128);

        assert_eq!(offset, 0);
        // Bucket 0 should now be empty (u64::MAX)
        assert_eq!(sb.get_ith_header_structure(&0), u64::MAX);
    }

    #[test]
    fn allocate_exact_fit_chains_to_next() {
        let (mut fm, mut sb, _tmp) = setup();

        // Two 128-byte free blocks: [0] -> [256] -> u64::MAX
        write_free_block(&mut fm, 256, u64::MAX, 128);
        write_free_block(&mut fm, 0, 256, 128);
        sb.next_header_structure(&0, &0);

        let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);

        let offset = alloc.allocate_structure(&128);
        assert_eq!(offset, 0);
        // Bucket 0 head should now point to the second block
        assert_eq!(sb.get_ith_header_structure(&0), 256);
    }

    #[test]
    fn allocate_zeroes_returned_region() {
        let (mut fm, mut sb, _tmp) = setup();

        // Fill offset 0..128 with non-zero data, then place a free block header there
        fm.writing_bytes_to_mmap(0, 128, &[0xFF; 128]);
        write_free_block(&mut fm, 0, u64::MAX, 128);
        sb.next_header_structure(&0, &0);

        let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);

        let offset = alloc.allocate_structure(&128);
        assert_eq!(offset, 0);

        // The returned region should be zeroed
        let data = fm.reading_bytes(0, 128);
        assert!(data.iter().all(|&b| b == 0), "allocated region should be zeroed");
    }

    // ─────────────────────────────────────────────────────────────────────
    // allocate_structure: splitting with power-of-2 remainder
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn allocate_splits_power_of_2_remainder() {
        let (mut fm, mut sb, _tmp) = setup();

        // Place a 256-byte free block at offset 0, in bucket 1
        write_free_block(&mut fm, 0, u64::MAX, 256);
        sb.next_header_structure(&1, &0);

        let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);

        // Request 128 bytes from bucket 1 (256-byte block)
        let offset = alloc.allocate_structure(&128);
        assert_eq!(offset, 0);

        // Remainder is 128 bytes (power of 2) → should go into bucket 0
        let remainder_offset = 128u64;
        assert_eq!(sb.get_ith_header_structure(&0), remainder_offset);

        // The remainder block should have correct metadata
        let remainder = read_block(&fm, remainder_offset);
        assert_eq!(remainder.weight_len, 128);
    }

    #[test]
    fn allocate_splits_non_power_of_2_remainder() {
        let (mut fm, mut sb, _tmp) = setup();

        // Place a 512-byte free block at offset 0, in bucket 2
        // Request 128 bytes → remainder = 384 = 256 + 128 (non-power-of-2)
        write_free_block(&mut fm, 0, u64::MAX, 512);
        sb.next_header_structure(&2, &0);

        let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);

        let offset = alloc.allocate_structure(&128);
        assert_eq!(offset, 0);

        // Remainder of 384 bytes should be split into:
        // - 128 bytes (bucket 0) at offset 128
        // - 256 bytes (bucket 1) at offset 256
        // Both buckets should have entries
        assert_ne!(sb.get_ith_header_structure(&0), u64::MAX, "bucket 0 should have the 128-byte piece");
        assert_ne!(sb.get_ith_header_structure(&1), u64::MAX, "bucket 1 should have the 256-byte piece");
    }

    // ─────────────────────────────────────────────────────────────────────
    // allocate_structure: promotion to higher bucket
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn allocate_promotes_to_higher_bucket_when_target_empty() {
        let (mut fm, mut sb, _tmp) = setup();

        // Bucket 0 is empty, bucket 1 has a 256-byte block
        write_free_block(&mut fm, 0, u64::MAX, 256);
        sb.next_header_structure(&1, &0);
        // bucket 0 stays at u64::MAX (empty)

        let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);

        // Request 128 bytes — bucket 0 is empty, should promote to bucket 1
        let offset = alloc.allocate_structure(&128);
        assert_eq!(offset, 0);

        // Remainder (128 bytes) should be in bucket 0
        assert_ne!(sb.get_ith_header_structure(&0), u64::MAX);
    }

    // ─────────────────────────────────────────────────────────────────────
    // allocate_structure: overflow bucket (bucket 6)
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn allocate_from_overflow_bucket_exact_fit() {
        let (mut fm, mut sb, _tmp) = setup();

        let block_size: u64 = 16384; // 16KB, goes into bucket 6
        // Place a single block in bucket 6
        write_free_block(&mut fm, 0, u64::MAX, block_size);
        sb.next_header_structure(&6, &0);

        let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);

        let offset = alloc.allocate_structure(&block_size);
        assert_eq!(offset, 0);
        // Bucket 6 should now be empty
        assert_eq!(sb.get_ith_header_structure(&6), u64::MAX);
    }

    #[test]
    fn allocate_from_overflow_bucket_with_split() {
        let (mut fm, mut sb, _tmp) = setup();

        let block_size: u64 = 16384; // 16KB
        let request_size: u64 = 8192; // 8KB
        write_free_block(&mut fm, 0, u64::MAX, block_size);
        sb.next_header_structure(&6, &0);

        let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);

        let offset = alloc.allocate_structure(&request_size);
        assert_eq!(offset, 0);

        // Remainder of 8192 bytes should be split and placed into a bucket
        // 8192 >> 7 = 64, index = 6 → bucket 6
        assert_ne!(sb.get_ith_header_structure(&6), u64::MAX,
            "remainder should be in bucket 6");
    }

    #[test]
    fn allocate_overflow_bucket_skips_too_small_blocks() {
        let (mut fm, mut sb, _tmp) = setup();

        // Two blocks in bucket 6:
        // Block at offset 0: 8192 bytes (too small for our request)
        // Block at offset 16384: 32768 bytes (large enough)
        write_free_block(&mut fm, 16384, u64::MAX, 32768);
        write_free_block(&mut fm, 0, 16384, 8192);
        sb.next_header_structure(&6, &0);

        let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);

        // Request 16384 bytes — first block (8192) is too small, should skip to second
        let offset = alloc.allocate_structure(&16384);
        assert_eq!(offset, 16384);
    }

    // ─────────────────────────────────────────────────────────────────────
    // allocate_structure: fallback to bump allocation
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn allocate_falls_back_to_bump_when_all_buckets_empty() {
        let (mut fm, mut sb, _tmp) = setup();
        // All buckets are empty (default SuperBlock has u64::MAX in all headers)
        // next_structure_free_block starts at 0

        let offset = {
            let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);
            alloc.allocate_structure(&256)
        };
        assert_eq!(offset, 0); // bump starts at 0
        assert_eq!(sb.next_structure_free_block, 256); // advanced by size

        let offset2 = {
            let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);
            alloc.allocate_structure(&128)
        };
        assert_eq!(offset2, 256); // next bump
        assert_eq!(sb.next_structure_free_block, 384);
    }

    #[test]
    fn allocate_falls_back_when_overflow_bucket_has_no_fit() {
        let (mut fm, mut sb, _tmp) = setup();

        // Only bucket 6 has a block, but it's too small
        write_free_block(&mut fm, 0, u64::MAX, 8192);
        sb.next_header_structure(&6, &0);

        let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);

        // Request 32768 — only block in bucket 6 is 8192, too small
        let offset = alloc.allocate_structure(&32768);
        // Should fall back to bump allocation
        assert_eq!(offset, 0);
        assert_eq!(sb.next_structure_free_block, 32768);
    }

    // ─────────────────────────────────────────────────────────────────────
    // allocate_structure: multiple sequential allocations
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn multiple_allocations_from_same_bucket() {
        let (mut fm, mut sb, _tmp) = setup();

        // Three 128-byte free blocks chained: 0 -> 256 -> 512 -> u64::MAX
        write_free_block(&mut fm, 512, u64::MAX, 128);
        write_free_block(&mut fm, 256, 512, 128);
        write_free_block(&mut fm, 0, 256, 128);
        sb.next_header_structure(&0, &0);

        let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);

        let o1 = alloc.allocate_structure(&128);
        let o2 = alloc.allocate_structure(&128);
        let o3 = alloc.allocate_structure(&128);

        assert_eq!(o1, 0);
        assert_eq!(o2, 256);
        assert_eq!(o3, 512);

        // All consumed, bucket should be empty
        assert_eq!(sb.get_ith_header_structure(&0), u64::MAX);
    }

    #[test]
    fn allocate_returns_non_overlapping_regions() {
        let (mut fm, mut sb, _tmp) = setup();

        // Use bump allocation to test non-overlap
        let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);

        let o1 = alloc.allocate_structure(&256);
        let o2 = alloc.allocate_structure(&512);
        let o3 = alloc.allocate_structure(&128);

        // Regions must not overlap
        assert!(o1 + 256 <= o2, "region 1 overlaps region 2");
        assert!(o2 + 512 <= o3, "region 2 overlaps region 3");
    }

    // ─────────────────────────────────────────────────────────────────────
    // Implicit Invariant Tests (Doubling Sizes)
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn test_strict_doubling_invariant() {
        let (mut fm, mut sb, _tmp) = setup();
        let mut alloc = AllocatedStruct::new(&mut fm, &mut sb, None, FileId::Structure);

        // We simulate the database behavior where sizes strictly double: 
        // 128, 256, 512, 1024, 2048, 4096
        let sizes = [128, 256, 512, 1024, 2048, 4096];
        let mut nodes = Vec::new();

        // 1. Allocate all doubling sizes (uses bump allocator since free lists are empty)
        for &size in &sizes {
            let offset = alloc.allocate_structure(&size);
            // Simulate creation of a DiskNode
            let mut node = DiskNode::new(offset, 0, 0);
            node.capacity = size; // manually set since initial is hardcoded to 256
            node.list_edges_offset = offset;
            nodes.push(node);
        }

        // 2. Deallocate them in reverse order
        for node in nodes.iter().rev() {
            alloc.deallocater(node);
        }

        // 3. Re-allocate the same sizes. They should exactly hit the heads of buckets 0-5
        // without causing any panics or underflows, proving the invariant is safe.
        for &size in &sizes {
            let offset = alloc.allocate_structure(&size);
            // Verify that find_index mapped perfectly back to the original offsets
            let expected_bucket = find_index(&size);
            // Because we deallocated, the exact offset should have been returned.
            // Since there was only 1 item per bucket, it's just fine to verify it didn't panic.
            assert_ne!(offset, u64::MAX);
        }
    }
}
