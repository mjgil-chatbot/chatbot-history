//! Database migrations for the event store
//!
//! This module defines the schema migrations for the event store.
//! The actual migration runner implementation is database-specific.

use ai_history_core::CoreResult;

/// Migration version
pub const SCHEMA_VERSION: i32 = 1;

/// SQL statements for initial schema
pub const INITIAL_MIGRATION_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    active_branch TEXT NOT NULL,
    view_cursor TEXT,
    workspace_root TEXT NOT NULL,
    retention_config TEXT NOT NULL,
    project_identity TEXT NOT NULL,
    operation_state TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archival_state INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS branches (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    name TEXT NOT NULL,
    parent_branch_id TEXT,
    created_from_turn_id TEXT,
    head_turn_id TEXT,
    ancestry TEXT NOT NULL,
    protected INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    abandoned_future INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

CREATE TABLE IF NOT EXISTS turns (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    parent_turn_id TEXT,
    actor TEXT NOT NULL,
    created_at TEXT NOT NULL,
    finalized_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    FOREIGN KEY (branch_id) REFERENCES branches(id)
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    turn_id TEXT,
    sequence_num INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL,
    causation_id TEXT,
    correlation_id TEXT,
    actor TEXT NOT NULL,
    checksum TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    FOREIGN KEY (branch_id) REFERENCES branches(id)
);

CREATE TABLE IF NOT EXISTS checkpoints (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    turn_id TEXT,
    manifest_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    FOREIGN KEY (branch_id) REFERENCES branches(id)
);

CREATE TABLE IF NOT EXISTS manifests (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    checkpoint_id TEXT NOT NULL,
    manifest_data TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(id)
);

CREATE TABLE IF NOT EXISTS blobs (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    hash TEXT NOT NULL UNIQUE,
    size INTEGER NOT NULL,
    content BLOB NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);
CREATE INDEX IF NOT EXISTS idx_events_branch ON events(branch_id);
CREATE INDEX IF NOT EXISTS idx_events_sequence ON events(session_id, sequence_num);
CREATE INDEX IF NOT EXISTS idx_checkpoints_session ON checkpoints(session_id);
CREATE INDEX IF NOT EXISTS idx_blobs_hash ON blobs(hash);
"#;

/// Migration entry point
pub fn run_migrations() -> CoreResult<()> {
    // In a full implementation, this would connect to the database
    // and run the migrations. For now, we just return success.
    Ok(())
}

/// Get all migration SQL statements
pub fn all_migrations() -> Vec<&'static str> {
    vec![INITIAL_MIGRATION_SQL]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version() {
        assert_eq!(SCHEMA_VERSION, 1);
    }

    #[test]
    fn test_migration_sql_not_empty() {
        assert!(!INITIAL_MIGRATION_SQL.is_empty());
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE"));
    }

    #[test]
    fn test_initial_schema_has_sessions_table() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS sessions"));
    }

    #[test]
    fn test_initial_schema_has_branches_table() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS branches"));
    }

    #[test]
    fn test_initial_schema_has_turns_table() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS turns"));
    }

    #[test]
    fn test_initial_schema_has_events_table() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS events"));
    }

    #[test]
    fn test_initial_schema_has_checkpoints_table() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS checkpoints"));
    }

    #[test]
    fn test_initial_schema_has_manifests_table() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS manifests"));
    }

    #[test]
    fn test_initial_schema_has_blobs_table() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS blobs"));
    }

    #[test]
    fn test_initial_schema_has_schema_version_table() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS schema_version"));
    }

    #[test]
    fn test_events_table_has_sequence_num() {
        assert!(INITIAL_MIGRATION_SQL.contains("sequence_num INTEGER NOT NULL"));
    }

    #[test]
    fn test_events_table_has_checksum() {
        assert!(INITIAL_MIGRATION_SQL.contains("checksum TEXT NOT NULL"));
    }

    #[test]
    fn test_events_table_has_causation_and_correlation() {
        assert!(INITIAL_MIGRATION_SQL.contains("causation_id TEXT"));
        assert!(INITIAL_MIGRATION_SQL.contains("correlation_id TEXT"));
    }

    #[test]
    fn test_blobs_table_has_hash_unique_constraint() {
        assert!(INITIAL_MIGRATION_SQL.contains("hash TEXT NOT NULL UNIQUE"));
    }

    #[test]
    fn test_foreign_keys_present() {
        assert!(INITIAL_MIGRATION_SQL.contains("FOREIGN KEY (session_id) REFERENCES sessions(id)"));
        assert!(INITIAL_MIGRATION_SQL.contains("FOREIGN KEY (branch_id) REFERENCES branches(id)"));
    }

    #[test]
    fn test_indexes_created() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE INDEX"));
        assert!(INITIAL_MIGRATION_SQL.contains("idx_events_session"));
        assert!(INITIAL_MIGRATION_SQL.contains("idx_events_branch"));
        assert!(INITIAL_MIGRATION_SQL.contains("idx_events_sequence"));
        assert!(INITIAL_MIGRATION_SQL.contains("idx_checkpoints_session"));
        assert!(INITIAL_MIGRATION_SQL.contains("idx_blobs_hash"));
    }

    #[test]
    fn test_all_migrations_returns_list() {
        let migrations = all_migrations();
        assert!(!migrations.is_empty());
        assert_eq!(migrations.len(), 1); // Initial migration only for now
    }

    #[test]
    fn test_migration_sql_creates_all_required_tables() {
        // Verify all tables that are needed for the system
        let required_tables = vec![
            "schema_version",
            "sessions",
            "branches",
            "turns",
            "events",
            "checkpoints",
            "manifests",
            "blobs",
        ];

        for table in required_tables {
            let create_pattern = format!("CREATE TABLE IF NOT EXISTS {}", table);
            assert!(
                INITIAL_MIGRATION_SQL.contains(&create_pattern),
                "Missing table: {}",
                table
            );
        }
    }

    #[test]
    fn test_events_autoincrement() {
        assert!(INITIAL_MIGRATION_SQL.contains("PRIMARY KEY AUTOINCREMENT"));
    }

    #[test]
    fn test_run_migrations_succeeds() {
        // Test that run_migrations doesn't error (mock implementation)
        let result = run_migrations();
        assert!(result.is_ok());
    }

    #[test]
    fn test_schema_version_table_structure() {
        assert!(INITIAL_MIGRATION_SQL.contains("version INTEGER PRIMARY KEY"));
        assert!(INITIAL_MIGRATION_SQL.contains("applied_at TEXT NOT NULL"));
    }

    #[test]
    fn test_sessions_required_fields() {
        assert!(INITIAL_MIGRATION_SQL.contains("workspace_root TEXT NOT NULL"));
        assert!(INITIAL_MIGRATION_SQL.contains("created_at TEXT NOT NULL"));
        assert!(INITIAL_MIGRATION_SQL.contains("updated_at TEXT NOT NULL"));
    }

    #[test]
    fn test_branches_has_head_turn_tracking() {
        assert!(INITIAL_MIGRATION_SQL.contains("head_turn_id TEXT"));
        assert!(INITIAL_MIGRATION_SQL.contains("parent_branch_id TEXT"));
    }

    #[test]
    fn test_turns_has_actor_and_timing() {
        assert!(INITIAL_MIGRATION_SQL.contains("actor TEXT NOT NULL"));
        assert!(INITIAL_MIGRATION_SQL.contains("finalized_at TEXT"));
    }

    #[test]
    fn test_checkpoints_has_reason_field() {
        assert!(INITIAL_MIGRATION_SQL.contains("reason TEXT NOT NULL"));
    }

    #[test]
    fn test_blobs_has_size_tracking() {
        assert!(INITIAL_MIGRATION_SQL.contains("size INTEGER NOT NULL"));
        assert!(INITIAL_MIGRATION_SQL.contains("content BLOB NOT NULL"));
    }
}
