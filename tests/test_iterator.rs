use multigraphrs::{MultiGraph, Directed, Weighted};

#[test]
pub fn test_into_iterator_for_graph() {
    let mut graph = MultiGraph::<u32, u32, Directed>::new();
    
    // Add 3 nodes
    graph.add_node(1).unwrap();
    graph.add_node(2).unwrap();
    graph.add_node(3).unwrap();

    // 1 -> 2
    // 1 -> 3
    // 2 -> 3
    graph.add_edge(1, 2).unwrap();
    graph.add_edge(1, 3).unwrap();
    graph.add_edge(2, 3).unwrap();

    let mut nodes_visited = 0;
    let mut total_edges_seen = 0;

    for (node, edges) in graph.iter_mut() {
        nodes_visited += 1;
        total_edges_seen += edges.len();

        match node {
            1 => {
                assert_eq!(edges.len(), 2);
                let targets: Vec<u32> = edges.iter().map(|e| e.target).collect();
                assert!(targets.contains(&2));
                assert!(targets.contains(&3));
            },
            2 => {
                assert_eq!(edges.len(), 1);
                assert_eq!(edges[0].target, 3);
            },
            3 => {
                assert_eq!(edges.len(), 0);
            },
            _ => panic!("Unexpected node found in graph!"),
        }
    }

    assert_eq!(nodes_visited, 3, "Should have iterated over exactly 3 nodes");
    assert_eq!(total_edges_seen, 3, "Should have seen exactly 3 total outgoing edges");
}

#[test]
pub fn test_explicit_iter_method() {
    let mut graph = MultiGraph::<&str, f64, Weighted>::new();
    
    graph.add_node("A").unwrap();
    graph.add_node("B").unwrap();
    
    graph.add_edge("A", "B", 99.9).unwrap();

    let mut iter = graph.iter();
    
    let (first_node, first_edges) = iter.next().expect("Expected a first node");
    assert_eq!(first_edges.len(), 1);
    assert_eq!(first_edges[0].weight, 99.9);

    let (second_node, second_edges) = iter.next().expect("Expected a second node");
    assert_eq!(second_edges.len(), 1);
    assert_eq!(second_edges[0].weight, 99.9);

    assert!(first_node != second_node, "Nodes should be different");
    
    assert!(iter.next().is_none());
}
