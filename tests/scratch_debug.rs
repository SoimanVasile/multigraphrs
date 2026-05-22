use multigraphrs::DiskStorage;
use std::env;
use std::fs;
use multigraphrs::storage::storage_backend::StorageBackend;

#[test]
fn test_debug_reverse_edges() {
    let mut dir = env::temp_dir();
    dir.push("multigraphrs_integration_debug");
    let _ = fs::remove_dir_all(&dir); // Clean up if exists from previous run
    fs::create_dir_all(&dir).unwrap();

    let mut backend = DiskStorage::<f64>::new(&dir);
    
    // Add nodes 0 to 200
    for i in 0..=200 {
        backend.add_node();
    }
    
    // Edges from 61..=210 to 60
    for i in 61..=210 {
        // We will just use public api
        backend.add_reverse_edge(60, i);
    }
    
    // Verify Reverse Edges
    let reverse_edges = backend.get_reverse_edges(60);
    println!("Reverse edges for 60 (len: {}): {:?}", reverse_edges.len(), reverse_edges);
    
    let expected: Vec<u64> = (61..=210).collect();
    if reverse_edges != expected {
        println!("Mismatch! Expected {:?}", expected);
    }
    
    // Let's just print reverse edges
    let reverse_edges = backend.get_reverse_edges(60);
    println!("Reverse edges for 60 (len: {}): {:?}", reverse_edges.len(), reverse_edges);
}
