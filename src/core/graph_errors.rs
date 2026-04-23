use std::fmt;

/// Represents all possible errors that can occur during graph operations.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GraphErrors {
    /// Returned when attempting to add an edge between nodes that do not exist.
    NodeNotFound,
    /// Returned when attempting to add a node that is already in the adjacency list.
    NodeAlreadyExists,
    /// Returned when attempting to remove an edge that does not exist.
    EdgeDoesntExists,
}

impl fmt::Display for GraphErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphErrors::NodeNotFound => write!(f, "Node not found in the graph"),
            GraphErrors::NodeAlreadyExists => write!(f, "Node already exists in the graph"),
            GraphErrors::EdgeDoesntExists => write!(f, "Edge does not exist in the graph"),
        }
    }
}

impl std::error::Error for GraphErrors {}
