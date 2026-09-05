use std::fs::File;
use std::io::Error;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::{fs::OpenOptions, path::PathBuf};
use arc_swap::ArcSwap;
use memmap2::MmapMut;
use memmap2::MmapOptions;
use crate::core::db_error::DbError;


const FILE_INITIAL_SIZE: u64 = 1024 * 1024 * 64;

const FOUR_GB: u64 = 1024 * 1024 * 1024 * 4;


fn open_file(file_path: &PathBuf) -> Result<File, Error> {
    OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(file_path)
}

fn initialize_file(file: &File, created: &mut bool) -> Result<(), Error>{
    if file.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
            file.set_len(FILE_INITIAL_SIZE)?;
            *created = true;
    }

    Ok(())
}

fn create_mmap(file: &File) -> Result<MmapMut, Error>{
    unsafe {
        MmapOptions::new()
            .map_mut(file)
    }
}

/// This is a fat pointer for the write to be able to send to the worker thread as it doesnt use
/// lifetime
#[derive(Debug, Clone, Copy)]
struct ZeroCopyBytes{
    length: u64,
    raw_pointer: *const [u8]
}

unsafe impl Send for ZeroCopyBytes{}

impl ZeroCopyBytes{
    /// Creates the fat pointer from a reference so the pointer is non-null.
    ///
    /// # Safety
    ///
    /// With the current architecture the fat pointer is safe as when a thread send some bytes to
    /// write it needs to wait for the response so it cant drop the pointer
    fn new(reference: &[u8]) -> Self{
        Self {length: reference.len() as u64, raw_pointer: reference as *const [u8]}
    }
}


/// All the requests that a thread can send to the worker thread
#[derive(Debug, Clone)]
enum FMTypeRequest{

    /// It needs the raw pointer to the bytes to be able to write them into the file and right now
    /// this operation is safe
    Write(ZeroCopyBytes),

    /// Request to increease the file size and it gives the length of the file to be next
    IncreaseFileSize(u64),

    /// The request to flush the file when we want to rotate the wal bin and be sure that everything
    /// got updated
    Flush,
}

/// The responses of the worker thread to their respetive requests
enum FMTypeResponse
{
    /// All the response dont need to give any information only the status that is found inside the
    /// [`FMResponse`] struct
    Write,
    IncreaseFileSize,
    Flush,
}

/// This struct will be send by worker thread after a request
pub struct FMResponse
{
    /// The statis is just for the information in case that an IO failed or other strange things and
    /// it doesnt need to return anything as this could be take care of by the type
    status: Result<(), DbError>,

    /// The response type given by the requester
    _type: FMTypeResponse
}

impl<'a> FMResponse
{
    /// A helper function to create a FMResponse easier
    fn new(_type: FMTypeResponse, status: Result<(), DbError>) -> Self{
        Self {_type, status}
    }
}


/// This struct will get the worker thread as a request getting the offset, type and the send
/// channel to be able to responde back
pub struct FMRequest
{
    _type: FMTypeRequest,
    offset: u64,

    /// This is the Sender which the worker thread will respond to give the status of the operation
    sender: Sender<FMResponse>
}

impl<'a> FMRequest{
    /// A helper function that just builds the FMRequest
    fn new(_type: FMTypeRequest, offset: u64, sender: Sender<FMResponse>) -> Self{
        Self { _type, offset, sender }
    }
}


#[derive(Debug)]
pub struct FileManager{
    sender: Sender<FMRequest>,
    len: u64,
    /// Is inside an Arc to give the mmap to the worker thread, but have a reference too to have
    /// instantenous reads
    mmap: Arc<ArcSwap<MmapMut>>,
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
        let file = open_file(&file_path)?;
        initialize_file(&file, &mut created)?;

        let mmap = create_mmap(&file)?;
        let shared_mmap = Arc::new(ArcSwap::new(Arc::new(mmap)));
        let thread_mmap = shared_mmap.clone();

        let (sender, request) = channel();

        thread::spawn(move || {
            file_manager_worker_thread(file_path, thread_mmap, request);
        });

            Ok((Self{sender, len: file.metadata().unwrap().len(), mmap: shared_mmap}, created))
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
        let request = FMRequest::new(
            FMTypeRequest::Write(ZeroCopyBytes::new(bytes)),
            start,
            send
        );

        self.sender.send(request)?;
        let res = receiv.recv()?;

