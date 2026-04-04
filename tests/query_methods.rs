use multigraphrs::{Directed, GraphErrors, MultiGraph, Undirected, Weighted, WeightedDirected};

// ============ contains_node ============

#[test]
fn contains_node_exists() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    assert!(g.contains_node(&1));
}

#[test]
fn contains_node_not_exists() {
    let g = MultiGraph::<u32, u32, Directed>::new();
    assert!(!g.contains_node(&999));
}

#[test]
fn contains_node_after_remove() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();
    g.add_node("A").unwrap();
    assert!(g.contains_node(&"A"));
    g.remove_node(&"A").unwrap();
    assert!(!g.contains_node(&"A"));
}

// ============ node_count ============

#[test]
fn node_count_empty() {
    let g = MultiGraph::<u32, u32, Directed>::new();
    assert_eq!(g.node_count(), 0);
}

#[test]
fn node_count_after_adds() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_node(3).unwrap();
    assert_eq!(g.node_count(), 3);
}

#[test]
fn node_count_after_remove() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    assert_eq!(g.node_count(), 2);
    g.remove_node(&1).unwrap();
    assert_eq!(g.node_count(), 1);
}

// ============ edge_count ============

#[test]
fn edge_count_empty() {
    let g = MultiGraph::<u32, u32, Directed>::new();
    assert_eq!(g.edge_count(), 0);
}

#[test]
fn edge_count_directed() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2).unwrap();
    assert_eq!(g.edge_count(), 1);
}

#[test]
fn edge_count_undirected() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2).unwrap();
    // Undirected stores 2 internal edges
    assert_eq!(g.edge_count(), 2);
}

#[test]
fn edge_count_after_remove_edge() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2).unwrap();
    g.add_edge(1, 2).unwrap();
    assert_eq!(g.edge_count(), 2);
    g.remove_edge(1, 2).unwrap();
    assert_eq!(g.edge_count(), 1);
}

#[test]
fn edge_count_after_failed_remove_edge() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    assert_eq!(g.edge_count(), 0);
    let _ = g.remove_edge(1, 2); // fails — no edge
    assert_eq!(g.edge_count(), 0); // count stays 0, not underflow
}

#[test]
fn edge_count_after_remove_node() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_node(3).unwrap();
    g.add_edge(1, 2).unwrap();
    g.add_edge(1, 3).unwrap();
    g.add_edge(3, 2).unwrap();
    assert_eq!(g.edge_count(), 3);

    g.remove_node(&2).unwrap();
    // Edges 1->2 and 3->2 should be gone, only 1->3 remains
    assert_eq!(g.edge_count(), 1);
}

#[test]
fn edge_count_weighted() {
    let mut g = MultiGraph::<u32, f64, Weighted>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2, 3.14).unwrap();
    assert_eq!(g.edge_count(), 2); // bidirectional
}

#[test]
fn edge_count_weighted_directed() {
    let mut g = MultiGraph::<u32, f64, WeightedDirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2, 3.14).unwrap();
    assert_eq!(g.edge_count(), 1);
}

// ============ contains_edge ============

#[test]
fn contains_edge_exists() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2).unwrap();
    assert!(g.contains_edge(&1, &2));
}

#[test]
fn contains_edge_not_exists() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    assert!(!g.contains_edge(&1, &2));
}

#[test]
fn contains_edge_directed_one_way() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2).unwrap();
    assert!(g.contains_edge(&1, &2));
    assert!(!g.contains_edge(&2, &1));
}

#[test]
fn contains_edge_undirected_both_ways() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2).unwrap();
    assert!(g.contains_edge(&1, &2));
    assert!(g.contains_edge(&2, &1));
}

#[test]
fn contains_edge_source_missing() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    assert!(!g.contains_edge(&99, &1));
}

#[test]
fn contains_edge_target_missing() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    assert!(!g.contains_edge(&1, &99));
}

#[test]
fn contains_edge_after_remove() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2).unwrap();
    assert!(g.contains_edge(&1, &2));
    g.remove_edge(1, 2).unwrap();
    assert!(!g.contains_edge(&1, &2));
}
