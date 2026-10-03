use std::{convert::Infallible, fmt};

use crate::core::db_error::DbError;

/// Represents all possible errors that can occur during graph operations.
///
/// This enum covers both recoverable user/logic errors (like looking up a
/// node that doesn't exist) and fatal database errors (wrapped in the
/// [`Db`](GraphError::Db) variant).
///
/// # Examples
/// ```rust,ignore
/// match result {
///     Err(GraphError::NodeNotFound) => { /* handle missing node */ }
///     Err(GraphError::Db(db_err)) => { /* fatal: inspect db_err */ }
///     _ => {}
/// }
/// ```
#[derive(Debug)]
pub enum GraphError {
    /// Returned when attempting to operate on a node that does not exist.
    NodeNotFound,

    /// Returned when attempting to add a node that is already in the graph.
    NodeAlreadyExists,

    /// Returned when attempting to remove an edge that does not exist.
    EdgeDoesntExist,

    /// A fatal database/storage error occurred.
    ///
    /// This wraps a [`DbError`] so that users of the public API can
    /// inspect the underlying cause (I/O failure, WAL corruption,
    /// poisoned state, etc.).
    Db(DbError),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::NodeNotFound => write!(f, "Node not found in the graph"),
            GraphError::NodeAlreadyExists => write!(f, "Node already exists in the graph"),
            GraphError::EdgeDoesntExist => write!(f, "Edge does not exist in the graph"),
            GraphError::Db(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl std::error::Error for GraphError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GraphError::Db(e) => Some(e),
            _ => None,
        }
    }
}

impl From<DbError> for GraphError {
    fn from(err: DbError) -> Self {
        GraphError::Db(err)
    }
}

impl PartialEq for GraphError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NodeNotFound, Self::NodeNotFound) => true,
            (Self::NodeAlreadyExists, Self::NodeAlreadyExists) => true,
            (Self::EdgeDoesntExist, Self::EdgeDoesntExist) => true,
            (Self::Db(d1), Self::Db(d2)) => d1 == d2,
            _ => false,
        }
    }
}

impl From<Infallible> for GraphError {
    fn from(err: Infallible) -> Self {
        // Since Infallible cannot exist, we use an empty match 
        // to prove to the compiler this code is unreachable.
        match err {} 
    }
}
