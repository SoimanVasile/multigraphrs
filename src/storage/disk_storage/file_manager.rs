use std::io::Write;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::{fs::OpenOptions, path::PathBuf};
use memmap2::MmapMut;
use memmap2::MmapOptions;
use crate::core::db_error::DbError;
use crate::storage::disk_storage::from_disk_bytes::{AsDiskBytes, FromDiskBytes};


const FILE_INITIAL_SIZE: u64 = 1024 * 1024 * 64;

const FOUR_GB: u64 = 1024 * 1024 * 1024 * 4;

#[derive(Debug, Clone, Copy)]
struct ZeroCopyBytes{
    length: u64,
    raw_pointer: *const [u8]
}

unsafe impl Send for ZeroCopyBytes{}

impl ZeroCopyBytes{
    fn new(reference: &[u8]) -> Self{
        Self {length: reference.len() as u64, raw_pointer: reference as *const [u8]}
    }
}

#[derive(Debug, Clone)]
enum FMTypeRequest{
    Read(/*length*/ u64, /*buffer*/ Vec<u8>),
    Write(/*bytes*/ ZeroCopyBytes),
    IncreaseFileSize(u64),
    Flush,
}

enum FMTypeResponse
{
    Read(Vec<u8>),
    Write,
    IncreaseFileSize { cur_file_size: u64},
    Flush,
}

pub struct FMResponse
{
    status: Result<(), DbError>,
    _type: FMTypeResponse
}

impl<'a> FMResponse
{
    fn new(_type: FMTypeResponse, status: Result<(), DbError>) -> Self{
        Self {_type, status}
    }
}

pub struct FMRequest
    {
    _type: FMTypeRequest,
    offset: u64,
    sender: Sender<FMResponse>
}

impl<'a> FMRequest{
    fn new(_type: FMTypeRequest, offset: u64, sender: Sender<FMResponse>) -> Self{
        Self { _type, offset, sender }
    }
}


#[derive(Debug)]
pub struct FileManager{
    sender: Sender<FMRequest>,
    len: u64,
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
            .open(&file_path)?;

        if file.metadata().map(|m| m.len()).unwrap_or(0) == 0{
            file.set_len(FILE_INITIAL_SIZE)?;
            created = true
        }

        let (sender, request) = channel();

        thread::spawn(move || {
            file_manager_worker_thread(file_path, request);
        });

