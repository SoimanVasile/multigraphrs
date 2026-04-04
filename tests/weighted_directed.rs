use multigraphrs::{MultiGraph, WeightedDirected};

#[test]
fn add_edge_success() {
    let mut g = MultiGraph::<char, f64, WeightedDirected>::new();
    g.add_node('X').unwrap();
    g.add_node('Y').unwrap();

    let edge = g.add_edge('X', 'Y', 3.14).unwrap();
    assert_eq!(edge.get_target(), 'Y');
    assert_eq!(edge.get_weight(), 3.14);
}

#[test]
fn add_edge_source_missing() {
    let mut g = MultiGraph::<u32, u32, WeightedDirected>::new();
    g.add_node(2).unwrap();
    assert!(g.add_edge(99, 2, 1).is_err());
}

#[test]
fn add_edge_target_missing() {
    let mut g = MultiGraph::<u32, u32, WeightedDirected>::new();
    g.add_node(1).unwrap();
    assert!(g.add_edge(1, 99, 1).is_err());
}

#[test]
fn edge_is_one_way() {
    let mut g = MultiGraph::<u32, u32, WeightedDirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2, 10).unwrap();

    assert_eq!(g.degree(&1), Ok(1));
    assert_eq!(g.degree(&2), Ok(0));
}

#[test]
fn multiple_edges_same_pair() {
    let mut g = MultiGraph::<u32, f64, WeightedDirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();

    g.add_edge(1, 2, 1.0).unwrap();
    g.add_edge(1, 2, 2.0).unwrap();

    assert_eq!(g.degree(&1), Ok(2));
    assert_eq!(g.degree(&2), Ok(0));
}

#[test]
fn both_directions_explicit() {
    let mut g = MultiGraph::<u32, u32, WeightedDirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();

    g.add_edge(1, 2, 10).unwrap();
    g.add_edge(2, 1, 20).unwrap();

    assert_eq!(g.degree(&1), Ok(1));
    assert_eq!(g.degree(&2), Ok(1));
}

#[test]
fn self_loop() {
    let mut g = MultiGraph::<u32, u32, WeightedDirected>::new();
    g.add_node(1).unwrap();
    g.add_edge(1, 1, 5).unwrap();
    assert_eq!(g.degree(&1), Ok(1));
}

#[test]
fn remove_edge_success() {
    let mut g = MultiGraph::<u32, u32, WeightedDirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2, 50).unwrap();

    let removed = g.remove_edge(1, 2, 50).unwrap();
    assert_eq!(removed.get_target(), 2);
    assert_eq!(removed.get_weight(), 50);
    assert_eq!(g.degree(&1), Ok(0));
}

#[test]
fn remove_edge_nonexistent() {
    let mut g = MultiGraph::<u32, u32, WeightedDirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    assert!(g.remove_edge(1, 2, 1).is_err());
}

#[test]
fn remove_edge_missing_node() {
    let mut g = MultiGraph::<u32, u32, WeightedDirected>::new();
    g.add_node(1).unwrap();
    assert!(g.remove_edge(1, 99, 1).is_err());
    assert!(g.remove_edge(99, 1, 1).is_err());
}
