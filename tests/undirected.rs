use multigraphrs::{MultiGraph, Undirected};

#[test]
fn add_edge_success() {
    let mut g = MultiGraph::<&str, u32, Undirected>::new();
    g.add_node("A").unwrap();
    g.add_node("B").unwrap();

    let edge = g.add_edge("A", "B").unwrap();
    assert_eq!(edge.get_target(), "B");
    assert_eq!(edge.get_weight(), 1);
}

#[test]
fn add_edge_source_missing() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(2).unwrap();
    assert!(g.add_edge(99, 2).is_err());
}

#[test]
fn add_edge_target_missing() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    assert!(g.add_edge(1, 99).is_err());
}

#[test]
fn edge_is_bidirectional() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2).unwrap();

    assert_eq!(g.degree(&1), Ok(1));
    assert_eq!(g.degree(&2), Ok(1));
}

#[test]
fn multiple_edges_same_pair() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();

    g.add_edge(1, 2).unwrap();
    g.add_edge(1, 2).unwrap();

    assert_eq!(g.degree(&1), Ok(2));
    assert_eq!(g.degree(&2), Ok(2));
}

#[test]
fn self_loop() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    g.add_edge(1, 1).unwrap();

    // self-loop in undirected: adds edge to source (1→1) and reverse to target (1→1)
    assert_eq!(g.degree(&1), Ok(2));
}

#[test]
fn triangle() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_node(3).unwrap();

    g.add_edge(1, 2).unwrap();
    g.add_edge(2, 3).unwrap();
    g.add_edge(1, 3).unwrap();

    assert_eq!(g.degree(&1), Ok(2));
    assert_eq!(g.degree(&2), Ok(2));
    assert_eq!(g.degree(&3), Ok(2));
}

#[test]
fn remove_edge_success() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2).unwrap();

    assert_eq!(g.degree(&1), Ok(1));
    assert_eq!(g.degree(&2), Ok(1));

    g.remove_edge(1, 2).unwrap();

    assert_eq!(g.degree(&1), Ok(0));
    assert_eq!(g.degree(&2), Ok(0));
}

#[test]
fn remove_edge_nonexistent() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    assert!(g.remove_edge(1, 2).is_err());
}

#[test]
fn remove_edge_missing_node() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    assert!(g.remove_edge(1, 99).is_err());
    assert!(g.remove_edge(99, 1).is_err());
}
