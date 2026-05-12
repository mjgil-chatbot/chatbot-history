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

pub mod integrity;
pub mod migrations;
pub mod traits;
pub use integrity::*;
pub use ai_history_events::Event;
