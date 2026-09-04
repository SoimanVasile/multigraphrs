use std::{fmt, sync::mpsc::{RecvError, SendError}};

use crate::storage::disk_storage::file_manager::FMRequest;

/// Represents fatal engine, I/O, and storage errors.
///
/// These errors indicate hardware, OS, file corruption, or internal
/// storage logic failures that prevent the database from operating correctly.
///
/// When a `DbError` occurs during a write operation, the database
/// enters a **poisoned** state and will refuse further writes to prevent
/// data corruption.
#[derive(Debug)]
pub enum DbError {
    /// An underlying I/O operation failed.
    Io(std::io::Error),

    /// The write-ahead log file is corrupted and cannot be replayed.
    CorruptedWal(String),

    /// An internal file ID does not map to a valid storage file.
    InvalidFileId(u8),

    /// The WAL thread is dead and cannot process further write-ahead logs.
    WalThreadDead,

    /// The database has been halted due to a previous fatal I/O error.
    ///
    /// Once poisoned, the database will not accept any further write operations.
    /// The only safe action is to close and re-open the database.
    Poisoned,
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::Io(e) => write!(f, "I/O error: {}", e),
            DbError::CorruptedWal(msg) => write!(f, "WAL is corrupted: {}", msg),
            DbError::InvalidFileId(id) => write!(f, "Invalid internal file ID: {}", id),
            DbError::Poisoned => write!(f, "Database is poisoned due to a previous fatal I/O error"),
            DbError::WalThreadDead => write!(f, "WAL thread is dead and cant write in log"),
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DbError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for DbError {
    fn from(err: std::io::Error) -> Self {
        DbError::Io(err)
    }
}

impl From<RecvError> for DbError {
    fn from(_: RecvError) -> Self {
        DbError::WalThreadDead
    }
}

impl From<SendError<FMRequest>> for DbError {
    fn from(_: SendError<FMRequest>) -> Self {
        DbError::WalThreadDead
    }
}

impl PartialEq for DbError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Io(e1), Self::Io(e2)) => e1.kind() == e2.kind(),
            (Self::CorruptedWal(m1), Self::CorruptedWal(m2)) => m1 == m2,
            (Self::InvalidFileId(id1), Self::InvalidFileId(id2)) => id1 == id2,
            (Self::Poisoned, Self::Poisoned) => true,
            _ => false,
        }
    }
}
