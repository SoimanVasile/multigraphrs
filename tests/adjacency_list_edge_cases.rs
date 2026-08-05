use multigraphrs::storage::adjacency_list::RamStorage;
use multigraphrs::storage::storage_backend::StorageBackend;
use multigraphrs::core::edge::Edge;

#[test]
fn test_ram_storage_edge_cases() {
    let mut storage = RamStorage::<u32, u32>::default(); // testing Default trait
    let node1 = storage.add_node().unwrap();
    let node2 = storage.add_node().unwrap();

    let edge = Edge::new(node2, &100);
    storage.add_edge_to_node(&node1, &edge).unwrap();
    
    // Testing get_edges_ref
    let edges_ref = storage.get_edges_ref(node1);
    assert_eq!(edges_ref.len(), 1);

    // Test reverse edges
    storage.add_reverse_edge(&node2, &node1).unwrap();
    assert_eq!(storage.get_reverse_edges(&node2), vec![node1]);

    // Test removing reverse edge
    storage.remove_reverse_edge(&node2, &node1).unwrap();
    assert!(storage.get_reverse_edges(&node2).is_empty());

    // Test removing a non-existent reverse edge (should not panic)
    storage.remove_reverse_edge(&node2, &999).unwrap();

    // Test clear node edges
    storage.clear_node_edges(&node1).unwrap();
    assert_eq!(storage.get_edges(&node1).count(), 0);

    // Free node id
    storage.free_node_id(&node2).unwrap();
    assert_eq!(storage.node_count(), 1);
    
    // Reusing the freed node ID
    let reused = storage.add_node().unwrap();
    assert_eq!(reused, node2);
    
    // Test increment node counter
    storage.increment_node_counter().unwrap();
    assert_eq!(storage.node_count(), 3);
}

#[test]
fn test_remove_edge_by_target() {
    let mut storage = RamStorage::<u32, u32>::new();
    let node1 = storage.add_node().unwrap();
    let node2 = storage.add_node().unwrap();

    storage.add_edge_to_node(&node1, &Edge::new(node2, &100));
    assert_eq!(storage.edge_count(), 1);
    
    // Test remove edge by target
    storage.remove_edge_by_target(&node1, &node2).unwrap();
    assert_eq!(storage.edge_count(), 0);
    
    // Removing by target when it doesn't exist
    storage.remove_edge_by_target(&node1, &node2).unwrap();
    assert_eq!(storage.edge_count(), 0);
}
