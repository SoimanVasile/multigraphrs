use multigraphrs::{MultiGraph, Directed, WeightedDirected, GraphErrors};

#[test]
fn add_node_returns_the_inserted_key() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    assert_eq!(g.add_node(1), Ok(1));
    assert_eq!(g.add_node(2), Ok(2));
}

#[test]
fn add_node_duplicate_returns_error() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    assert_eq!(g.add_node(1), Err(GraphErrors::NodeAlreadyExists));
}

#[test]
fn add_node_different_key_types_str() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();
    assert_eq!(g.add_node("hello"), Ok("hello"));
    assert_eq!(g.add_node("hello"), Err(GraphErrors::NodeAlreadyExists));
}

#[test]
fn add_node_different_key_types_char() {
    let mut g = MultiGraph::<char, f64, WeightedDirected>::new();
    assert_eq!(g.add_node('A'), Ok('A'));
    assert_eq!(g.add_node('B'), Ok('B'));
    assert_eq!(g.add_node('A'), Err(GraphErrors::NodeAlreadyExists));
}

#[test]
fn add_node_many_nodes() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    for i in 0..100 {
        assert_eq!(g.add_node(i), Ok(i));
    }
    for i in 0..100 {
        assert_eq!(g.add_node(i), Err(GraphErrors::NodeAlreadyExists));
    }
}

#[test]
fn add_node_initial_degree_is_zero() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    assert_eq!(g.degree(&1), Ok(0));
}

#[test]
fn degree_on_nonexistent_node() {
    let g = MultiGraph::<u32, u32, Directed>::new();
    assert_eq!(g.degree(&999), Err(GraphErrors::NodeNotFound));
}
