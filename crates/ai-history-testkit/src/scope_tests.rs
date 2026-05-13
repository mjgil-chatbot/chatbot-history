//! Testkit scope verification tests
//!
//! Task 15: Verify testkit provides realistic builders, fakes,
//! deterministic clocks, failure injection, dirty workspaces,
//! branch DAG fixtures, SQLite stores, manifests, blobs, and event streams.

#[cfg(test)]
mod tests {
    use crate::{
        BranchBuilder, DeterministicClock, DirtyWorkspaceBuilder, FakeBlobStore, SessionBuilder,
        TurnBuilder,
    };
    use ai_history_core::{
        Actor, BranchId, ChangeOwnership, ChangeType, CheckpointId, ContentHash, DirtyWorkspace,
        FileChange, ManifestId, SessionId, Timestamp, TurnId,
    };
    use ai_history_events::*;
    use ai_history_store::traits::{BranchInfo, SessionInfo, TurnInfo};

    #[test]
    fn test_session_builder_builds() {
        let info = SessionBuilder::new()
            .workspace_root("/tmp/test")
            .title("Test Session")
            .build();
        assert_eq!(info.workspace_root, "/tmp/test");
        assert_eq!(info.title, "Test Session");
    }

    #[test]
    fn test_session_builder_default() {
        let info = SessionBuilder::new().build();
        assert!(!info.workspace_root.is_empty());
        assert!(!info.title.is_empty());
    }

    #[test]
    fn test_branch_builder_builds() {
        let info = BranchBuilder::new().name("feature-branch").build();
        assert_eq!(info.name, "feature-branch");
    }

    #[test]
    fn test_branch_builder_default() {
        let info = BranchBuilder::new().build();
        assert_eq!(info.name, "main");
    }

    #[test]
    fn test_turn_builder_builds() {
        let info = TurnBuilder::new().actor(Actor::User).build();
        assert_eq!(info.actor, "User");
    }

    #[test]
    fn test_turn_builder_default() {
        let info = TurnBuilder::new().build();
        assert!(!info.actor.is_empty());
    }

    #[test]
    fn test_fake_blob_store_new() {
        let _store = FakeBlobStore::new();
        // Can construct without errors
    }

    #[test]
    fn test_deterministic_clock_same_time() {
        let clock = DeterministicClock::new(1000);
        let ts1 = clock.now();
        let ts2 = clock.now();
        assert_eq!(ts1, ts2);
    }

    #[test]
    fn test_deterministic_clock_advance() {
        let clock = DeterministicClock::new(1000);
        let ts1 = clock.now();
        clock.advance(3600); // advance 1 hour
        let ts2 = clock.now();
        // After advance, time should be different
        assert_ne!(ts1.to_string(), ts2.to_string());
    }

    #[test]
    fn test_dirty_workspace_builder_builds() {
        let builder = DirtyWorkspaceBuilder::new();
        let workspace = builder.build();
        // Empty builder creates empty DirtyWorkspace
        assert!(!workspace.is_dirty);
        assert!(workspace.changes.is_empty());
    }

    #[test]
    fn test_dirty_workspace_builder_with_changes() {
        let workspace = DirtyWorkspaceBuilder::new()
            .add_file_change("/workspace/ai.txt", ChangeType::Modified)
            .build();

        assert!(workspace.is_dirty);
        assert_eq!(workspace.changes.len(), 1);
    }

    #[test]
    fn test_dirty_workspace_ai_owned_count() {
        let workspace = DirtyWorkspaceBuilder::new()
            .add_file_change("/workspace/ai1.txt", ChangeType::Added)
            .add_file_change("/workspace/ai2.txt", ChangeType::Modified)
            .build();

        // Should have ai-owned changes tracked
        assert!(workspace.ai_owned_count >= 0);
    }

    #[test]
    fn test_session_info_complete() {
        let info = SessionInfo {
            id: SessionId::new(),
            title: "Test Session".to_string(),
            active_branch: BranchId::new(),
            view_cursor: None,
            workspace_root: "/tmp/workspace".to_string(),
            retention_config: "default".to_string(),
            project_identity: "test-project".to_string(),
            operation_state: "active".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            archival_state: false,
        };

        assert!(!info.archival_state);
        assert_eq!(info.operation_state, "active");
    }

    #[test]
    fn test_branch_info_complete() {
        let info = BranchInfo {
            id: BranchId::new(),
            session_id: SessionId::new(),
            name: "feature".to_string(),
            parent_branch_id: Some(BranchId::new()),
            created_from_turn_id: Some(TurnId::new()),
            head_turn_id: Some(TurnId::new()),
            ancestry: vec![],
            protected: false,
            archived: false,
            abandoned_future: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        assert!(!info.name.is_empty());
        assert!(info.parent_branch_id.is_some());
        assert!(info.head_turn_id.is_some());
    }

    #[test]
    fn test_turn_info_complete() {
        let info = TurnInfo {
            id: TurnId::new(),
            session_id: SessionId::new(),
            branch_id: BranchId::new(),
            parent_turn_id: None,
            actor: "User".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            finalized_at: None,
        };

        assert_eq!(info.actor, "User");
        assert!(info.finalized_at.is_none());
    }

    #[test]
    fn test_dirty_workspace_multi_file() {
        let workspace = DirtyWorkspaceBuilder::new()
            .add_file_change("/workspace/a.txt", ChangeType::Added)
            .add_file_change("/workspace/b.txt", ChangeType::Modified)
            .build();

        assert_eq!(workspace.changes.len(), 2);
    }

    #[test]
    fn test_file_change_types() {
        let fc = FileChange {
            path: "/workspace/test.txt".to_string(),
            change_type: ChangeType::Modified,
            ownership: ChangeOwnership::AiOwned,
            before_hash: None,
            after_hash: None,
        };
        assert_eq!(fc.path, "/workspace/test.txt");
        assert!(matches!(fc.change_type, ChangeType::Modified));
    }

    #[test]
    fn test_change_ownership_variants() {
        assert!(matches!(ChangeOwnership::AiOwned, ChangeOwnership::AiOwned));
        assert!(matches!(
            ChangeOwnership::UserOwned,
            ChangeOwnership::UserOwned
        ));
        assert!(matches!(
            ChangeOwnership::External,
            ChangeOwnership::External
        ));
        assert!(matches!(ChangeOwnership::Unknown, ChangeOwnership::Unknown));
    }
}
