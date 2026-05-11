//! Error types for ai-history system
//!
//! Task 9: Finalize error taxonomy with structured error categories:
//! - Storage: database and persistence errors
//! - Filesystem: file operations and path errors
//! - Checksum: hash verification and integrity errors
//! - Migration: schema migration errors
//! - Restore: workspace restore errors
//! - Validation: input validation errors

use thiserror::Error;
/// Core error types for the ai-history system
#[derive(Error, Debug)]
pub enum CoreError {
    // ID/parsing errors
    #[error("ID parse error: {0}")]
    IdParse(String),

    #[error("Invalid ID format: {0}")]
    InvalidId(String),

    // Storage errors (database, persistence)
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Migration error: {0}")]
    Migration(String),

    // Filesystem errors
    #[error("Filesystem error: {0}")]
    Filesystem(String),

    #[error("Path error: {0}")]
    PathError(String),

    // Checksum/integrity errors
    #[error("Checksum error: {0}")]
    Checksum(String),

    #[error("Checksum mismatch: {0}")]
    ChecksumMismatch(String),

    // Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    // Timestamp errors
    #[error("Timestamp error: {0}")]
    Timestamp(String),

    // Actor/state errors
    #[error("Actor error: {0}")]
    Actor(String),

    // Invariant violations
    #[error("Invariant violation: {0}")]
    Invariant(String),

    // Sequence errors
    #[error("Invalid sequence: {0}")]
    InvalidSequence(String),

    // Version errors
    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    // Restore errors
    #[error("Restore error: {0}")]
    Restore(String),

    // Validation errors
    #[error("Validation error: {0}")]
    Validation(String),
}
/// Result type alias for core operations
pub type CoreResult<T> = Result<T, CoreError>;
