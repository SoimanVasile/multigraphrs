use multigraphrs::{Directed, GraphErrors, MultiGraph, Undirected, Weighted, WeightedDirected};

// --- Directed ---

#[test]
fn neighbours_directed_basic() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();
    g.add_node("A").unwrap();
    g.add_node("B").unwrap();
    g.add_node("C").unwrap();

    g.add_edge("A", "B").unwrap();
    g.add_edge("A", "C").unwrap();

    let neighbours = g.get_neighbours(&"A").unwrap();
    assert_eq!(neighbours.len(), 2);

    let targets: Vec<&str> = neighbours.iter().map(|e| e.get_target()).collect();
    assert!(targets.contains(&"B"));
    assert!(targets.contains(&"C"));
}

#[test]
fn neighbours_directed_no_edges() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();

    let neighbours = g.get_neighbours(&1).unwrap();
    assert!(neighbours.is_empty());
}

#[test]
fn neighbours_directed_only_outgoing() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2).unwrap();

    // Node 2 has no outgoing edges
    let neighbours = g.get_neighbours(&2).unwrap();
    assert!(neighbours.is_empty());
}

// --- Undirected ---

#[test]
fn neighbours_undirected_basic() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_node(3).unwrap();

    g.add_edge(1, 2).unwrap();
    g.add_edge(1, 3).unwrap();

    let n1 = g.get_neighbours(&1).unwrap();
    assert_eq!(n1.len(), 2);

    // Node 2 should see node 1 as a neighbour
    let n2 = g.get_neighbours(&2).unwrap();
    assert_eq!(n2.len(), 1);
    assert_eq!(n2[0].get_target(), 1);
}

// --- WeightedDirected ---

#[test]
fn neighbours_weighted_directed_basic() {
    let mut g = MultiGraph::<&str, f64, WeightedDirected>::new();
    g.add_node("X").unwrap();
    g.add_node("Y").unwrap();

    g.add_edge("X", "Y", 3.14).unwrap();

    let neighbours = g.get_neighbours(&"X").unwrap();
    assert_eq!(neighbours.len(), 1);
    assert_eq!(neighbours[0].get_target(), "Y");
    assert_eq!(neighbours[0].get_weight(), 3.14);
}

// --- Weighted (undirected) ---

#[test]
fn neighbours_weighted_basic() {
    let mut g = MultiGraph::<u32, i32, Weighted>::new();
    g.add_node(10).unwrap();
    g.add_node(20).unwrap();

    g.add_edge(10, 20, 500).unwrap();

    let n10 = g.get_neighbours(&10).unwrap();
    assert_eq!(n10.len(), 1);
    assert_eq!(n10[0].get_target(), 20);
    assert_eq!(n10[0].get_weight(), 500);

    let n20 = g.get_neighbours(&20).unwrap();
    assert_eq!(n20.len(), 1);
    assert_eq!(n20[0].get_target(), 10);
}

// --- Error paths ---

#[test]
fn neighbours_nonexistent_node_directed() {
    let g = MultiGraph::<u32, u32, Directed>::new();
    assert_eq!(g.get_neighbours(&999), Err(GraphErrors::NodeNotFound));
}

#[test]
fn neighbours_nonexistent_node_undirected() {
    let g = MultiGraph::<u32, u32, Undirected>::new();
    assert_eq!(g.get_neighbours(&1), Err(GraphErrors::NodeNotFound));
}

#[test]
fn neighbours_nonexistent_node_weighted() {
    let g = MultiGraph::<u32, f64, Weighted>::new();
    assert_eq!(g.get_neighbours(&1), Err(GraphErrors::NodeNotFound));
}

#[test]
fn neighbours_nonexistent_node_weighted_directed() {
    let g = MultiGraph::<u32, f64, WeightedDirected>::new();
    assert_eq!(g.get_neighbours(&1), Err(GraphErrors::NodeNotFound));
}

// --- After remove_node ---

#[test]
fn neighbours_after_removing_neighbour() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();
    g.add_node("A").unwrap();
    g.add_node("B").unwrap();
    g.add_node("C").unwrap();

    g.add_edge("A", "B").unwrap();
    g.add_edge("A", "C").unwrap();

    g.remove_node(&"B").unwrap();

    let neighbours = g.get_neighbours(&"A").unwrap();
    assert_eq!(neighbours.len(), 1);
    assert_eq!(neighbours[0].get_target(), "C");
}

// --- Parallel edges ---

#[test]
fn neighbours_with_parallel_edges() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();

    g.add_edge(1, 2).unwrap();
    g.add_edge(1, 2).unwrap();
    g.add_edge(1, 2).unwrap();

    let neighbours = g.get_neighbours(&1).unwrap();
    assert_eq!(neighbours.len(), 3);
}
