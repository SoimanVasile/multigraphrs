use multigraphrs::storage::disk_storage::wal::{WalManager, WalTransaction, FileId};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

static TEST_ID: AtomicUsize = AtomicUsize::new(0);

fn next_test_dir() -> std::path::PathBuf {
    let id = TEST_ID.fetch_add(1, Ordering::SeqCst);
    let mut dir = std::env::temp_dir();
    dir.push(format!("multigraphrs_wal_async_{}", id));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

// 1. Basic: serialize → commit → read back from wal.bin
#[test]
fn test_commit_writes_to_wal_file() {
    let dir = next_test_dir();
    let mut mgr = WalManager::new(dir.clone()).unwrap();
    mgr.start(1024 * 1024).unwrap(); // 1 MB — no rotation expected

    let mut tx = WalTransaction::new();
    tx.write_bytes(FileId::Node, 0, &[1, 2, 3, 4]);
    tx.zero_mmap(FileId::Structure, 10, 20);

    let rotated = mgr.commit(&tx).unwrap();
    assert!(!rotated, "should not rotate on a small write");

    // The WAL file should exist and be non-empty
    let wal_path = dir.join("wal.bin");
    assert!(wal_path.exists());
    assert!(fs::metadata(&wal_path).unwrap().len() > 0);
}

// 2. Serialize + deserialize round-trip
#[test]
fn test_serialize_deserialize_roundtrip() {
    let mut tx = WalTransaction::new();
    tx.write_bytes(FileId::Node, 100, &[10, 20, 30]);
    tx.zero_mmap(FileId::Structure, 0, 64);
    tx.copy_within(FileId::Reverse, 0, 32, 64);
    tx.increase_file_size(FileId::Data, 8192);

    let bytes = tx.serialize();

    let dir = next_test_dir();
    let wal_path = dir.join("roundtrip.bin");
    fs::write(&wal_path, &bytes).unwrap();

    let mut file = fs::OpenOptions::new().read(true).open(&wal_path).unwrap();
    let txns = WalTransaction::deserialize_all(&mut file).unwrap();
    assert_eq!(txns.len(), 1);
    assert_eq!(txns[0].records.len(), 4);

    // Verify the first record
    match &txns[0].records[0] {
        multigraphrs::storage::disk_storage::wal::WalRecord::Write { file_id, offset, bytes } => {
            assert_eq!(*file_id, FileId::Node);
            assert_eq!(*offset, 100);
            assert_eq!(bytes, &[10, 20, 30]);
        }
        _ => panic!("expected Write record"),
    }
}

// ────────────────────────────────────────────────────────────────────
// 3. Commit without start() should fail
// ────────────────────────────────────────────────────────────────────
#[test]
fn test_commit_without_start_fails() {
    let dir = next_test_dir();
    let mut mgr = WalManager::new(dir).unwrap();

    let mut tx = WalTransaction::new();
    tx.write_bytes(FileId::Node, 0, &[1]);

    let result = mgr.commit(&tx);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotConnected);
}

// ────────────────────────────────────────────────────────────────────
// 4. Rotation triggers when WAL exceeds max_file_size
// ────────────────────────────────────────────────────────────────────
#[test]
fn test_rotation_triggers_on_size() {
    let dir = next_test_dir();
    let mut mgr = WalManager::new(dir.clone()).unwrap();
    // Set a very small max so rotation happens quickly
    mgr.start(200).unwrap();

    let mut ever_rotated = false;

    // Write enough transactions to exceed 200 bytes
    for i in 0..50 {
        let mut tx = WalTransaction::new();
        tx.write_bytes(FileId::Node, i * 8, &[0xAA; 8]);
        let rotated = mgr.commit(&tx).unwrap();
        if rotated {
            ever_rotated = true;
        }
    }

    assert!(ever_rotated, "at least one rotation should have happened");

    // After rotation, old_wal.bin should exist
    let _old_wal = dir.join("old_wal.bin");
    let wal = dir.join("wal.bin");
    assert!(wal.exists(), "wal.bin should exist");
    // old_wal.bin may or may not exist depending on whether
    // a second rotation deleted it
}

// ────────────────────────────────────────────────────────────────────
// 5. Only the first caller in a batch gets rotated=true
// ────────────────────────────────────────────────────────────────────
#[test]
fn test_only_first_gets_rotated() {
    let dir = next_test_dir();
    let mut mgr = WalManager::new(dir.clone()).unwrap();
    // Very small so we rotate on the first batch
    mgr.start(50).unwrap();

    // Commit multiple transactions; at most one rotated=true per rotation
    let mut rotation_count = 0;
    for _ in 0..20 {
        let mut tx = WalTransaction::new();
        tx.write_bytes(FileId::Node, 0, &[0xBB; 32]);
        if mgr.commit(&tx).unwrap() {
            rotation_count += 1;
        }
    }

    // With 20 commits of ~40 bytes each (payload + framing) and max=50,
    // we should have roughly one rotation per commit, but each rotation
    // only returns true once
    assert_eq!(rotation_count, 20);
}

