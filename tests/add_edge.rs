use multigraphrs::MultiGraph;
use multigraphrs::graph_errors::GraphErrors;
use multigraphrs::weighted::Weighted;
use multigraphrs::directed::Directed;
use multigraphrs::undirected::Undirected;
use multigraphrs::weighted_directed::WeightedDirected;

#[test]
pub fn test_add_nodes_and_errors() {
    let mut graph = MultiGraph::<u32, u32, Weighted>::new();
    
    // Test successful node insertion
    assert_eq!(graph.add_node(1), Ok(1));
    assert_eq!(graph.add_node(2), Ok(2));
    
    // Test error on duplicate node
    assert_eq!(graph.add_node(1), Err(GraphErrors::NodeAlreadyExists));
}

#[test]
pub fn test_weighted_graph() {
    // Weighted creates 2 edges (source->target and target->source) with the specified weight
    let mut graph = MultiGraph::<u32, i32, Weighted>::new();
    
    graph.add_node(10).unwrap();
    graph.add_node(20).unwrap();

    let edges = graph.add_edge(10, 20, 500).unwrap();
    
    assert_eq!(edges.len(), 2); // Should return both edges
    assert_eq!(edges[0].target, 1);
    assert_eq!(edges[0].weight, 500);
    assert_eq!(edges[1].target, 0);
    assert_eq!(edges[1].weight, 500);

    // Test missing node error
    assert_eq!(graph.add_edge(10, 99, 100).unwrap_err(), GraphErrors::NodeNotFound);
}

#[test]
pub fn test_directed_graph() {
    // Directed takes only 2 params for add_edge and defaults the weight to 1
    let mut graph = MultiGraph::<&str, u32, Directed>::new();
    
    graph.add_node("A").unwrap();
    graph.add_node("B").unwrap();

    let edges = graph.add_edge("A", "B").unwrap(); // Only 2 arguments!
    
    assert_eq!(edges.len(), 1); // Directed means only 1 edge is created
    assert_eq!(edges[0].target, 1);
    assert_eq!(edges[0].weight, 1);
}

#[test]
pub fn test_weighted_directed_graph() {
    // WeightedDirected takes 3 params and creates 1 edge
    let mut graph = MultiGraph::<char, f64, WeightedDirected>::new();
    
    graph.add_node('X').unwrap();
    graph.add_node('Y').unwrap();

    let edges = graph.add_edge('X', 'Y', 3.14).unwrap();
    
    assert_eq!(edges.len(), 1); 
    assert_eq!(edges[0].target, 1);
    assert_eq!(edges[0].weight, 3.14);
}

#[test]
pub fn test_undirected_graph() {
    // Undirected takes 2 params and creates 2 edges, defaulting weight to 1
    let mut graph = MultiGraph::<i32, u32, Undirected>::new();
    
    graph.add_node(100).unwrap();
    graph.add_node(200).unwrap();

    let edges = graph.add_edge(100, 200).unwrap(); // Only 2 arguments!
    
    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0].target, 1);
    assert_eq!(edges[0].weight, 1);
    assert_eq!(edges[1].target, 0);
    assert_eq!(edges[1].weight, 1);
}
