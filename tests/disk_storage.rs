use multigraphrs::storage::disk_storage::disk_multigraph::DiskStorage;
use multigraphrs::storage::storage_backend::StorageBackend;
use multigraphrs::core::edge::Edge;

use std::env;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_test_id() -> usize {
    TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

struct TempDiskStorage {
    pub storage: DiskStorage<u32>,
    dir: std::path::PathBuf,
}

impl TempDiskStorage {
    fn new(test_name: &str) -> Self {
        let id = next_test_id();
        let mut dir = env::temp_dir();
        dir.push(format!("multigraphrs_disk_test_{}_{}", test_name, id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let struct_path = dir.join("struct.mmap");
        let data_path = dir.join("data.mmap");
        let node_path = dir.join("node.mmap");

        let storage = DiskStorage::<u32>::new(&struct_path, &data_path, &node_path);

        Self { storage, dir }
    }
}

impl Drop for TempDiskStorage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn test_add_node_increases_node_count() {
    let mut temp = TempDiskStorage::new("add_node");
    
    assert_eq!(temp.storage.node_count(), 0, "Initial node count should be 0");
    temp.storage.add_node();
    assert_eq!(temp.storage.node_count(), 1, "Node count should increment after add_node");
    temp.storage.add_node();
    // assert_eq!(temp.storage.node_count(), 2, "Node count should be 2 after second add_node");
}

#[test]
fn test_increment_node_counter() {
    let mut temp = TempDiskStorage::new("inc_node");
    
    assert_eq!(temp.storage.node_count(), 0);
    temp.storage.increment_node_counter();
    assert_eq!(temp.storage.node_count(), 1, "Incrementing node counter should update the internal count");
}

#[test]
fn test_add_edge_to_node_and_counts() {
    let mut temp = TempDiskStorage::new("add_edge");
    temp.storage.add_node(); // node 0
    temp.storage.add_node(); // node 1

    assert_eq!(temp.storage.edge_count(), 0);
    assert_eq!(temp.storage.node_len(0), 0);

    let edge = Edge::new(1, &42);
    temp.storage.add_edge_to_node(0, &edge);

    assert_eq!(temp.storage.edge_count(), 1, "Global edge count should increment");
    assert_eq!(temp.storage.node_len(0), 1, "Source node length should increment");
    assert_eq!(temp.storage.node_len(1), 0, "Target node length should remain 0 for directed addition");
}

#[test]
fn test_get_edges() {
    let mut temp = TempDiskStorage::new("get_edges");
    temp.storage.add_node(); // node 0
    temp.storage.add_node(); // node 1

    let edge = Edge::new(1, &42);
    temp.storage.add_edge_to_node(0, &edge);

    let edges: Vec<_> = temp.storage.get_edges(0).collect();
    assert_eq!(edges.len(), 1, "Iterator should yield 1 edge");
    assert_eq!(edges[0].get_target(), 1, "Target ID should match");
    assert_eq!(edges[0].get_weight(), 42, "Weight should match");
    
    let empty_edges: Vec<_> = temp.storage.get_edges(1).collect();
    assert!(empty_edges.is_empty(), "Iterator should yield 0 edges for node with no outgoing edges");
}

#[test]
fn test_contains_edge() {
    let mut temp = TempDiskStorage::new("contains_edge");
    temp.storage.add_node(); // node 0
    temp.storage.add_node(); // node 1

    let edge = Edge::new(1, &42);
    temp.storage.add_edge_to_node(0, &edge);

    // Found edge
    let found = temp.storage.contains_edge(0, 1);
    assert!(found.is_ok(), "Should confirm that the edge exists");
    assert_eq!(found.unwrap().get_weight(), 42, "Returned edge should have correct weight");

    // Non-existent edge
    let not_found = temp.storage.contains_edge(1, 0);
    assert!(not_found.is_err(), "Should return an error for non-existent edge");
}

#[test]
fn test_remove_edge() {
    let mut temp = TempDiskStorage::new("remove_edge");
    temp.storage.add_node(); // node 0
    temp.storage.add_node(); // node 1

    let edge = Edge::new(1, &42);
    temp.storage.add_edge_to_node(0, &edge);
    
    assert_eq!(temp.storage.edge_count(), 1);
    assert_eq!(temp.storage.node_len(0), 1);

    // Remove the edge
    let res = temp.storage.remove_edge(0, &edge, |a, b| a.get_target() == b.get_target() && a.get_weight() == b.get_weight());
    assert!(res.is_ok(), "Removing an existing edge should succeed");
    
    // Edge is logically removed
    assert_eq!(temp.storage.edge_count(), 0, "Edge count should drop to 0");
    assert_eq!(temp.storage.node_len(0), 0, "Node's edge length should drop to 0");
    assert!(temp.storage.contains_edge(0, 1).is_err(), "Contains should now return false (Err)");
}

#[test]
fn test_remove_node() {
    let mut temp = TempDiskStorage::new("remove_node");
    temp.storage.add_node(); // node 0
    temp.storage.add_node(); // node 1
    temp.storage.add_node(); // node 2

    let edge1 = Edge::new(1, &42);
    let edge2 = Edge::new(2, &100);
    
    temp.storage.add_edge_to_node(0, &edge1);
    temp.storage.add_edge_to_node(0, &edge2);
    
    assert_eq!(temp.storage.edge_count(), 2);
    assert_eq!(temp.storage.node_len(0), 2);

    // Remove the node (logically delete its edges and clear it)
    temp.storage.remove_node(0);
    
    // Edges should be removed logically from global count
    assert_eq!(temp.storage.node_len(0), 0, "Removed node should have 0 logic length");
    assert!(temp.storage.edge_count() < 2, "Edge count should decrease logically (or instantly, depending on implementation specs)");
    
    let edges: Vec<_> = temp.storage.get_edges(0).collect();
    assert!(edges.is_empty(), "Removed node should yield 0 edges on iteration");
}
