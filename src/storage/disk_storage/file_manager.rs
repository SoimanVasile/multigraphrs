use std::{fs::OpenOptions, path::PathBuf};
use memmap2::MmapMut;
use memmap2::MmapOptions;

const FILE_INITIAL_SIZE: u64 = 1024 * 1024 * 64;

const FOUR_GB: u64 = 1024 * 1024 * 1024 * 4;

#[derive(Debug)]
pub struct FileManager{
    mmap: MmapMut,
    file: std::fs::File,
}

impl FileManager{
    pub fn new(file_path: PathBuf) -> Result<(Self, bool), std::io::Error>{
        let mut created = false;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
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
    /// # Panics
    /// Panics if `start > end` or if `ends` exceeds the actual length of the memory map or if the length
    /// of the bytes is different from the range
    pub fn writing_bytes_to_mmap(&mut self, start: u64, end: u64,  bytes: &[u8]){
        self.mmap[start as usize .. end as usize].copy_from_slice(bytes);
    }

    pub fn reading_bytes(&self, start: u64, end: u64) -> &[u8]{
        &self.mmap[start as usize .. end as usize]
    }

    pub fn copy_within(&mut self, src_start: u64, src_end: u64, dest_start: u64){
        self.mmap.copy_within(src_start as usize .. src_end as usize, dest_start as usize);
    }

    pub fn increase_file_size(&mut self) -> Result<(), std::io::Error>{
        let mut length = self.file.metadata()?.len();
        if length >= FOUR_GB{
            length+= FOUR_GB;
        }else{
            length *= 2;
        }
        self.file.set_len(length)?;

        self.mmap = unsafe{
            MmapOptions::new()
                .map_mut(&self.file)?
        };
        Ok(())
    }

    pub fn file_len(&self) -> Result<u64, std::io::Error>{
        // We can just return the length of the memory map, which is identical to the file's length.
        // This avoids making a statx syscall to the OS.
        Ok(self.mmap.len() as u64)
    }
}