        Ok((Self{sender, len: file.metadata().unwrap().len()}, created))
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
    pub fn zeroing_mmap(&mut self, start: u64, end: u64) -> Result<(), DbError>{
        self.writing_bytes_to_mmap(start, end, &vec![0; (end-start) as usize])
        // self.mmap[start as usize .. end as usize].fill(0);
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
    pub fn writing_bytes_to_mmap(&mut self, start: u64, _end: u64,  bytes: &[u8]) -> Result<(), DbError>{
        let (send, receiv) = channel();
        let pointer = ZeroCopyBytes::new(bytes);
        let _type = FMTypeRequest::Write(pointer);
        let request = FMRequest::new(_type, start, send);

        self.sender.send(request).map_err(|_| {
            DbError::WalThreadDead
        })?;

        let res = receiv.recv().map_err(|_|{
            DbError::WalThreadDead
        })?;

        if let Err(e) = res.status {
            Err(e)
        } else{
            Ok(())
        }

    }

    /// Reads a slice of bytes from the memory map.
    ///
    /// `start` is needed to specify the starting byte offset (inclusive).
    /// `end` is needed to specify the end byte offset (exclusive).
    ///
    /// # Panics
    /// Panics if the range `[start, end)` is out of bounds for the memory map.
    pub fn reading_bytes(&self, start: u64, end: u64, mut buffer: Vec<u8>) -> Result<Vec<u8>, DbError>{
        buffer.clear();
        let (send, receiv) = channel();

        let _type = FMTypeRequest::Read(end - start, buffer);
        let req = FMRequest::new(_type, start, send);

        self.sender.send(req).map_err(|_|{
            DbError::WalThreadDead
        })?;

        let res = receiv.recv().map_err(|_|{
            DbError::WalThreadDead
        })?;

        if let Err(e) = res.status{
            Err(e)
        } else{
            Ok(match res._type{
                FMTypeResponse::Read(bytes) => {bytes}
                _ => {return Err(DbError::WalThreadDead)}
            })
        }
    }

    /// Reads a mutable slice of bytes from the memory map that can modify the memory-mapped file.
    ///
    /// `start` is needed to specify the starting byte offset (inclusive).
    /// `end` is needed to specify the end byte offset (exclusive).
    ///
    /// # Panics
    /// Panics if the range `[start, end)` is out of bounds for the memory map.
    // pub fn reading_bytes_mut(&mut self, start: u64, end: u64) -> &mut [u8] {
    //     todo!();
        // &mut self.mmap[start as usize .. end as usize]
    // }

    /// Copies data within the memory map from a source range to a destination offset, which will eventually be flushed to disk.
    ///
    /// `src_start` is needed to specify the starting byte offset of the source data (inclusive).
    /// `src_end` is needed to specify the end byte offset of the source data (exclusive).
    /// `dest_start` is needed to specify the starting byte offset of the destination.
    ///
    /// # Panics
    /// Panics if either the source range or the destination range is out of bounds.
    pub fn copy_within(&mut self, src_start: u64, src_end: u64, dest_start: u64){
        todo!()
        // self.mmap.copy_within(src_start as usize .. src_end as usize, dest_start as usize);
    }

    /// Increases the size of the underlying file. Modifies the file size on disk and re-establishes the memory map.
    ///
    /// # Errors
    /// Returns a [`DbError`] if file metadata cannot be read, resizing fails, or mapping fails.
    pub fn increase_file_size(&mut self) -> Result<u64, DbError>{
        let _type = FMTypeRequest::IncreaseFileSize(self.check_next_size(self.file_len()?)?);
        let (sender, recv) = channel();
        let request = FMRequest::new(_type, u64::MAX, sender);

        self.sender.send(request).map_err(|_| {
            DbError::WalThreadDead
        })?;

        let response = recv.recv()?;
        
        if let Err(e) = response.status{
            Err(e)
        }else{
            let cur_file_size = match response._type{
                FMTypeResponse::IncreaseFileSize { cur_file_size } => cur_file_size,
                _ => return Err(DbError::WalThreadDead),
            };
            self.len = cur_file_size;
            Ok(cur_file_size)
        }

        // let length = self.check_next_size(self.file_len()?)?;
        // self.file.set_len(length)?;
        //
        // self.mmap = unsafe{
        //     MmapOptions::new()
        //         .map_mut(&self.file)?
        // };
        // Ok(())
    }

    pub fn check_next_size(&self, length: u64) -> Result<u64, DbError>{

        if length >= FOUR_GB{
            Ok(length + FOUR_GB)
        }else{
            Ok(length * 2)
        }    }

    /// Gets the current length of the file/memory map.
    ///
    /// # Errors
    /// Returns a [`DbError`] (though currently infallible) for API consistency.
    pub fn file_len(&self) -> Result<u64, DbError>{
        Ok(self.len)
        // We can just return the length of the memory map, which is identical to the file's length.
        // This avoids making a statx syscall to the OS.
        // Ok(self.mmap.len() as u64)
    }

    /// Flushes the memory map changes to disk asynchronously. Forces the OS to synchronize memory map modifications to the underlying storage.
    ///
    /// # Errors
    /// Returns a [`DbError`] if the flush operation fails.
    pub fn flush(&self) -> Result<(), DbError> {
        let _type = FMTypeRequest::Flush;
        let (sender, recv) = channel();
        let request = FMRequest::new(_type, u64::MAX, sender);

        self.sender.send(request)?;


        let response = recv.recv()?;

        if let Err(e) = response.status{
            Err(e)
        } else{
            match response._type {
                FMTypeResponse::Flush => {},
                _ => return Err(DbError::WalThreadDead)
            }
            Ok(())
        }
    }
}

fn file_manager_worker_thread(file_path: PathBuf, rec: Receiver<FMRequest>){
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(file_path).unwrap();

    let mut mmap = unsafe {
        MmapOptions::new()
            .map_mut(&file)
    }.unwrap();


    loop{
        let mut requests: Vec<FMRequest> = Vec::with_capacity(1<<15);

        requests.push(rec.recv().unwrap());

        while let Ok(req) = rec.try_recv(){
            requests.push(req);
        }

        for req in requests.into_iter(){
            match req._type{
                FMTypeRequest::Read(length, mut buffer) => {
                    let offset = req.offset;
                    buffer.extend_from_slice(&mmap[offset as usize .. (offset + length) as usize]);
                    req.sender.send(FMResponse::new(FMTypeResponse::Read(buffer), Ok(()))).unwrap();
                },
                FMTypeRequest::Write(pointer) => {
                    let offset = req.offset;
                    let reference = unsafe { &pointer.raw_pointer.as_ref().unwrap()};
                    mmap[offset as usize .. offset as usize + pointer.length as usize].copy_from_slice(reference);
                    req.sender.send(FMResponse::new(FMTypeResponse::Write, Ok(()))).unwrap();
                },
                FMTypeRequest::IncreaseFileSize(length) => {
                    drop(mmap);
                    file.set_len(length);

                    mmap = unsafe {
                        MmapOptions::new()
                            .map_mut(&file)
                    }.unwrap();

                    let _type = FMTypeResponse::IncreaseFileSize { cur_file_size: length };
                    let response = FMResponse::new(_type, Ok(()));
                    req.sender.send(response).unwrap();
                },
                FMTypeRequest::Flush => {
                    file.flush().unwrap();
                    let response = FMResponse::new(FMTypeResponse::Flush, Ok(()));
                    req.sender.send(response).unwrap();
                }
            }
        }
        
    }
}
