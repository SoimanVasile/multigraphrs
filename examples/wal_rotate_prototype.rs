use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;

const MAX_FILE_SIZE: u64 = 50 * 1024; // 50 KB for testing to easily observe rotation

pub struct WalRequest {
    payload: Vec<u8>,
    response_tx: Sender<io::Result<()>>,
}

pub struct Wal {
    request_tx: Sender<WalRequest>,
}

impl Wal {
    pub fn new<P: AsRef<Path>>(dir: P, max_file_size: u64) -> io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;

        let (request_tx, request_rx) = mpsc::channel();

        thread::spawn(move || {
            wal_background_thread(dir, max_file_size, request_rx);
        });

        Ok(Wal { request_tx })
    }

    pub fn write(&self, data: &[u8]) -> io::Result<()> {
        let (response_tx, response_rx) = mpsc::channel();
        let req = WalRequest {
            payload: data.to_vec(),
            response_tx,
        };

        if self.request_tx.send(req).is_err() {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "WAL thread died",
            ));
        }

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "WAL thread died before responding",
            ))
        })
    }
}

fn wal_background_thread(dir: PathBuf, max_file_size: u64, request_rx: Receiver<WalRequest>) {
    let mut file_index = 0;
    let mut current_file: Option<File> = None;
    let mut current_size = 0;

    let open_new_file = |index: u64| -> io::Result<(File, u64)> {
        let file_path = dir.join(format!("wal_{:04}.log", index));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;
        let metadata = file.metadata()?;
        Ok((file, metadata.len()))
    };

    // Try to open the first file
    match open_new_file(file_index) {
        Ok((f, size)) => {
            current_file = Some(f);
            current_size = size;
        }
        Err(e) => {
            eprintln!("WAL thread failed to open initial log file: {}", e);
            return;
        }
    }

    loop {
        // Block until we get at least one request
        let req = match request_rx.recv() {
            Ok(req) => req,
            Err(_) => break, // All senders disconnected, shut down
        };

        let mut batch = vec![req];

        // Try to collect any other pending requests without blocking
        while let Ok(req) = request_rx.try_recv() {
            batch.push(req);
        }

        let mut file = current_file.take().unwrap();
        let mut io_error = None;

        // Write batch
        for req in &batch {
            // Check rotation before writing
            if current_size >= max_file_size {
                file_index += 1;
                match open_new_file(file_index) {
                    Ok((new_file, size)) => {
                        file = new_file;
                        current_size = size;
                    }
                    Err(e) => {
                        io_error = Some(e);
                        break;
                    }
                }
            }

            match file.write_all(&req.payload) {
                Ok(_) => {
                    current_size += req.payload.len() as u64;
                }
                Err(e) => {
                    io_error = Some(e);
                    break;
                }
            }
        }

        // Sync all if no write error
        if io_error.is_none() {
            if let Err(e) = file.sync_all() {
                io_error = Some(e);
            }
        }

        // Put the file back if it's still good
        current_file = Some(file);

        // Notify all senders
        if let Some(e) = io_error {
            // Send the error to all callers in the batch
            for req in batch {
                let _ = req
                    .response_tx
                    .send(Err(io::Error::new(e.kind(), e.to_string())));
            }
        } else {
            for req in batch {
                let _ = req.response_tx.send(Ok(()));
            }
        }
    }
}

fn main() -> io::Result<()> {
    let wal_dir = Path::new("wal_test_dir");
    // Clean up from previous runs
    let _ = fs::remove_dir_all(wal_dir);

    println!("Initializing WAL prototype in {:?}", wal_dir);
    let wal = Arc::new(Wal::new(wal_dir, MAX_FILE_SIZE)?);

    let num_threads = 10;
    let writes_per_thread = 1000;
    let mut handles = vec![];

    let start_time = std::time::Instant::now();

    for i in 0..num_threads {
        let wal_clone = Arc::clone(&wal);
        let handle = thread::spawn(move || {
            for j in 0..writes_per_thread {
                let payload = format!("Thread {:02} - Message {:04}\n", i, j).into_bytes();
                if let Err(e) = wal_clone.write(&payload) {
                    eprintln!("Thread {} failed to write: {}", i, e);
                    break;
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let duration = start_time.elapsed();
    println!(
        "Completed {} total writes across {} threads in {:?}",
        num_threads * writes_per_thread,
        num_threads,
        duration
    );

    // List the generated files
    println!("Generated WAL files:");
    let mut files: Vec<_> = fs::read_dir(wal_dir)?
        .filter_map(|entry| entry.ok())
        .collect();
    files.sort_by_key(|e| e.file_name());

    for file in files {
        let meta = file.metadata()?;
        println!(" - {:?} ({} bytes)", file.file_name(), meta.len());
    }

    Ok(())
}