        res.status
    }

    /// Reads a slice of bytes from the memory map.
    ///
    /// `start` is needed to specify the starting byte offset (inclusive).
    /// `end` is needed to specify the end byte offset (exclusive).
    ///
    /// # Panics
    /// Panics if the range `[start, end)` is out of bounds for the memory map.
    pub fn reading_bytes<F, R>(&self, start: u64, end: u64, parser: F) -> Result<R, DbError>
    where 
        F: FnOnce(&[u8]) -> R,
    {
        let guard = self.mmap.load();
        let bytes: &[u8] = &guard;

        let read_bytes = bytes.get(start as usize..end as usize)
            .ok_or(DbError::OutOfBoundIndexing {offset: start, len: end.saturating_sub(start)})?;
        Ok(parser(read_bytes))
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
    pub fn copy_within(&mut self, _src_start: u64, _src_end: u64, _dest_start: u64){
        todo!()
        // self.mmap.copy_within(src_start as usize .. src_end as usize, dest_start as usize);
    }

    /// Increases the size of the underlying file. Modifies the file size on disk and re-establishes the memory map.
    ///
    /// # Errors
    /// Returns a [`DbError`] if file metadata cannot be read, resizing fails, or mapping fails.
    pub fn increase_file_size(&mut self) -> Result<u64, DbError>{

        let (sender, recv) = channel();
        let next_length = self.check_next_size(self.file_len()?)?;

        let request = FMRequest::new(
            FMTypeRequest::IncreaseFileSize(next_length),
            u64::MAX,
            sender);

        self.sender.send(request)?;
        let response = recv.recv()?;
        
        response.status?;

        match response._type{
            FMTypeResponse::IncreaseFileSize  => {
                self.len = next_length;
                Ok(next_length)
            },
            _ => Err(DbError::WalThreadDead),
        }
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

        let (sender, recv) = channel();

        let request = FMRequest::new(
            FMTypeRequest::Flush,
            u64::MAX,
            sender);

        self.sender.send(request)?;
        let res = recv.recv()?;

        res.status
    }
}

fn file_manager_worker_thread(file_path: PathBuf, arc_mmap: Arc<ArcSwap<MmapMut>>, rec: Receiver<FMRequest>){

    let Ok(file) = open_file(&file_path) else {return;};

    let mut requests: Vec<FMRequest> = Vec::with_capacity(1<<15);
    loop{
        
        get_batch(&mut requests, &rec);
        for req in requests.iter_mut(){
            match req._type{
                FMTypeRequest::Write(pointer) => if write_req(&arc_mmap, &req, &pointer).is_err(){ return; },
                FMTypeRequest::IncreaseFileSize(length) => if increase_file_size_req(&arc_mmap, &req, &file, length).is_err() { return; },
                FMTypeRequest::Flush => if flush_req(&arc_mmap, &req).is_err() { return; }
            }
        }
    }
}

fn get_batch(requests: &mut Vec<FMRequest>, rec: &Receiver<FMRequest>){
    requests.clear();
    requests.push(rec.recv().unwrap());

    while let Ok(req) = rec.try_recv(){
        requests.push(req);
    }

}

fn write_req(arc_mmap: &ArcSwap<MmapMut>, req: &FMRequest, pointer: &ZeroCopyBytes) -> Result<(), DbError>{
    let guard = arc_mmap.load();
    let offset = req.offset;
    let reference = unsafe { &*pointer.raw_pointer};

    unsafe {
        let mut_ptr = guard.as_ptr() as *mut u8;
        let mmap = std::slice::from_raw_parts_mut(mut_ptr, guard.len());
        mmap[offset as usize .. offset as usize + pointer.length as usize].copy_from_slice(reference);
    }
    Ok(req.sender.send(FMResponse::new(FMTypeResponse::Write, Ok(())))?)
}

fn increase_file_size_req(arc_mmap: &ArcSwap<MmapMut>, req: &FMRequest, file: &File, length: u64) -> Result<(), DbError>{

    file.set_len(length)?;

    let status_mmap = unsafe {
        MmapOptions::new()
            .map_mut(file)
    };

    let mut status = Ok(());
    if let Err(e) = status_mmap{
        status = Err(DbError::Io(e))
    } else{
        let new_mmap = Arc::new(status_mmap.unwrap());
        arc_mmap.swap(new_mmap);
    }


    let _type = FMTypeResponse::IncreaseFileSize;
    let response = FMResponse::new(_type, status);
    Ok(req.sender.send(response)?)
}

fn flush_req(arc_mmap: &ArcSwap<MmapMut>, req: &FMRequest) -> Result<(), DbError>{

    let guard = arc_mmap.load();
    match guard.flush() {
        Ok(()) => {
            let response = FMResponse::new(FMTypeResponse::Flush, Ok(()));
            req.sender.send(response)?;
            Ok(())
        },
        Err(e) =>{
            let err_kind = e.kind();
            let response = FMResponse::new(FMTypeResponse::Flush, Err(DbError::Io(e)));
            req.sender.send(response)?;
            Err(DbError::Io(err_kind.into()))
        },
    }
}
