use std::{fs, path::PathBuf};

use multigraphrs::{Directed, DiskMultiGraph, DiskStorage, GraphError, MultiGraph, Undirected, Weighted, WeightedDirected};

#[test]
fn test_remove_node_directed() {
    let mut graph = MultiGraph::<&str, u32, Directed>::new();

    graph.add_node("A").unwrap();
    graph.add_node("B").unwrap();
    graph.add_node("C").unwrap();

    assert!(graph.node_count() == 3);
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
    assert_eq!(graph.node_count(), 2);

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
    assert_eq!(graph.remove_node(&999), Err(GraphError::NodeNotFound));
}

#[test]
fn remove_nonexistent_node_undirected() {
    let mut graph = MultiGraph::<u32, u32, Undirected>::new();
    assert_eq!(graph.remove_node(&42), Err(GraphError::NodeNotFound));
}

#[test]
fn remove_nonexistent_node_weighted() {
    let mut graph = MultiGraph::<u32, f64, Weighted>::new();
    assert_eq!(graph.remove_node(&1), Err(GraphError::NodeNotFound));
}

#[test]
fn remove_nonexistent_node_weighted_directed() {
    let mut graph = MultiGraph::<u32, f64, WeightedDirected>::new();
    assert_eq!(graph.remove_node(&0), Err(GraphError::NodeNotFound));
}

#[test]
fn remove_same_node_twice_returns_error() {
    let mut graph = MultiGraph::<&str, u32, Directed>::new();
    graph.add_node("A").unwrap();

    assert_eq!(graph.remove_node(&"A"), Ok("A"));
    // Second removal should fail
    assert_eq!(graph.remove_node(&"A"), Err(GraphError::NodeNotFound));
}

#[test]
fn remove_node_in_disk(){
    let mut dir = PathBuf::from("/home/missuki/Documents/rust_temp/");
    dir.push(format!("multigraphrs_stress_{}", "remove_node"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let backend = DiskStorage::<u32>::new(&dir);
    let mut graph: DiskMultiGraph<u32, u32, Undirected> = MultiGraph::with_backend(backend);

    graph.add_node(1).unwrap();
    graph.add_node(2).unwrap();
    graph.add_node(3).unwrap();

    assert_eq!(graph.node_count(), 3);

    graph.remove_node(&2).unwrap();

    assert_eq!(graph.node_count(), 2);
}
