use multigraphrs::{Directed, MultiGraph, Undirected, Weighted, WeightedDirected};

// ============ Empty / Single Node ============

#[test]
fn iter_empty_graph() {
    let g = MultiGraph::<u32, u32, Directed>::new();
    assert_eq!(g.iter().count(), 0);
}

#[test]
fn iter_single_node_no_edges() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();
    g.add_node("A").unwrap();

    let items: Vec<_> = g.iter().collect();
    assert_eq!(items.len(), 1);
    assert_eq!(*items[0].0, "A");
    assert!(items[0].1.is_empty());
}

// ============ Directed ============

#[test]
fn iter_directed_with_edges() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();
    g.add_node("A").unwrap();
    g.add_node("B").unwrap();
    g.add_node("C").unwrap();

    g.add_edge("A", "B").unwrap();
    g.add_edge("A", "C").unwrap();

    let mut found_a = false;
    let mut found_b = false;
    let mut found_c = false;

    for (node, edges) in g.iter() {
        match *node {
            "A" => {
                found_a = true;
                assert_eq!(edges.len(), 2);
                let targets: Vec<&str> = edges.iter().map(|e| e.get_target()).collect();
                assert!(targets.contains(&"B"));
                assert!(targets.contains(&"C"));
            }
            "B" => {
                found_b = true;
                assert!(edges.is_empty()); // no outgoing edges
            }
            "C" => {
                found_c = true;
                assert!(edges.is_empty());
            }
            _ => panic!("Unexpected node"),
        }
    }

    assert!(found_a && found_b && found_c);
}

// ============ Undirected ============

#[test]
fn iter_undirected_with_edges() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();

    g.add_edge(1, 2).unwrap();

    for (node, edges) in g.iter() {
        match *node {
            1 => {
                assert_eq!(edges.len(), 1);
                assert_eq!(edges[0].get_target(), 2);
            }
            2 => {
                assert_eq!(edges.len(), 1);
                assert_eq!(edges[0].get_target(), 1);
            }
            _ => panic!("Unexpected node"),
        }
    }
}

// ============ Weighted Directed ============

#[test]
fn iter_weighted_directed() {
    let mut g = MultiGraph::<&str, f64, WeightedDirected>::new();
    g.add_node("X").unwrap();
    g.add_node("Y").unwrap();

    g.add_edge("X", "Y", 3.14).unwrap();

    for (node, edges) in g.iter() {
        match *node {
            "X" => {
                assert_eq!(edges.len(), 1);
                assert_eq!(edges[0].get_target(), "Y");
                assert_eq!(edges[0].get_weight(), 3.14);
            }
            "Y" => {
                assert!(edges.is_empty());
            }
            _ => panic!("Unexpected node"),
        }
    }
}

// ============ Weighted Undirected ============

#[test]
fn iter_weighted_undirected() {
    let mut g = MultiGraph::<u32, i32, Weighted>::new();
    g.add_node(10).unwrap();
    g.add_node(20).unwrap();

    g.add_edge(10, 20, 500).unwrap();

    for (node, edges) in g.iter() {
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].get_weight(), 500);
        match *node {
            10 => assert_eq!(edges[0].get_target(), 20),
            20 => assert_eq!(edges[0].get_target(), 10),
            _ => panic!("Unexpected node"),
        }
    }
}

// ============ After remove_node ============

#[test]
fn iter_after_remove_node() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();
    g.add_node("A").unwrap();
    g.add_node("B").unwrap();
    g.add_node("C").unwrap();

    g.add_edge("A", "B").unwrap();
    g.add_edge("A", "C").unwrap();
    g.add_edge("C", "B").unwrap();

    g.remove_node(&"B").unwrap();

    let nodes: Vec<&str> = g.iter().map(|(k, _)| *k).collect();
    assert_eq!(nodes.len(), 2);
    assert!(nodes.contains(&"A"));
    assert!(nodes.contains(&"C"));
    assert!(!nodes.contains(&"B"));

    // A should only have edge to C now
    for (node, edges) in g.iter() {
        match *node {
            "A" => {
                assert_eq!(edges.len(), 1);
                assert_eq!(edges[0].get_target(), "C");
            }
            "C" => {
                assert!(edges.is_empty());
            }
            _ => panic!("Unexpected node"),
        }
    }
}

// ============ Count matches node_count ============

#[test]
fn iter_count_matches_node_count() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_node(3).unwrap();
    g.add_node(4).unwrap();

    assert_eq!(g.iter().count(), g.node_count());

    g.remove_node(&2).unwrap();
    assert_eq!(g.iter().count(), g.node_count());
}

// ============ Parallel edges ============

#[test]
fn iter_parallel_edges() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();

    g.add_edge(1, 2).unwrap();
    g.add_edge(1, 2).unwrap();
    g.add_edge(1, 2).unwrap();

    for (node, edges) in g.iter() {
        match *node {
            1 => assert_eq!(edges.len(), 3),
            2 => assert!(edges.is_empty()),
            _ => panic!("Unexpected node"),
        }
    }
}
