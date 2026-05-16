// AI History Store - Persistence layer
//
// This crate provides:
// - SQLite storage adapter
// - Storage traits for future backends (Postgres, cloud)
// - Migrations
// - Transactional append APIs
// - Operation orchestrator for FSM command execution
// - Unit of work for atomic operations
// - Materialized view persistence
// - Archive import/export APIs

#![cfg_attr(
    test,
    allow(
        dead_code,
        unused_comparisons,
        unused_imports,
        unused_mut,
        unused_variables
    )
)]

pub mod gc;
pub mod integrity;
pub mod migrations;
pub mod orchestrator;
#[cfg(test)]
mod scope_tests;
pub mod session;
pub mod sqlite_store;
pub mod store;
pub mod traits;
pub mod unit_of_work;
pub mod views;
pub mod workspace_store;
mod workspace_store_backup;

pub use migrations::*;
pub use store::*;
// Explicitly export trait definitions (but not info types which are duplicated in sqlite_store)
pub use integrity::*;
pub use orchestrator::*;
pub use session::*;
pub use sqlite_store::*;
pub use traits::{BlobStore, BranchStore, CheckpointStore, ManifestStore, SessionStore, TurnStore};
pub use unit_of_work::*;
pub use views::*;
pub use workspace_store::*;

// Re-export info types from sqlite_store (which are canonical for the store crate)
pub use sqlite_store::{
    BlobInfo, BranchInfo, CheckpointInfo, ContextSnapshotRecord, ManifestInfo, MessageRecord,
    SessionInfo, TurnInfo,
};

// Re-export Event for use in unit of work operations
pub use ai_history_events::Event;
