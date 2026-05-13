//! Store crate design tests
//!
//! Task 10: Complete `ai-history-store` crate design
//!
//! Verify:
//! - SqliteStore exists and can be instantiated
//! - Unit-of-work exists
//! - Migration constants exist
//! - Views exist and are serializable
//! - Info structs exist

#[cfg(test)]
mod test_store_views {
    use ai_history_core::{BranchId, SessionId};
    use ai_history_store::views::{BranchTimelineView, ChatTranscriptView, CheckpointListView};

    /// Verify ChatTranscriptView exists and is serializable
    #[test]
    fn test_chat_transcript_view_serialization() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let view = ChatTranscriptView::new(session_id, branch_id);

        // Should be serializable to JSON
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("session_id"));
    }

    /// Verify BranchTimelineView exists
    #[test]
    fn test_branch_timeline_view_serialization() {
        let session_id = SessionId::new();
        let view = BranchTimelineView::new(session_id);

        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("session_id"));
    }

    /// Verify CheckpointListView exists
    #[test]
    fn test_checkpoint_list_view_serialization() {
        let session_id = SessionId::new();
        let view = CheckpointListView::new(session_id);

        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("session_id"));
    }
}
#[cfg(test)]
mod test_store_migrations {
    use ai_history_store::migrations::{INITIAL_MIGRATION_SQL, SCHEMA_VERSION};

    /// Verify migration constants exist
    #[test]
    fn test_migration_infrastructure() {
        assert_eq!(SCHEMA_VERSION, 1);
        assert!(!INITIAL_MIGRATION_SQL.is_empty());
        assert!(INITIAL_MIGRATION_SQL.contains("CREATE TABLE"));
    }
}
#[cfg(test)]
mod test_store_sqlite {
    use ai_history_store::SqliteStore;

    /// Verify SqliteStore can be created in-memory
    #[test]
    fn test_sqlite_store_in_memory() {
        let store = SqliteStore::in_memory();
        assert!(store.is_ok());
    }
}
#[cfg(test)]
mod test_store_info_structs {
    use ai_history_store::{BranchInfo, CheckpointInfo, SessionInfo, TurnInfo};

    /// Verify SessionInfo exists and has required fields
    #[test]
    fn test_session_info() {
        let info = SessionInfo {
            id: ai_history_core::SessionId::new(),
            title: "Test".to_string(),
            active_branch: ai_history_core::BranchId::new(),
            view_cursor: None,
            workspace_root: "/tmp".to_string(),
            retention_config: "default".to_string(),
            project_identity: "test".to_string(),
            operation_state: "active".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            archival_state: false,
        };
        assert!(!info.workspace_root.is_empty());
    }

    /// Verify BranchInfo exists and has required fields
    #[test]
    fn test_branch_info() {
        let info = BranchInfo {
            id: ai_history_core::BranchId::new(),
            session_id: ai_history_core::SessionId::new(),
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
        assert!(!info.name.is_empty());
    }

    /// Verify TurnInfo exists and has required fields
    #[test]
    fn test_turn_info() {
        let info = TurnInfo {
            id: ai_history_core::TurnId::new(),
            session_id: ai_history_core::SessionId::new(),
            branch_id: ai_history_core::BranchId::new(),
            parent_turn_id: None,
            actor: "user".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            finalized_at: None,
        };
        assert!(!info.actor.is_empty());
    }

    /// Verify CheckpointInfo exists and has required fields
    #[test]
    fn test_checkpoint_info() {
        let info = CheckpointInfo {
            id: ai_history_core::CheckpointId::new(),
            session_id: ai_history_core::SessionId::new(),
            branch_id: ai_history_core::BranchId::new(),
            turn_id: None,
            manifest_id: ai_history_core::ManifestId::new(),
            reason: "test".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(!info.reason.is_empty());
    }
}
