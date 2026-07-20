use multigraphrs::core::graph_errors::GraphErrors;
use std::error::Error;

#[test]
fn test_graph_errors_display_and_error() {
    let not_found = GraphErrors::NodeNotFound;
    assert_eq!(format!("{}", not_found), "Node not found in the graph");
    assert!(not_found.source().is_none());

    let already_exists = GraphErrors::NodeAlreadyExists;
    assert_eq!(format!("{}", already_exists), "Node already exists in the graph");

    let edge_missing = GraphErrors::EdgeDoesntExists;
    assert_eq!(format!("{}", edge_missing), "Edge does not exist in the graph");
}
