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
pub mod fsm_model;
pub mod gc;
pub mod hash;
pub mod id;
pub mod ignore_rules;
pub mod implementation_matrix;
pub mod integrity;
pub mod invariants;
pub mod materialized_views;
pub mod message;
pub mod timestamp;
pub use accessibility::*;
pub use actor::*;
pub use blob_store::*;
pub use branch::*;
pub use completion_gate::*;
pub use concurrency::*;
pub use design_review::*;
pub use doc_completeness::*;
pub use error::*;
pub use export::*;
pub use gc::*;
pub use hash::*;
pub use id::*;
pub use integrity::*;
pub use invariants::*;
pub use timestamp::*;
pub use benchmark::*;
pub use canonical_ownership::*;
pub use checkpoint::*;
pub use conflict_resolution::*;
pub use context_snapshot::*;
pub use cursor_navigation::*;
pub use dependency_rules::*;
pub use dirty_workspace::*;
pub use docs_gate::*;
pub use event_log::{
    EventId, EventOrdering, EventSchemaValidator, PayloadVersion, ReplayResult, SequenceNumber,
};
pub use fsm_model::*;
pub use ignore_rules::*;
pub use implementation_matrix::*;
