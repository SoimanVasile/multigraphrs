use multigraphrs::{Directed, GraphErrors, MultiGraph, Undirected, Weighted, WeightedDirected};

#[test]
fn test_remove_node_directed() {
    let mut graph = MultiGraph::<&str, u32, Directed>::new();

    graph.add_node("A").unwrap();
    graph.add_node("B").unwrap();
    graph.add_node("C").unwrap();

    // A -> B
    // A -> C
    // C -> B
    graph.add_edge("A", "B").unwrap();
    graph.add_edge("A", "C").unwrap();
    graph.add_edge("C", "B").unwrap();

    assert_eq!(graph.degree(&"A").unwrap(), 2);
    assert_eq!(graph.degree(&"C").unwrap(), 1);

    // Remove B
    let removed = graph.remove_node(&"B").unwrap();
    assert_eq!(removed, "B");

    // The edges A -> B and C -> B should be removed
    // So degree of A should be 1 (A -> C), and C should be 0.
    assert_eq!(graph.degree(&"A").unwrap(), 1);
    assert_eq!(graph.degree(&"C").unwrap(), 0);

    // Trying to get degree of B should return an error
    assert!(graph.degree(&"B").is_err());
}

#[test]
fn test_remove_node_undirected() {
    let mut graph = MultiGraph::<&str, u32, Undirected>::new();

    graph.add_node("X").unwrap();
    graph.add_node("Y").unwrap();
    
    // X <-> Y
    graph.add_edge("X", "Y").unwrap();
    
    assert_eq!(graph.degree(&"X").unwrap(), 1);
    assert_eq!(graph.degree(&"Y").unwrap(), 1);
    
    graph.remove_node(&"Y").unwrap();
    
    // Y is gone, X has no edges left
    assert_eq!(graph.degree(&"X").unwrap(), 0);
    assert!(graph.degree(&"Y").is_err());
}

#[test]
fn test_remove_node_weighted() {
    let mut graph = MultiGraph::<&str, f64, Weighted>::new();
    
    graph.add_node("N1").unwrap();
    graph.add_node("N2").unwrap();
    
    graph.add_edge("N1", "N2", 5.5).unwrap();
    
    assert_eq!(graph.degree(&"N1").unwrap(), 1);
    
    graph.remove_node(&"N2").unwrap();
    assert_eq!(graph.degree(&"N1").unwrap(), 0);
}

#[test]
fn test_remove_node_weighted_directed() {
    let mut graph = MultiGraph::<&str, f64, WeightedDirected>::new();
    
    graph.add_node("Source").unwrap();
    graph.add_node("Dest").unwrap();
    
    graph.add_edge("Source", "Dest", 10.0).unwrap();
    
    assert_eq!(graph.degree(&"Source").unwrap(), 1);
    
    graph.remove_node(&"Dest").unwrap();
    assert_eq!(graph.degree(&"Source").unwrap(), 0);
}

// --- Error path tests ---

#[test]
fn remove_nonexistent_node_directed() {
    let mut graph = MultiGraph::<u32, u32, Directed>::new();
    assert_eq!(graph.remove_node(&999), Err(GraphErrors::NodeNotFound));
}

#[test]
fn remove_nonexistent_node_undirected() {
    let mut graph = MultiGraph::<u32, u32, Undirected>::new();
    assert_eq!(graph.remove_node(&42), Err(GraphErrors::NodeNotFound));
}

#[test]
fn remove_nonexistent_node_weighted() {
    let mut graph = MultiGraph::<u32, f64, Weighted>::new();
    assert_eq!(graph.remove_node(&1), Err(GraphErrors::NodeNotFound));
}

#[test]
fn remove_nonexistent_node_weighted_directed() {
    let mut graph = MultiGraph::<u32, f64, WeightedDirected>::new();
    assert_eq!(graph.remove_node(&0), Err(GraphErrors::NodeNotFound));
}

#[test]
fn remove_same_node_twice_returns_error() {
    let mut graph = MultiGraph::<&str, u32, Directed>::new();
    graph.add_node("A").unwrap();

    assert_eq!(graph.remove_node(&"A"), Ok("A"));
    // Second removal should fail
    assert_eq!(graph.remove_node(&"A"), Err(GraphErrors::NodeNotFound));
}
