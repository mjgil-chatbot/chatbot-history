//! Result type for the ai-history system

use crate::error::CoreError;

/// Result type alias for core operations
pub type Result<T> = std::result::Result<T, CoreError>;
