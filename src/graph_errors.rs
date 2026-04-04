/// Represents all possible errors that can occur during graph operations.
#[derive(Debug, PartialEq, Eq)]
pub enum GraphErrors {
    /// Returned when attempting to add an edge between nodes that do not exist.
    NodeNotFound,
    /// Returned when attempting to add a node that is already in the adjacency list.
    NodeAlreadyExists,
    EdgeDoesntExists,
}
