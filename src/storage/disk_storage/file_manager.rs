use std::{fs::OpenOptions, path::PathBuf};
use memmap2::MmapMut;
use memmap2::MmapOptions;
use crate::core::db_error::DbError;


const FILE_INITIAL_SIZE: u64 = 1024 * 1024 * 64;

const FOUR_GB: u64 = 1024 * 1024 * 1024 * 4;

#[derive(Debug)]
pub struct FileManager{
    mmap: MmapMut,
    file: std::fs::File,
}

impl FileManager{
    /// Creates a new `FileManager` instance for the given file path.
    ///
    /// # Side Effects
    /// Creates the file if it does not exist. Modifies the file length on disk if it is empty.
    /// Maps the file into memory.
    ///
    /// # Errors
    /// Returns an `std::io::Error` if file operations (open, metadata, set_len, map_mut) fail.
    ///
    pub fn new(file_path: PathBuf) -> Result<(Self, bool), DbError>{
        let mut created = false;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(file_path)?;

        if file.metadata().map(|m| m.len()).unwrap_or(0) == 0{
            file.set_len(FILE_INITIAL_SIZE)?;
            created = true
        }
        let mmap = unsafe{
            MmapOptions::new()
                .map_mut(&file)?
        };

        Ok((Self{file, mmap}, created))
    }

    
    /// Fill the range [start,end) with zeros in the memory map
    ///
    /// This is a convenience wrapper around [`slice::fill`]
    /// 
    /// # Arguments
    /// * `start` - The starting byte  offset (inclusive)
    /// * `end` - The end byte offset (exclusive)
    ///
    /// # Side Effects
    /// Modifies the contents of the memory-mapped file, which will eventually be flushed to disk.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if `start > end` or if `ends` exceeds the actual length of the memory map.
    pub fn zeroing_mmap(&mut self, start: u64, end: u64){
        self.mmap[start as usize .. end as usize].fill(0);
    }



    /// Fill the range [start,end) with the bytes given in the memory map
    ///
    /// This is a convenience wrapper around [`slice::copy_from_slice`]
    /// 
    /// # Arguments
    /// * `start` - The starting byte  offset (inclusive)
    /// * `end` - The end byte offset (exclusive)
    /// * `bytes` - The raw data to be written
    ///
    /// # Side Effects
    /// Modifies the contents of the memory-mapped file, which will eventually be flushed to disk.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if `start > end` or if `ends` exceeds the actual length of the memory map or if the length
    /// of the bytes is different from the range
    pub fn writing_bytes_to_mmap(&mut self, start: u64, end: u64,  bytes: &[u8]){
        self.mmap[start as usize .. end as usize].copy_from_slice(bytes);
    }

    /// Reads a slice of bytes from the memory map.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if the range `[start, end)` is out of bounds for the memory map.
    pub fn reading_bytes(&self, start: u64, end: u64) -> &[u8]{
        &self.mmap[start as usize .. end as usize]
    }

    /// Reads a mutable slice of bytes from the memory map.
    ///
    /// # Side Effects
    /// Returns a mutable slice that can modify the memory-mapped file.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if the range `[start, end)` is out of bounds for the memory map.
    pub fn reading_bytes_mut(&mut self, start: u64, end: u64) -> &mut [u8] {
        &mut self.mmap[start as usize .. end as usize]
    }

    /// Returns a raw mutable pointer to the underlying memory map.
    ///
    /// # Side Effects
    /// None directly, but provides access that can mutate the memory-mapped file.
    ///
    /// # Errors
    /// None.
    ///
    pub fn mmap_ptr_mut(&mut self) -> *mut u8 {
        self.mmap.as_mut_ptr()
    }



    /// Copies data within the memory map from a source range to a destination offset.
    ///
    /// # Side Effects
    /// Modifies the contents of the memory-mapped file, which will eventually be flushed to disk.
    ///
    /// # Errors
    /// None.
    ///
    /// # Panics
    /// Panics if either the source range or the destination range is out of bounds.
    pub fn copy_within(&mut self, src_start: u64, src_end: u64, dest_start: u64){
        self.mmap.copy_within(src_start as usize .. src_end as usize, dest_start as usize);
    }

    /// Increases the size of the underlying file.
    ///
    /// # Side Effects
    /// Modifies the file size on disk and re-establishes the memory map.
    ///
    /// # Errors
    /// Returns `std::io::Error` if file metadata cannot be read, resizing fails, or mapping fails.
    ///
    pub fn increase_file_size(&mut self) -> Result<(), DbError>{
        let length = self.check_next_size(self.file_len()?)?;
        self.file.set_len(length)?;

        self.mmap = unsafe{
            MmapOptions::new()
                .map_mut(&self.file)?
        };
        Ok(())
    }

    pub fn check_next_size(&self, length: u64) -> Result<u64, DbError>{

        if length >= FOUR_GB{
            Ok(length + FOUR_GB)
        }else{
            Ok(length * 2)
        }
    }

    /// Gets the current length of the file/memory map.
    ///
    /// # Errors
    /// Returns `std::io::Error` (though currently infallible) for API consistency.
    ///
    pub fn file_len(&self) -> Result<u64, DbError>{
        // We can just return the length of the memory map, which is identical to the file's length.
        // This avoids making a statx syscall to the OS.
        Ok(self.mmap.len() as u64)
    }

    /// Flushes the memory map changes to disk asynchronously.
    ///
    /// # Side Effects
    /// Forces the OS to synchronize memory map modifications to the underlying storage.
    ///
    /// # Errors
    /// Returns `std::io::Error` if the flush operation fails.
    ///
    pub fn flush(&self) -> Result<(), DbError> {
        Ok(self.mmap.flush()?)
    }
}
