use multigraphrs::storage::disk_storage::disk_multigraph::DiskStorage;
use multigraphrs::storage::storage_backend::StorageBackend;
use multigraphrs::storage::disk_storage::wal::{WalTransaction, FileId};
use multigraphrs::core::edge::Edge;
use std::{env, fs};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::io::Write;

static TEST_ID: AtomicUsize = AtomicUsize::new(0);

fn next_test_id() -> usize {
    TEST_ID.fetch_add(1, Ordering::SeqCst)
}

#[test]
fn test_wal_crash_recovery() {
    let id = next_test_id();
    let mut dir = env::temp_dir();
    dir.push(format!("multigraphrs_wal_crash_{}", id));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // 1. Start a fresh database and add some initial data via the backend directly.
    {
        let mut backend = DiskStorage::<u32>::new(&dir);
        let n1 = backend.add_node();
        let n2 = backend.add_node();
        let edge = Edge::new(n2, &42);
        backend.add_edge_to_node(&n1, &edge);
        
        assert_eq!(backend.edge_count(), 1);
        assert_eq!(backend.node_count(), 2);
    }
    
    // 2. Simulate a mid-transaction crash by appending a half-written transaction to wal.bin
    {
        let wal_path = dir.join("wal.bin");
        let mut wal_file = fs::OpenOptions::new().append(true).open(&wal_path).unwrap();
        
        let mut fake_tx = WalTransaction::new();
        fake_tx.write_bytes(FileId::Node, 0, &[1, 2, 3, 4]); 
        let serialized = fake_tx.serialize();
        
        // Write only half of the serialized data to simulate power loss during commit
        wal_file.write_all(&serialized[0..serialized.len() / 2]).unwrap();
        wal_file.sync_all().unwrap();
    }
    
    // 3. Restart the database. It should ignore the corrupted WAL payload and still retain the previous data.
    {
        let mut backend = DiskStorage::<u32>::new(&dir);
        
        assert_eq!(backend.edge_count(), 1);
        assert_eq!(backend.node_count(), 2);
        
        // Add more edges to prove it's still healthy
        let n3 = backend.add_node();
        let edge2 = Edge::new(n3, &99);
        backend.add_edge_to_node(&1, &edge2);
        assert_eq!(backend.edge_count(), 2);
    }
}
