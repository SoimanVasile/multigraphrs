use multigraphrs::{RamMultiGraph, Directed, WeightedDirected, GraphError};

#[test]
fn add_node_returns_the_inserted_key() {
    let mut g = RamMultiGraph::<u32, u32, Directed>::new();
    assert_eq!(g.add_node(1), Ok(1));
    assert_eq!(g.add_node(2), Ok(2));
}

#[test]
fn add_node_duplicate_returns_error() {
    let mut g = RamMultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    assert_eq!(g.add_node(1), Err(GraphError::NodeAlreadyExists));
}

#[test]
fn add_node_different_key_types_str() {
    let mut g = RamMultiGraph::<String, u32, Directed>::new();
    assert_eq!(g.add_node("hello".into()), Ok("hello".to_string()));
    assert_eq!(g.add_node("hello".into()), Err(GraphError::NodeAlreadyExists));
}

#[test]
fn add_node_different_key_types_char() {
    let mut g = RamMultiGraph::<u32, f64, WeightedDirected>::new();
    assert_eq!(g.add_node(65), Ok(65));
    assert_eq!(g.add_node(66), Ok(66));
    assert_eq!(g.add_node(65), Err(GraphError::NodeAlreadyExists));
}

#[test]
fn add_node_many_nodes() {
    let mut g = RamMultiGraph::<u32, u32, Directed>::new();
    for i in 0..100 {
        assert_eq!(g.add_node(i), Ok(i));
    }
    for i in 0..100 {
        assert_eq!(g.add_node(i), Err(GraphError::NodeAlreadyExists));
    }
}

#[test]
fn add_node_initial_degree_is_zero() {
    let mut g = RamMultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    assert_eq!(g.degree(&1), Ok(0));
}

#[test]
fn degree_on_nonexistent_node() {
    let g = RamMultiGraph::<u32, u32, Directed>::new();
    assert_eq!(g.degree(&999), Err(GraphError::NodeNotFound));
}
