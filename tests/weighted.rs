use multigraphrs::{MultiGraph, Weighted};

#[test]
fn add_edge_success() {
    let mut g = MultiGraph::<u32, f64, Weighted>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();

    let edge = g.add_edge(1, 2, 3.14).unwrap();
    assert_eq!(*edge.get_target(), 2);
    assert_eq!(*edge.get_weight(), 3.14);
}

#[test]
fn add_edge_source_missing() {
    let mut g = MultiGraph::<u32, i32, Weighted>::new();
    g.add_node(2).unwrap();
    assert!(g.add_edge(99, 2, 10).is_err());
}

#[test]
fn add_edge_target_missing() {
    let mut g = MultiGraph::<u32, i32, Weighted>::new();
    g.add_node(1).unwrap();
    assert!(g.add_edge(1, 99, 10).is_err());
}

#[test]
fn edge_is_bidirectional() {
    let mut g = MultiGraph::<u32, i32, Weighted>::new();
    g.add_node(10).unwrap();
    g.add_node(20).unwrap();
    g.add_edge(10, 20, 500).unwrap();

    assert_eq!(g.degree(&10), Ok(1));
    assert_eq!(g.degree(&20), Ok(1));
}

#[test]
fn multiple_edges_different_weights() {
    let mut g = MultiGraph::<u32, i32, Weighted>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();

    g.add_edge(1, 2, 100).unwrap();
    g.add_edge(1, 2, 200).unwrap();
    g.add_edge(1, 2, 300).unwrap();

    assert_eq!(g.degree(&1), Ok(3));
    assert_eq!(g.degree(&2), Ok(3));
}

#[test]
fn preserves_weight_value() {
    let mut g = MultiGraph::<&str, f64, Weighted>::new();
    g.add_node("NYC").unwrap();
    g.add_node("LON").unwrap();

    let edge = g.add_edge("NYC", "LON", 5585.0).unwrap();
    assert_eq!(*edge.get_target(), "LON");
    assert_eq!(*edge.get_weight(), 5585.0);
}

#[test]
fn self_loop() {
    let mut g = MultiGraph::<u32, u32, Weighted>::new();
    g.add_node(1).unwrap();
    g.add_edge(1, 1, 42).unwrap();
    assert_eq!(g.degree(&1), Ok(2));
}

#[test]
fn remove_edge_success() {
    let mut g = MultiGraph::<u32, i32, Weighted>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2, 100).unwrap();

    assert_eq!(g.degree(&1), Ok(1));
    assert_eq!(g.degree(&2), Ok(1));

    let removed = g.remove_edge(1, 2, 100).unwrap();
    assert_eq!(*removed.get_target(), 2);
    assert_eq!(*removed.get_weight(), 100);

    assert_eq!(g.degree(&1), Ok(0));
    assert_eq!(g.degree(&2), Ok(0));
}

#[test]
fn remove_edge_wrong_weight() {
    let mut g = MultiGraph::<u32, i32, Weighted>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2, 100).unwrap();

    // Edge exists with weight 100, not 999
    assert!(g.remove_edge(1, 2, 999).is_err());
    assert_eq!(g.degree(&1), Ok(1)); // edge still there
}

#[test]
fn remove_specific_parallel_edge() {
    let mut g = MultiGraph::<u32, i32, Weighted>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2, 100).unwrap();
    g.add_edge(1, 2, 200).unwrap();

    assert_eq!(g.degree(&1), Ok(2));

    g.remove_edge(1, 2, 100).unwrap();
    assert_eq!(g.degree(&1), Ok(1));
    assert_eq!(g.degree(&2), Ok(1));
}

#[test]
fn remove_edge_missing_node() {
    let mut g = MultiGraph::<u32, i32, Weighted>::new();
    g.add_node(1).unwrap();
    assert!(g.remove_edge(1, 99, 10).is_err());
    assert!(g.remove_edge(99, 1, 10).is_err());
}
