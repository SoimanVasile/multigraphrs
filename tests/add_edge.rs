use multigraphrs::{MultiGraph, Directed, Undirected, Weighted, WeightedDirected, GraphErrors};

// ============================================================
//  add_node tests
// ============================================================

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
    // all duplicates should fail
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

// ============================================================
//  add_edge – Directed
// ============================================================

#[test]
fn directed_add_edge_success() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();
    g.add_node("A").unwrap();
    g.add_node("B").unwrap();

    let edge = g.add_edge("A", "B").unwrap();
    assert_eq!(edge.target, 1); // B is the second node added → internal id 1
    assert_eq!(edge.weight, 1); // default weight for Directed
}

#[test]
fn directed_add_edge_source_missing() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(2).unwrap();
    assert_eq!(g.add_edge(99, 2), Err(GraphErrors::NodeNotFound));
}

#[test]
fn directed_add_edge_target_missing() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    assert_eq!(g.add_edge(1, 99), Err(GraphErrors::NodeNotFound));
}

#[test]
fn directed_add_edge_both_missing() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    assert_eq!(g.add_edge(1, 2), Err(GraphErrors::NodeNotFound));
}

#[test]
fn directed_edge_is_one_way() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2).unwrap();

    // source should have degree 1, target should have degree 0
    assert_eq!(g.degree(&1), Ok(1));
    assert_eq!(g.degree(&2), Ok(0));
}

#[test]
fn directed_multiple_edges_same_pair() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();

    g.add_edge(1, 2).unwrap();
    g.add_edge(1, 2).unwrap();
    g.add_edge(1, 2).unwrap();

    // multigraph allows parallel edges
    assert_eq!(g.degree(&1), Ok(3));
    assert_eq!(g.degree(&2), Ok(0));
}

#[test]
fn directed_self_loop() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_edge(1, 1).unwrap();
    assert_eq!(g.degree(&1), Ok(1));
}

#[test]
fn directed_multiple_targets() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(0).unwrap();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_node(3).unwrap();

    g.add_edge(0, 1).unwrap();
    g.add_edge(0, 2).unwrap();
    g.add_edge(0, 3).unwrap();

    assert_eq!(g.degree(&0), Ok(3));
    assert_eq!(g.degree(&1), Ok(0));
    assert_eq!(g.degree(&2), Ok(0));
    assert_eq!(g.degree(&3), Ok(0));
}

// ============================================================
//  add_edge – Undirected
// ============================================================

#[test]
fn undirected_add_edge_success() {
    let mut g = MultiGraph::<&str, u32, Undirected>::new();
    g.add_node("A").unwrap();
    g.add_node("B").unwrap();

    let edge = g.add_edge("A", "B").unwrap();
    assert_eq!(edge.weight, 1);
}

#[test]
fn undirected_add_edge_source_missing() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(2).unwrap();
    assert_eq!(g.add_edge(99, 2), Err(GraphErrors::NodeNotFound));
}

#[test]
fn undirected_add_edge_target_missing() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    assert_eq!(g.add_edge(1, 99), Err(GraphErrors::NodeNotFound));
}

#[test]
fn undirected_edge_is_bidirectional() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2).unwrap();

    // both nodes should have degree 1
    assert_eq!(g.degree(&1), Ok(1));
    assert_eq!(g.degree(&2), Ok(1));
}

#[test]
fn undirected_multiple_edges_same_pair() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();

    g.add_edge(1, 2).unwrap();
    g.add_edge(1, 2).unwrap();

    assert_eq!(g.degree(&1), Ok(2));
    assert_eq!(g.degree(&2), Ok(2));
}

#[test]
fn undirected_self_loop() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    g.add_edge(1, 1).unwrap();

    // self-loop in undirected: adds edge to source (1→1) and reverse to target (1→1)
    assert_eq!(g.degree(&1), Ok(2));
}