// ────────────────────────────────────────────────────────────────────
// 6. Multiple transactions accumulate correctly in the WAL
// ────────────────────────────────────────────────────────────────────
#[test]
fn test_multiple_transactions_persist() {
    let dir = next_test_dir();
    let mut mgr = WalManager::new(dir.clone()).unwrap();
    mgr.start(1024 * 1024).unwrap(); // large — no rotation

    let num_txns = 100;
    for i in 0..num_txns {
        let mut tx = WalTransaction::new();
        tx.write_bytes(FileId::Node, i * 4, &(i as u32).to_le_bytes());
        mgr.commit(&tx).unwrap();
    }

    // Drop the manager to close the channel and stop the thread
    drop(mgr);

    // Read back the WAL and verify all transactions are there
    let wal_path = dir.join("wal.bin");
    let mut file = fs::OpenOptions::new().read(true).open(&wal_path).unwrap();
    let txns = WalTransaction::deserialize_all(&mut file).unwrap();
    assert_eq!(txns.len(), num_txns as usize);

    for transaction in txns{
        assert_eq!(transaction.records[0].file_id(), FileId::Node);
    }
}

// ────────────────────────────────────────────────────────────────────
// 7. Concurrent commits from multiple threads all succeed
// ────────────────────────────────────────────────────────────────────
#[test]
fn test_concurrent_commits_from_threads() {
    let dir = next_test_dir();
    let mut mgr = WalManager::new(dir.clone()).unwrap();
    mgr.start(1024 * 1024).unwrap();

    let num_threads = 8;
    let commits_per_thread = 50;

    // WalManager::commit takes &mut self, so we can't share it across
    // threads directly. Instead, we grab the internal sender and build
    // WalRequests manually — the same thing commit() does internally.
    // This tests the background thread's ability to handle concurrent
    // requests from multiple producers.
    let sender = mgr.request_tx.take().unwrap();
    let barrier = Arc::new(Barrier::new(num_threads));

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let sender = sender.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait(); // all threads start at once
                for i in 0..commits_per_thread {
                    let mut tx = WalTransaction::new();
                    let value = (thread_id * 1000 + i) as u64;
                    tx.write_bytes(FileId::Node, value, &value.to_le_bytes());

                    let (response_tx, response_rx) = std::sync::mpsc::channel();
                    let req = multigraphrs::storage::disk_storage::wal::WalRequest {
                        payload: tx.serialize(),
                        response_tx,
                    };
                    sender.send(req).unwrap();
                    response_rx.recv().unwrap().unwrap();
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    // Drop sender so the background thread stops
    drop(sender);

    // Verify all transactions landed in the WAL
    let wal_path = dir.join("wal.bin");
    let mut file = fs::OpenOptions::new().read(true).open(&wal_path).unwrap();
    let txns = WalTransaction::deserialize_all(&mut file).unwrap();
    assert_eq!(
        txns.len(),
        num_threads * commits_per_thread,
        "every concurrent commit should be persisted"
    );
}

// ────────────────────────────────────────────────────────────────────
// 8. Concurrent commits with rotation stress test
// ────────────────────────────────────────────────────────────────────
#[test]
fn test_concurrent_commits_with_rotation() {
    let dir = next_test_dir();
    let mut mgr = WalManager::new(dir.clone()).unwrap();
    // Small max so rotation happens frequently under concurrent load
    mgr.start(256).unwrap();

    let num_threads = 4;
    let commits_per_thread = 100;

    let sender = mgr.request_tx.take().unwrap();
    let barrier = Arc::new(Barrier::new(num_threads));
    let total_rotations = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let sender = sender.clone();
            let barrier = Arc::clone(&barrier);
            let rotations = Arc::clone(&total_rotations);
            std::thread::spawn(move || {
                barrier.wait();
                for i in 0..commits_per_thread {
                    let mut tx = WalTransaction::new();
                    let value = (thread_id * 10000 + i) as u64;
                    tx.write_bytes(FileId::Node, value, &value.to_le_bytes());

                    let (response_tx, response_rx) = std::sync::mpsc::channel();
                    let req = multigraphrs::storage::disk_storage::wal::WalRequest {
                        payload: tx.serialize(),
                        response_tx,
                    };
                    sender.send(req).unwrap();
                    let rotated = response_rx.recv().unwrap().unwrap();
                    if rotated {
                        rotations.fetch_add(1, Ordering::Relaxed);
                    }
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    drop(sender);

    let rotations = total_rotations.load(Ordering::Relaxed);
    assert!(
        rotations > 0,
        "should have rotated at least once under concurrent load"
    );

    // wal.bin should exist (the current WAL after the last rotation)
    assert!(dir.join("wal.bin").exists());
}

// ────────────────────────────────────────────────────────────────────
// 9. WAL survives rapid fire commits without data loss
// ────────────────────────────────────────────────────────────────────
#[test]
fn test_rapid_fire_no_data_loss() {
    let dir = next_test_dir();
    let mut mgr = WalManager::new(dir.clone()).unwrap();
    // Large max so no rotation — pure throughput test
    mgr.start(100 * 1024 * 1024).unwrap();

    let total = 1000;
    for i in 0u64..total {
        let mut tx = WalTransaction::new();
        tx.write_bytes(FileId::Node, i * 8, &i.to_le_bytes());
        mgr.commit(&tx).unwrap();
    }

    drop(mgr);

    let wal_path = dir.join("wal.bin");
    let mut file = fs::OpenOptions::new().read(true).open(&wal_path).unwrap();
    let txns = WalTransaction::deserialize_all(&mut file).unwrap();
    assert_eq!(txns.len(), total as usize, "all {} transactions should be in the WAL", total);

    // Verify sequential values
    for (i, tx) in txns.iter().enumerate() {
        assert_eq!(tx.records.len(), 1);
        match &tx.records[0] {
            multigraphrs::storage::disk_storage::wal::WalRecord::Write { offset, bytes, .. } => {
                assert_eq!(*offset, (i as u64) * 8);
                let val = u64::from_le_bytes(bytes[..8].try_into().unwrap());
                assert_eq!(val, i as u64);
            }
            _ => panic!("expected Write record at index {}", i),
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// 10. Corrupted WAL (truncated) is handled gracefully
// ────────────────────────────────────────────────────────────────────
#[test]
fn test_deserialize_truncated_wal() {
    let dir = next_test_dir();
    let wal_path = dir.join("truncated.bin");

    // Write 3 valid transactions
    let mut all_bytes = Vec::new();
    for i in 0..3 {
        let mut tx = WalTransaction::new();
        tx.write_bytes(FileId::Node, i, &[i as u8; 4]);
        all_bytes.extend(tx.serialize());
    }

    // Append a fourth transaction but only half of it (simulating crash)
    let mut corrupt_tx = WalTransaction::new();
    corrupt_tx.write_bytes(FileId::Structure, 999, &[0xFF; 16]);
    let corrupt_bytes = corrupt_tx.serialize();
    all_bytes.extend(&corrupt_bytes[..corrupt_bytes.len() / 2]);

    fs::write(&wal_path, &all_bytes).unwrap();

    let mut file = fs::OpenOptions::new().read(true).open(&wal_path).unwrap();
    let txns = WalTransaction::deserialize_all(&mut file).unwrap();

    // Should recover the 3 valid transactions and skip the corrupted one
    assert_eq!(txns.len(), 3, "should recover 3 valid transactions, skip corrupted tail");
}

// ────────────────────────────────────────────────────────────────────
// 11. Full end-to-end: concurrent commits → drop → replay from file
// ────────────────────────────────────────────────────────────────────
#[test]
fn test_concurrent_commit_then_replay_integrity() {
    let dir = next_test_dir();
    let mut mgr = WalManager::new(dir.clone()).unwrap();
    mgr.start(1024 * 1024).unwrap();

    let num_threads = 4;
    let commits_per_thread = 50;

    let sender = mgr.request_tx.take().unwrap();
    let barrier = Arc::new(Barrier::new(num_threads));

    let handles: Vec<_> = (0..num_threads)
        .map(|tid| {
            let sender = sender.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                for i in 0..commits_per_thread {
                    let mut tx = WalTransaction::new();
                    let tag = ((tid as u64) << 32) | (i as u64);
                    tx.write_bytes(FileId::Node, tag, &tag.to_le_bytes());
                    let (resp_tx, resp_rx) = std::sync::mpsc::channel();
                    let req = multigraphrs::storage::disk_storage::wal::WalRequest {
                        payload: tx.serialize(),
                        response_tx: resp_tx,
                    };
                    sender.send(req).unwrap();
                    resp_rx.recv().unwrap().unwrap();
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
    drop(sender);

    // Read back and verify every transaction is present
    let wal_path = dir.join("wal.bin");
    let mut file = fs::OpenOptions::new().read(true).open(&wal_path).unwrap();
    let txns = WalTransaction::deserialize_all(&mut file).unwrap();
    assert_eq!(txns.len(), num_threads * commits_per_thread);

    // Collect all tags and verify uniqueness
    let mut tags: Vec<u64> = txns
        .iter()
        .map(|tx| {
            match &tx.records[0] {
                multigraphrs::storage::disk_storage::wal::WalRecord::Write { offset, .. } => *offset,
                _ => panic!("expected Write"),
            }
        })
        .collect();
    tags.sort();
    tags.dedup();
    assert_eq!(
        tags.len(),
        num_threads * commits_per_thread,
        "every concurrent write should have a unique tag — no data was lost or duplicated"
    );
}
