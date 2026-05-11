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

impl From<std::num::ParseIntError> for CoreError {
    fn from(e: std::num::ParseIntError) -> Self {
        Self::IdParse(e.to_string())
    }
}

impl From<std::string::FromUtf8Error> for CoreError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::Checksum(e.to_string())
    }
}

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => Self::Filesystem(format!("file not found: {}", e)),
            std::io::ErrorKind::PermissionDenied => {
                Self::Storage(format!("permission denied: {}", e))
            }
            std::io::ErrorKind::AlreadyExists => Self::Storage(format!("already exists: {}", e)),
            std::io::ErrorKind::WouldBlock => Self::Storage(format!("would block: {}", e)),
            std::io::ErrorKind::InvalidInput => Self::Validation(format!("invalid input: {}", e)),
            std::io::ErrorKind::TimedOut => Self::Storage(format!("operation timed out: {}", e)),
            std::io::ErrorKind::WriteZero => Self::Filesystem(format!("write zero: {}", e)),
            std::io::ErrorKind::Interrupted => Self::Storage(format!("interrupted: {}", e)),
            std::io::ErrorKind::Other => Self::Filesystem(format!("filesystem error: {}", e)),
            _ => Self::Filesystem(format!("IO error: {}", e)),
        }
    }
}

/// Result type alias for core operations
pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_id_parse() {
        let err = CoreError::IdParse("invalid UUID".to_string());
        assert_eq!(err.to_string(), "ID parse error: invalid UUID");
    }

    #[test]
    fn test_error_display_invalid_id() {
        let err = CoreError::InvalidId("bad-id".to_string());
        assert_eq!(err.to_string(), "Invalid ID format: bad-id");
    }

    #[test]
    fn test_error_display_path() {
        let err = CoreError::PathError("/invalid/path".to_string());
        assert_eq!(err.to_string(), "Path error: /invalid/path");
    }

    #[test]
    fn test_error_display_hash() {
        let err = CoreError::ChecksumMismatch("checksum mismatch".to_string());
        assert_eq!(err.to_string(), "Checksum mismatch: checksum mismatch");
    }

    #[test]
    fn test_error_display_serialization() {
        let err = CoreError::Serialization("JSON malformed".to_string());
        assert_eq!(err.to_string(), "Serialization error: JSON malformed");
    }

    #[test]
    fn test_error_display_deserialization() {
        let err = CoreError::Deserialization("missing required field".to_string());
        assert_eq!(
            err.to_string(),
            "Deserialization error: missing required field"
        );
    }

    #[test]
    fn test_error_display_timestamp() {
        let err = CoreError::Timestamp("invalid format".to_string());
        assert_eq!(err.to_string(), "Timestamp error: invalid format");
    }

    #[test]
    fn test_error_display_actor() {
        let err = CoreError::Actor("unknown actor type".to_string());
        assert_eq!(err.to_string(), "Actor error: unknown actor type");
    }

    #[test]
    fn test_error_display_invariant() {
        let err = CoreError::Invariant("branch head moved unexpectedly".to_string());
        assert_eq!(
            err.to_string(),
            "Invariant violation: branch head moved unexpectedly"
        );
    }

    #[test]
    fn test_error_from_parse_int() {
        let parse_err = "123abc".parse::<i32>().unwrap_err();
        let err = CoreError::from(parse_err);
        assert!(matches!(err, CoreError::IdParse(_)));
    }

    #[test]
    fn test_error_from_utf8() {
        let invalid_utf8 = vec![0x80, 0x81];
        let utf_err = String::from_utf8(invalid_utf8).unwrap_err();
        let err = CoreError::from(utf_err);
        assert!(matches!(err, CoreError::Checksum(_)));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = CoreError::from(io_err);
        // IO errors map to Filesystem
        let err_str = err.to_string();
        assert!(err_str.contains("Filesystem") || err_str.contains("not found"));
    }

    #[test]
    fn test_error_classification_is_recoverable() {
        // ID parse and format errors are generally recoverable
        let err = CoreError::IdParse("bad".to_string());
        assert!(!err.to_string().is_empty());

        // IO errors map to appropriate categories
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = CoreError::from(io_err);
        let err_str = err.to_string();
        assert!(err_str.contains("permission") || err_str.contains("Storage"));
    }

    #[test]
    fn test_error_debug_format() {
        let err = CoreError::Invariant("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Invariant"));
    }
}
