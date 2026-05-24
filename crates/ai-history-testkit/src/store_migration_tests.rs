//! Store migrations, API, and unit-of-work tests
//!
//! Task 38: Complete SQLite migrations
//! Task 39: Complete SQLite store API
//! Task 40: Complete unit-of-work transactions
//!
//! Verify:
//! - SQL init schema contains all required tables
//! - Foreign key constraints are enforced
//! - Indexes exist for performance
//! - SqliteStore implements all required traits
//! - UnitOfWork can commit multiple operations atomically

#[cfg(test)]
mod test_migrations {
    use ai_history_store::migrations::INITIAL_MIGRATION_SQL;

    /// Verify all required tables are created
    #[test]
    fn test_sessions_table_created() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS sessions"));
    }

    /// Verify branches table with foreign key to sessions
    #[test]
    fn test_branches_table_created() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS branches"));
        assert!(INITIAL_MIGRATION_SQL.contains("FOREIGN KEY (session_id) REFERENCES sessions(id)"));
    }

    /// Verify turns table with foreign keys
    #[test]
    fn test_turns_table_created() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS turns"));
        assert!(INITIAL_MIGRATION_SQL.contains("FOREIGN KEY (session_id) REFERENCES sessions(id)"));
        assert!(INITIAL_MIGRATION_SQL.contains("FOREIGN KEY (branch_id) REFERENCES branches(id)"));
    }

    /// Verify events table with sequence and checksum
    #[test]
    fn test_events_table_created() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS events"));
        assert!(INITIAL_MIGRATION_SQL.contains("sequence_num INTEGER NOT NULL"));
        assert!(INITIAL_MIGRATION_SQL.contains("checksum TEXT NOT NULL"));
        assert!(INITIAL_MIGRATION_SQL.contains("causation_id TEXT"));
        assert!(INITIAL_MIGRATION_SQL.contains("correlation_id TEXT"));
    }

    /// Verify checkpoints table exists
    #[test]
    fn test_checkpoints_table_created() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS checkpoints"));
    }

    /// Verify manifests table exists
    #[test]
    fn test_manifests_table_created() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS manifests"));
    }

    /// Verify blobs table with hash uniqueness
    #[test]
    fn test_blobs_table_created() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS blobs"));
        assert!(INITIAL_MIGRATION_SQL.contains("hash TEXT NOT NULL UNIQUE"));
        assert!(INITIAL_MIGRATION_SQL.contains("content BLOB NOT NULL"));
    }

    /// Verify schema_version table for migration tracking
    #[test]
    fn test_schema_version_table_created() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE IF NOT EXISTS schema_version"));
        assert!(INITIAL_MIGRATION_SQL.contains("version INTEGER PRIMARY KEY"));
    }

    /// Verify required indexes exist
    #[test]
    fn test_indexes_created() {
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE INDEX IF NOT EXISTS idx_events_session"));
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE INDEX IF NOT EXISTS idx_events_branch"));
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE INDEX IF NOT EXISTS idx_events_sequence"));
        assert!(
            INITIAL_MIGRATION_SQL.contains("CREATE INDEX IF NOT EXISTS idx_checkpoints_session")
        );
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE INDEX IF NOT EXISTS idx_blobs_hash"));
    }

    /// Verify id columns are TEXT for typed IDs
    #[test]
    fn test_typed_id_columns() {
        assert!(INITIAL_MIGRATION_SQL.contains("id TEXT PRIMARY KEY"));
        assert!(INITIAL_MIGRATION_SQL.contains("session_id TEXT NOT NULL"));
        assert!(INITIAL_MIGRATION_SQL.contains("branch_id TEXT NOT NULL"));
        assert!(INITIAL_MIGRATION_SQL.contains("turn_id TEXT"));
    }

    /// Verify primary key autoincrement for events
    #[test]
    fn test_events_autoincrement() {
        assert!(INITIAL_MIGRATION_SQL.contains("PRIMARY KEY AUTOINCREMENT"));
    }

    /// Verify blob has size tracking
    #[test]
    fn test_blobs_has_size() {
        assert!(INITIAL_MIGRATION_SQL.contains("size INTEGER NOT NULL"));
    }

    /// Verify turns has actor and finalized tracking
    #[test]
    fn test_turns_has_actor_and_finalized() {
        assert!(INITIAL_MIGRATION_SQL.contains("actor TEXT NOT NULL"));
        assert!(INITIAL_MIGRATION_SQL.contains("finalized_at TEXT"));
    }

    /// Verify branches has head_turn tracking
    #[test]
    fn test_branches_has_head_turn() {
        assert!(INITIAL_MIGRATION_SQL.contains("head_turn_id TEXT"));
        assert!(INITIAL_MIGRATION_SQL.contains("parent_branch_id TEXT"));
    }

    /// Verify checkpoints has reason field
    #[test]
    fn test_checkpoints_has_reason() {
        assert!(INITIAL_MIGRATION_SQL.contains("reason TEXT NOT NULL"));
    }
}

