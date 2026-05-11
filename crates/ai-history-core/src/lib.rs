// AI History Core - Shared domain types for the ai-history system
//
// This crate contains:
// - Typed IDs (SessionId, BranchId, TurnId, CheckpointId, BlobId, etc.)
// - Timestamp types
// - Error types
// - Common domain traits
// - Actor types
// - Path types
// - Hash types

#![cfg_attr(
    test,
    allow(
        dead_code,
        unused_comparisons,
        unused_imports,
        unused_must_use,
        unused_mut,
        unused_variables
    )
)]

pub mod accessibility;
pub mod actor;
pub mod benchmark;
pub mod blob_store;
pub mod branch;
pub mod canonical_ownership;
pub mod checkpoint;
pub mod completion_gate;
pub mod concurrency;
pub mod conflict_resolution;
pub mod context_snapshot;
pub mod cursor_navigation;
pub mod dependency_rules;
pub mod design_review;
pub mod dirty_workspace;
pub mod doc_completeness;
pub mod docs_gate;
pub mod error;
pub mod event_log;
pub mod export;
pub mod hash;
pub mod id;
pub mod timestamp;
pub use actor::*;
pub use error::*;
pub use hash::*;
pub use id::*;
pub use timestamp::*;
