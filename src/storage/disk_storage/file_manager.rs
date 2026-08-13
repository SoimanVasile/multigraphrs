use std::sync::{Arc, RwLock};
use std::{fs::OpenOptions, path::PathBuf};
use memmap2::MmapMut;
use memmap2::MmapOptions;
use crate::core::db_error::DbError;


const FILE_INITIAL_SIZE: u64 = 1024 * 1024 * 64;

const FOUR_GB: u64 = 1024 * 1024 * 1024 * 4;

#[derive(Debug)]
pub struct FileManager{
    mmap: Arc<RwLock<MmapMut>>,
    file: std::fs::File,
}

impl FileManager{
    /// Creates a new `FileManager` instance for the given file path. Creates the file if it does not exist, modifies the file length on disk if it is empty, and maps the file into memory.
    ///
    /// `file_path` is needed to specify the location of the file to open or create.
    ///
    /// # Errors
    /// Returns a [`DbError`] (wrapping `std::io::Error`) if file operations (open, metadata, set_len, map_mut) fail.
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

        Ok((Self{file, mmap: Arc::new(RwLock::new(mmap))}, created))
    }

    
    /// Fill the range [start,end) with zeros in the memory map, which will eventually be flushed to disk.
    ///
    /// This is a convenience wrapper around [`slice::fill`]
    /// 
    /// `start` is needed to specify the starting byte offset (inclusive).
    /// `end` is needed to specify the end byte offset (exclusive).
    ///
    /// # Panics
    /// Panics if `start > end` or if `end` exceeds the actual length of the memory map.
    pub fn zeroing_mmap(&mut self, start: u64, end: u64){
        let mut mmap = self.mmap.write().unwrap();
        mmap[start as usize .. end as usize].fill(0);
    }



    /// Fill the range [start,end) with the bytes given in the memory map, which will eventually be flushed to disk.
    ///
    /// This is a convenience wrapper around [`slice::copy_from_slice`]
    /// 
    /// `start` is needed to specify the starting byte offset (inclusive).
    /// `end` is needed to specify the end byte offset (exclusive).
    /// `bytes` is needed to provide the raw data to be written.
    ///
    /// # Panics
    /// Panics if `start > end` or if `end` exceeds the actual length of the memory map or if the length
    /// of the bytes is different from the range
    pub fn writing_bytes_to_mmap(&mut self, start: u64, end: u64,  bytes: &[u8]){
        let mut mmap = self.mmap.write().unwrap();
        mmap[start as usize .. end as usize].copy_from_slice(bytes);
    }

    /// Reads a slice of bytes from the memory map.
    ///
    /// `start` is needed to specify the starting byte offset (inclusive).
    /// `end` is needed to specify the end byte offset (exclusive).
    ///
    /// # Panics
    /// Panics if the range `[start, end)` is out of bounds for the memory map.
    pub fn reading_bytes(&self, start: u64, end: u64) -> &[u8]{
        let mmap = self.mmap.read().unwrap();
        // To return a slice that outlives the RwLockReadGuard, we must use a raw pointer.
        // This is safe as long as no other thread resizes the mmap while this slice is being read.
        unsafe {
            let ptr = mmap.as_ptr().add(start as usize);
            std::slice::from_raw_parts(ptr, (end - start) as usize)
        }
    }

    /// Reads a mutable slice of bytes from the memory map that can modify the memory-mapped file.
    ///
    /// `start` is needed to specify the starting byte offset (inclusive).
    /// `end` is needed to specify the end byte offset (exclusive).
    ///
    /// # Panics
    /// Panics if the range `[start, end)` is out of bounds for the memory map.
    pub fn reading_bytes_mut(&mut self, start: u64, end: u64) -> &mut [u8] {
        let mut mmap = self.mmap.write().unwrap();
        unsafe {
            let ptr = mmap.as_mut_ptr().add(start as usize);
            std::slice::from_raw_parts_mut(ptr, (end - start) as usize)
        }
    }

    /// Returns a raw mutable pointer to the underlying memory map. Provides access that can mutate the memory-mapped file.
    pub fn mmap_ptr_mut(&mut self) -> *mut u8 {
        let mut mmap = self.mmap.write().unwrap();
        mmap.as_mut_ptr()
    }



    /// Copies data within the memory map from a source range to a destination offset, which will eventually be flushed to disk.
    ///
    /// `src_start` is needed to specify the starting byte offset of the source data (inclusive).
    /// `src_end` is needed to specify the end byte offset of the source data (exclusive).
    /// `dest_start` is needed to specify the starting byte offset of the destination.
    ///
    /// # Panics
    /// Panics if either the source range or the destination range is out of bounds.
    pub fn copy_within(&mut self, src_start: u64, src_end: u64, dest_start: u64){
        let mut mmap = self.mmap.write().unwrap();
        mmap.copy_within(src_start as usize .. src_end as usize, dest_start as usize);
    }

    /// Increases the size of the underlying file. Modifies the file size on disk and re-establishes the memory map.
    ///
    /// # Errors
    /// Returns a [`DbError`] if file metadata cannot be read, resizing fails, or mapping fails.
    pub fn increase_file_size(&mut self) -> Result<(), DbError>{
        let length = self.check_next_size(self.file_len()?)?;        
        self.file.set_len(length)?;

        let mut mmap = self.mmap.write().unwrap();
        *mmap = unsafe{
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
    /// Returns a [`DbError`] (though currently infallible) for API consistency.
    pub fn file_len(&self) -> Result<u64, DbError>{
        // We can just return the length of the memory map, which is identical to the file's length.
        // This avoids making a statx syscall to the OS.
        let mmap = self.mmap.read().unwrap();
        Ok(mmap.len() as u64)
    }

    /// Flushes the memory map changes to disk asynchronously. Forces the OS to synchronize memory map modifications to the underlying storage.
    ///
    /// # Errors
    /// Returns a [`DbError`] if the flush operation fails.
    pub fn flush(&self) -> Result<(), DbError> {
        let mmap = self.mmap.read().unwrap();
        Ok(mmap.flush()?)
    }
}