#[test]
fn undirected_triangle() {
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

// ============================================================
//  add_edge – Weighted (undirected)
// ============================================================

#[test]
fn weighted_add_edge_success() {
    let mut g = MultiGraph::<u32, f64, Weighted>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();

    let edge = g.add_edge(1, 2, 3.14).unwrap();
    assert_eq!(edge.weight, 3.14);
}

#[test]
fn weighted_add_edge_source_missing() {
    let mut g = MultiGraph::<u32, i32, Weighted>::new();
    g.add_node(2).unwrap();
    assert_eq!(g.add_edge(99, 2, 10), Err(GraphErrors::NodeNotFound));
}

#[test]
fn weighted_add_edge_target_missing() {
    let mut g = MultiGraph::<u32, i32, Weighted>::new();
    g.add_node(1).unwrap();
    assert_eq!(g.add_edge(1, 99, 10), Err(GraphErrors::NodeNotFound));
}

#[test]
fn weighted_edge_is_bidirectional() {
    let mut g = MultiGraph::<u32, i32, Weighted>::new();
    g.add_node(10).unwrap();
    g.add_node(20).unwrap();
    g.add_edge(10, 20, 500).unwrap();

    assert_eq!(g.degree(&10), Ok(1));
    assert_eq!(g.degree(&20), Ok(1));
}

#[test]
fn weighted_multiple_edges_different_weights() {
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
fn weighted_preserves_weight_value() {
    let mut g = MultiGraph::<&str, f64, Weighted>::new();
    g.add_node("NYC").unwrap();
    g.add_node("LON").unwrap();

    let edge = g.add_edge("NYC", "LON", 5585.0).unwrap();
    assert_eq!(edge.weight, 5585.0);
}

#[test]
fn weighted_self_loop() {
    let mut g = MultiGraph::<u32, u32, Weighted>::new();
    g.add_node(1).unwrap();
    g.add_edge(1, 1, 42).unwrap();
    assert_eq!(g.degree(&1), Ok(2));
}

// ============================================================
//  add_edge – WeightedDirected
// ============================================================

#[test]
fn weighted_directed_add_edge_success() {
    let mut g = MultiGraph::<char, f64, WeightedDirected>::new();
    g.add_node('X').unwrap();
    g.add_node('Y').unwrap();

    let edge = g.add_edge('X', 'Y', 3.14).unwrap();
    assert_eq!(edge.weight, 3.14);
}

#[test]
fn weighted_directed_add_edge_source_missing() {
    let mut g = MultiGraph::<u32, u32, WeightedDirected>::new();
    g.add_node(2).unwrap();
    assert_eq!(g.add_edge(99, 2, 1), Err(GraphErrors::NodeNotFound));
}

#[test]
fn weighted_directed_add_edge_target_missing() {
    let mut g = MultiGraph::<u32, u32, WeightedDirected>::new();
    g.add_node(1).unwrap();
    assert_eq!(g.add_edge(1, 99, 1), Err(GraphErrors::NodeNotFound));
}

#[test]
fn weighted_directed_edge_is_one_way() {
    let mut g = MultiGraph::<u32, u32, WeightedDirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2, 10).unwrap();

    assert_eq!(g.degree(&1), Ok(1));
    assert_eq!(g.degree(&2), Ok(0));
}

#[test]
fn weighted_directed_multiple_edges_same_pair() {
    let mut g = MultiGraph::<u32, f64, WeightedDirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();

    g.add_edge(1, 2, 1.0).unwrap();
    g.add_edge(1, 2, 2.0).unwrap();

    assert_eq!(g.degree(&1), Ok(2));
    assert_eq!(g.degree(&2), Ok(0));
}

#[test]
fn weighted_directed_both_directions_explicit() {
    let mut g = MultiGraph::<u32, u32, WeightedDirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();

    g.add_edge(1, 2, 10).unwrap();
    g.add_edge(2, 1, 20).unwrap();

    assert_eq!(g.degree(&1), Ok(1));
    assert_eq!(g.degree(&2), Ok(1));
}

#[test]
fn weighted_directed_self_loop() {
    let mut g = MultiGraph::<u32, u32, WeightedDirected>::new();
    g.add_node(1).unwrap();
    g.add_edge(1, 1, 5).unwrap();
    assert_eq!(g.degree(&1), Ok(1));
}

// ============================================================
//  degree on non-existent node
// ============================================================

#[test]
fn degree_on_nonexistent_node() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    assert_eq!(g.degree(&999), Err(GraphErrors::NodeNotFound));
}
