use multigraphrs::core::graph_errors::GraphError;
use std::error::Error;

#[test]
fn test_graph_errors_display_and_error() {
    let not_found = GraphError::NodeNotFound;
    assert_eq!(format!("{}", not_found), "Node not found in the graph");
    assert!(not_found.source().is_none());

    let already_exists = GraphError::NodeAlreadyExists;
    assert_eq!(format!("{}", already_exists), "Node already exists in the graph");

    let edge_missing = GraphError::EdgeDoesntExist;
    assert_eq!(format!("{}", edge_missing), "Edge does not exist in the graph");
}
