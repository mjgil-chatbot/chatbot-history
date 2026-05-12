//! GC planning and execution helpers for the store layer
//!
//! This module provides store-level GC operations that wrap the core
//! GC planner and executor, adapting them to work with SqliteStore.

use crate::SqliteStore;
use ai_history_core::gc::{GcExecutor, GcOptions, GcPlan};

/// Plan garbage collection (dry-run mode)
pub fn plan_gc(store: &SqliteStore, options: &GcOptions) -> Result<GcPlan, String> {
    ai_history_core::gc::GcPlanner::plan_dry_run(store, options).map_err(|e| e.to_string())
}

/// Run garbage collection with options and return summary
pub fn run_gc(store: &SqliteStore, options: &GcOptions) -> Result<String, String> {
    GcExecutor::execute_with_summary(store, options).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_history_core::gc::{GcOptions, GcPlanner};

    #[test]
    fn test_plan_gc_empty_store() {
        let store = SqliteStore::in_memory().unwrap();
        let options = GcOptions::default();

        let plan = ai_history_core::gc::GcPlanner::plan_dry_run(&store, &options);
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        // Empty store should have nothing to delete
        assert!(!plan.would_delete());
    }

    #[test]
    fn test_plan_gc_with_session() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = ai_history_core::SessionId::new();
        store.create_session(session_id, "/tmp/test").unwrap();

        let options = GcOptions::default();
        let plan = ai_history_core::gc::GcPlanner::plan_dry_run(&store, &options);
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        // Session should be live, no orphaned data
        assert!(!plan.would_delete());
    }
}
