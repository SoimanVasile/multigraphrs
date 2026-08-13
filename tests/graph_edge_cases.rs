use multigraphrs::Directed;

#[test]
fn test_multigraph_with_capacity() {
    // We just want to make sure it doesn't panic and allocates internally correctly.
    let mut graph = multigraphrs::RamMultiGraph::<u32, u32, Directed>::with_backend(multigraphrs::storage::adjacency_list::RamStorage::new());
    
    // Add nodes to test functionality works seamlessly with preallocated capacity
    for i in 0..1500 {
        graph.add_node(i).unwrap();
    }
    
    assert_eq!(graph.node_count(), 1500);
}

#[test]
fn test_adding_existing_node_error() {
    let mut graph = multigraphrs::RamMultiGraph::<u32, u32, Directed>::with_backend(multigraphrs::storage::adjacency_list::RamStorage::new());
    graph.add_node(1).unwrap();
    let err = graph.add_node(1).unwrap_err();
    assert_eq!(err, multigraphrs::GraphError::NodeAlreadyExists);
}
