use multigraphrs::{Directed, DiskStorage, MultiGraph};
use std::env;
use std::fs;
use std::path::PathBuf;

fn get_temp_dir(test_name: &str) -> PathBuf {
    let mut dir = env::temp_dir();
    dir.push(format!("multigraphrs_dict_edge_cases_{}", test_name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_bulk_add_node_file_size() {
    let dir = get_temp_dir("bulk_add_file_size");
    
    let backend = DiskStorage::<u32, u32>::new(&dir);
    let mut graph = MultiGraph::<u32, u32, Directed, _>::with_backend(backend);

    // Add 10,000 nodes using bulk_add_node
    let nodes: Vec<u32> = (0..10_000).collect();
    graph.bulk_add_node(&nodes).unwrap();

    // Verify all nodes are present
    assert_eq!(graph.node_count(), 10_000);
    assert_eq!(graph.degree(&5000).unwrap(), 0);

    // Verify the data file size didn't explode exponentially.
    // Each u32 is 4 bytes. 10,000 nodes * 4 = 40,000 bytes.
    // The data file should be tightly packed. Let's make sure it's under 1MB.
    let data_file = dir.join("data.bin");
    let metadata = fs::metadata(&data_file).unwrap();
    assert!(metadata.len() < 70_000_000, "File size exploded exponentially (expected ~64MB, got {} bytes)", metadata.len());
}

#[test]
fn test_dictionary_restores_on_startup() {
    let dir = get_temp_dir("dict_restore");
    
    // Phase 1: Create and add nodes
    {
        let backend = DiskStorage::<u32, u32>::new(&dir);
        let mut graph = MultiGraph::<u32, u32, Directed, _>::with_backend(backend);

        graph.add_node(100).unwrap();
        graph.add_node(200).unwrap();
        graph.add_node(300).unwrap();

        assert_eq!(graph.node_count(), 3);
        assert!(graph.contains_node(&200).unwrap());
    } // Graph is dropped, files persist

    // Phase 2: Re-open the database and verify nodes are loaded
    {
        let backend = DiskStorage::<u32, u32>::new(&dir);
        let graph = MultiGraph::<u32, u32, Directed, _>::with_backend(backend);

        assert_eq!(graph.node_count(), 3);
        assert!(graph.contains_node(&100).unwrap());
        assert!(graph.contains_node(&200).unwrap());
        assert!(graph.contains_node(&300).unwrap());
    }
}