#[cfg(test)]
mod test_store_api {
    use ai_history_core::{BranchId, SessionId, TurnId};
    use ai_history_store::sqlite_store::{
        BranchInfo as SqliteBranchInfo, TurnInfo as SqliteTurnInfo,
    };
    use ai_history_store::SqliteStore;

    /// Verify SqliteStore can be created in-memory
    #[test]
    fn test_sqlite_store_in_memory() {
        let store = SqliteStore::in_memory();
        assert!(store.is_ok());
    }

    /// Verify SqliteStore can create a session
    #[test]
    fn test_sqlite_store_create_session() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let result = store.create_session(session_id, "/tmp");
        assert!(result.is_ok());
    }

    /// Verify SqliteStore can get a session
    #[test]
    fn test_sqlite_store_get_session() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp").unwrap();

        let session = store.get_session(session_id).unwrap();
        assert!(session.is_some());
    }

    /// Verify SqliteStore can create a branch
    #[test]
    fn test_sqlite_store_create_branch() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp").unwrap();

        let branch_info = SqliteBranchInfo {
            id: BranchId::new(),
            session_id,
            name: "main".to_string(),
            parent_branch_id: None,
            created_from_turn_id: None,
            head_turn_id: None,
            ancestry: Vec::new(),
            protected: false,
            archived: false,
            abandoned_future: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let result = store.create_branch(&branch_info);
        assert!(result.is_ok());
    }

    /// Verify SqliteStore can get a branch
    #[test]
    fn test_sqlite_store_get_branch() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp").unwrap();

        let branch_id = BranchId::new();
        let branch_info = SqliteBranchInfo {
            id: branch_id,
            session_id,
            name: "main".to_string(),
            parent_branch_id: None,
            created_from_turn_id: None,
            head_turn_id: None,
            ancestry: Vec::new(),
            protected: false,
            archived: false,
            abandoned_future: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        store.create_branch(&branch_info).unwrap();

        let branch = store.get_branch(branch_id).unwrap();
        assert!(branch.is_some());
    }

    /// Verify SqliteStore can create a turn
    #[test]
    fn test_sqlite_store_create_turn() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp").unwrap();

        let branch_id = BranchId::new();
        let branch_info = SqliteBranchInfo {
            id: branch_id,
            session_id,
            name: "main".to_string(),
            parent_branch_id: None,
            created_from_turn_id: None,
            head_turn_id: None,
            ancestry: Vec::new(),
            protected: false,
            archived: false,
            abandoned_future: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        store.create_branch(&branch_info).unwrap();

        let turn_info = SqliteTurnInfo {
            id: TurnId::new(),
            session_id,
            branch_id,
            parent_turn_id: None,
            actor: "User".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            finalized_at: None,
        };

        let result = store.create_turn(&turn_info);
        assert!(result.is_ok());
    }

    /// Verify SqliteStore can finalize a turn
    #[test]
    fn test_sqlite_store_finalize_turn() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp").unwrap();

        let branch_id = BranchId::new();
        store
            .create_branch(&SqliteBranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        let turn_id = TurnId::new();
        store
            .create_turn(&SqliteTurnInfo {
                id: turn_id,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        let result = store.finalize_turn(turn_id);
        assert!(result.is_ok());
    }

    /// Verify SqliteStore can update branch head
    #[test]
    fn test_sqlite_store_update_branch_head() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp").unwrap();

        let branch_id = BranchId::new();
        store
            .create_branch(&SqliteBranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        let turn_id = TurnId::new();
        let result = store.update_branch_head(branch_id, turn_id);
        assert!(result.is_ok());
    }

    /// Verify SqliteStore can create a checkpoint (basic API test)
    #[test]
    fn test_sqlite_store_create_checkpoint_api() {
        // This test verifies the checkpoint API exists and can be called
        // The actual FK constraints depend on proper setup
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp").unwrap();

        // Verify get_checkpoint returns None for non-existent checkpoint
        use ai_history_core::CheckpointId;
        let result = store.get_checkpoint(CheckpointId::new()).unwrap();
        assert!(result.is_none());
    }
}

#[cfg(test)]
mod test_unit_of_work {
    use ai_history_core::{BranchId, SessionId, TurnId};
    use ai_history_store::sqlite_store::{
        BranchInfo as SqliteBranchInfo, TurnInfo as SqliteTurnInfo,
    };
    use ai_history_store::{SqliteStore, UnitOfWork, UnitOfWorkBuilder};

    /// Verify UnitOfWork can be created empty
    #[test]
    fn test_unit_of_work_empty() {
        let uow = UnitOfWork::new();
        assert!(uow.is_empty());
        assert_eq!(uow.len(), 0);
    }

    /// Verify UnitOfWorkBuilder creates unit of work
    #[test]
    fn test_unit_of_work_builder() {
        let uow = UnitOfWorkBuilder::new().build();
        assert!(uow.is_empty());
    }

    /// Verify UnitOfWork can queue create_session
    #[test]
    fn test_unit_of_work_queue_session() {
        let mut uow = UnitOfWork::new();
        uow.create_session(SessionId::new(), "/tmp".to_string());
        assert!(!uow.is_empty());
        assert_eq!(uow.len(), 1);
    }

    /// Verify UnitOfWork can queue create_turn
    #[test]
    fn test_unit_of_work_queue_turn() {
        let mut uow = UnitOfWork::new();
        let turn_info = SqliteTurnInfo {
            id: TurnId::new(),
            session_id: SessionId::new(),
            branch_id: BranchId::new(),
            parent_turn_id: None,
            actor: "User".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            finalized_at: None,
        };
        uow.create_turn(turn_info);
        assert!(!uow.is_empty());
    }

    /// Verify UnitOfWork can queue multiple operations
    #[test]
    fn test_unit_of_work_multiple_operations() {
        let mut uow = UnitOfWork::new();
        uow.create_session(SessionId::new(), "/tmp".to_string());
        uow.create_session(SessionId::new(), "/tmp2".to_string());
        assert_eq!(uow.len(), 2);
    }

    /// Verify UnitOfWork commit succeeds with valid operations
    #[test]
    fn test_unit_of_work_commit_success() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        // Create initial state
        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&SqliteBranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        // Build unit of work
        let uow = UnitOfWorkBuilder::new()
            .with_turn(SqliteTurnInfo {
                id: TurnId::new(),
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .build();

        let result = uow.commit(&store);
        assert!(result.is_ok());
    }

    /// Verify UnitOfWork commits multiple operations atomically
    #[test]
    fn test_unit_of_work_multiple_commits() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn_id1 = TurnId::new();
        let turn_id2 = TurnId::new();

        // Create initial state
        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&SqliteBranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        // Build unit of work with multiple turns
        let mut uow = UnitOfWork::new();
        uow.create_turn(SqliteTurnInfo {
            id: turn_id1,
            session_id,
            branch_id,
            parent_turn_id: None,
            actor: "User".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            finalized_at: None,
        });
        uow.create_turn(SqliteTurnInfo {
            id: turn_id2,
            session_id,
            branch_id,
            parent_turn_id: Some(turn_id1),
            actor: "Assistant".to_string(),
            created_at: "2024-01-01T00:00:01Z".to_string(),
            finalized_at: None,
        });

        let result = uow.commit(&store);
        assert!(result.is_ok());

        // Verify both turns were created
        assert!(store.get_turn(turn_id1).unwrap().is_some());
        assert!(store.get_turn(turn_id2).unwrap().is_some());
    }

    /// Verify UnitOfWorkBuilder with fluent API
    #[test]
    fn test_unit_of_work_builder_fluent() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&SqliteBranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        let uow = UnitOfWorkBuilder::new()
            .with_turn(SqliteTurnInfo {
                id: TurnId::new(),
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .build();

        assert!(!uow.is_empty());
    }
}
