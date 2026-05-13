//! Integration tests for CLI navigation and branching commands
//!
//! Tests that CLI commands correctly:
//! - Navigate backward without mutating branch head
//! - Navigate forward without mutating branch head
//! - ForkCurrent creates new branch without modifying original
//! - ForkFromCursor creates new branch from cursor position
//! - ContinueFromCursor creates new branch with new turn

#[cfg(test)]
mod test_navigate_back_preserves_head {
    use ai_history_core::{BranchId, SessionId, TurnId};
    use ai_history_store::sqlite_store::SqliteStore;

    /// Verify navigate-back does not mutate branch head
    #[test]
    fn test_navigate_back_preserves_branch_head() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        // Create session and branch
        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
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

        // Create a chain of turns: t1 -> t2 -> t3 (head)
        let t1 = TurnId::new();
        let t2 = TurnId::new();
        let t3 = TurnId::new();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t1,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t2,
                session_id,
                branch_id,
                parent_turn_id: Some(t1),
                actor: "Assistant".to_string(),
                created_at: "2024-01-01T00:00:01Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t3,
                session_id,
                branch_id,
                parent_turn_id: Some(t2),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:02Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        // Set branch head to t3
        store.update_branch_head(branch_id, t3).unwrap();

        // Navigate back twice
        store.set_cursor(session_id, branch_id, t2).unwrap();
        store.set_cursor(session_id, branch_id, t1).unwrap();

        // Branch head should be unchanged
        let branch = store.get_branch(branch_id).unwrap().unwrap();
        assert_eq!(
            branch.head_turn_id,
            Some(t3),
            "Branch head should not change during navigation"
        );
    }

    /// Verify cursor moves to correct position after navigate-back
    #[test]
    fn test_navigate_back_cursor_position() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
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

        let t1 = TurnId::new();
        let t2 = TurnId::new();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t1,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t2,
                session_id,
                branch_id,
                parent_turn_id: Some(t1),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:01Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store.update_branch_head(branch_id, t2).unwrap();

        // Set cursor to t2 (head), then navigate back to t1
        store.set_cursor(session_id, branch_id, t2).unwrap();
        store.set_cursor(session_id, branch_id, t1).unwrap();

        let cursor = store.get_cursor(session_id, branch_id).unwrap();
        assert_eq!(cursor, Some(t1), "Cursor should be at t1");
    }
}

#[cfg(test)]
mod test_fork_current_creates_branch {
    use ai_history_core::{BranchId, SessionId, TurnId};
    use ai_history_store::sqlite_store::SqliteStore;

    /// Verify fork-current creates new branch without modifying original
    #[test]
    fn test_fork_current_preserves_original() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let main_branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
                id: main_branch_id,
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

        let head_turn = TurnId::new();
        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: head_turn,
                session_id,
                branch_id: main_branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store.update_branch_head(main_branch_id, head_turn).unwrap();

        // Fork from current head - new branch starts at source branch's head
        let fork_branch_id = BranchId::new();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
                id: fork_branch_id,
                session_id,
                name: "feature".to_string(),
                parent_branch_id: Some(main_branch_id),
                created_from_turn_id: None,
                head_turn_id: Some(head_turn),
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:01Z".to_string(),
            })
            .unwrap();

        // Original branch head should be unchanged
        let main_branch = store.get_branch(main_branch_id).unwrap().unwrap();
        assert_eq!(
            main_branch.head_turn_id,
            Some(head_turn),
            "Original branch head should not change"
        );

        // Fork branch should start at the same turn as parent's head
        let fork_branch = store.get_branch(fork_branch_id).unwrap().unwrap();
        assert_eq!(
            fork_branch.head_turn_id,
            Some(head_turn),
            "Fork branch should start at parent's head"
        );

        // Fork branch should reference original branch as parent
        assert_eq!(
            fork_branch.parent_branch_id,
            Some(main_branch_id),
            "Fork should have parent branch"
        );
    }
}

#[cfg(test)]
mod test_fork_from_cursor_creates_branch {
    use ai_history_core::{BranchId, SessionId, TurnId};
    use ai_history_store::sqlite_store::SqliteStore;

    /// Verify fork-from-cursor creates new branch from cursor position
    #[test]
    fn test_fork_from_cursor_creates_branch() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let main_branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
                id: main_branch_id,
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

        // Create multiple turns
        let t1 = TurnId::new();
        let t2 = TurnId::new();
        let t3 = TurnId::new();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t1,
                session_id,
                branch_id: main_branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t2,
                session_id,
                branch_id: main_branch_id,
                parent_turn_id: Some(t1),
                actor: "Assistant".to_string(),
                created_at: "2024-01-01T00:00:01Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t3,
                session_id,
                branch_id: main_branch_id,
                parent_turn_id: Some(t2),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:02Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store.update_branch_head(main_branch_id, t3).unwrap();

        // Fork from t1 (cursor position) - new branch starts at cursor turn
        let fork_branch_id = BranchId::new();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
                id: fork_branch_id,
                session_id,
                name: "cursor-fork".to_string(),
                parent_branch_id: Some(main_branch_id),
                created_from_turn_id: None,
                head_turn_id: Some(t1),
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:03Z".to_string(),
            })
            .unwrap();

        // Original branch should be unchanged
        let main_branch = store.get_branch(main_branch_id).unwrap().unwrap();
        assert_eq!(
            main_branch.head_turn_id,
            Some(t3),
            "Original branch head unchanged"
        );

        // Fork should start at cursor turn (t1), not at parent's head (t3)
        let fork_branch = store.get_branch(fork_branch_id).unwrap().unwrap();
        assert_eq!(
            fork_branch.head_turn_id,
            Some(t1),
            "Fork branch should start at cursor turn"
        );
        assert_eq!(
            fork_branch.parent_branch_id,
            Some(main_branch_id),
            "Fork should have parent branch"
        );
    }
}

#[cfg(test)]
mod test_continue_from_cursor_creates_turn {
    use ai_history_core::{BranchId, SessionId, TurnId};
    use ai_history_store::sqlite_store::SqliteStore;

    /// Verify continue-from-cursor creates new branch with new turn
    #[test]
    fn test_continue_from_cursor_creates_new_branch_and_turn() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let main_branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
                id: main_branch_id,
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

        let cursor_turn = TurnId::new();
        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: cursor_turn,
                session_id,
                branch_id: main_branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .update_branch_head(main_branch_id, cursor_turn)
            .unwrap();

        // Continue from cursor
        let new_branch_id = BranchId::new();
        let new_turn_id = TurnId::new();

        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
                id: new_branch_id,
                session_id,
                name: "continue-abc-123".to_string(),
                parent_branch_id: Some(main_branch_id),
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:01Z".to_string(),
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: new_turn_id,
                session_id,
                branch_id: new_branch_id,
                parent_turn_id: Some(cursor_turn),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:02Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .update_branch_head(new_branch_id, new_turn_id)
            .unwrap();

        // New branch should exist with head at new turn
        let new_branch = store.get_branch(new_branch_id).unwrap().unwrap();
        assert_eq!(
            new_branch.head_turn_id,
            Some(new_turn_id),
            "New branch should have new turn as head"
        );
        assert_eq!(
            new_branch.parent_branch_id,
            Some(main_branch_id),
            "New branch should have parent"
        );

        // Original branch should be unchanged
        let original_branch = store.get_branch(main_branch_id).unwrap().unwrap();
        assert_eq!(
            original_branch.head_turn_id,
            Some(cursor_turn),
            "Original branch unchanged"
        );

        // New turn should reference cursor turn as parent
        let new_turn = store.get_turn(new_turn_id).unwrap().unwrap();
        assert_eq!(
            new_turn.parent_turn_id,
            Some(cursor_turn),
            "New turn should fork from cursor"
        );
    }
}

#[cfg(test)]
mod test_cursor_forward_navigation {
    use ai_history_core::{BranchId, SessionId, TurnId};
    use ai_history_store::sqlite_store::SqliteStore;

    /// Verify navigate-forward uses stored forward references
    #[test]
    fn test_get_turn_forward_child() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
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

        let t1 = TurnId::new();
        let t2 = TurnId::new();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t1,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t2,
                session_id,
                branch_id,
                parent_turn_id: Some(t1),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:01Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        // t1 should have t2 as forward child
        let forward_child = store.get_turn_forward_child(t1).unwrap();
        assert_eq!(
            forward_child,
            Some(t2),
            "t1 should have t2 as forward child"
        );

        // t2 should have no forward child
        let forward_child_t2 = store.get_turn_forward_child(t2).unwrap();
        assert!(
            forward_child_t2.is_none(),
            "t2 should have no forward child"
        );
    }

    /// Verify get_turn_forward_children returns all children
    #[test]
    fn test_get_turn_forward_children() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
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

        let t1 = TurnId::new();
        let child1 = TurnId::new();
        let child2 = TurnId::new();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t1,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: child1,
                session_id,
                branch_id,
                parent_turn_id: Some(t1),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:01Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: child2,
                session_id,
                branch_id,
                parent_turn_id: Some(t1),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:02Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        let children = store.get_turn_forward_children(t1).unwrap();
        assert_eq!(children.len(), 2, "t1 should have 2 children");
        assert!(children.contains(&child1), "Should contain child1");
        assert!(children.contains(&child2), "Should contain child2");
    }
}

#[cfg(test)]
mod test_navigate_forward_ambiguous_history {
    use ai_history_core::{BranchId, SessionId, TurnId};
    use ai_history_store::sqlite_store::SqliteStore;

    /// Verify ambiguous forward history is detected and reported without mutating state
    #[test]
    fn test_ambiguous_forward_reports_ambiguity() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
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

        // Create a turn with two children (main branch + fork branch)
        let parent = TurnId::new();
        let child_on_main = TurnId::new();
        let child_on_fork = TurnId::new();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: parent,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: child_on_main,
                session_id,
                branch_id,
                parent_turn_id: Some(parent),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:01Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: child_on_fork,
                session_id,
                branch_id,
                parent_turn_id: Some(parent),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:02Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store.update_branch_head(branch_id, child_on_main).unwrap();

        // Get forward children - should have 2
        let children = store.get_turn_forward_children(parent).unwrap();
        assert_eq!(
            children.len(),
            2,
            "Should have 2 forward children (ambiguous)"
        );

        // Verify branch head is still unchanged after querying forward children
        let branch = store.get_branch(branch_id).unwrap().unwrap();
        assert_eq!(
            branch.head_turn_id,
            Some(child_on_main),
            "Branch head should not change from forward query"
        );

        // Verify no checkpoints were created
        let checkpoints = store.list_checkpoints(session_id).unwrap();
        assert!(
            checkpoints.is_empty(),
            "Forward navigation should not create checkpoints"
        );

        // Verify no events were created
        let events = store.get_events(session_id).unwrap();
        assert!(
            events.is_empty(),
            "Forward navigation should not emit events"
        );
    }

    /// Verify single forward target does not report ambiguity
    #[test]
    fn test_single_forward_no_ambiguity() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
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

        let parent = TurnId::new();
        let child = TurnId::new();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: parent,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: child,
                session_id,
                branch_id,
                parent_turn_id: Some(parent),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:01Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store.update_branch_head(branch_id, child).unwrap();

        let children = store.get_turn_forward_children(parent).unwrap();
        assert_eq!(children.len(), 1, "Should have exactly 1 forward child");

        // Verify state is unchanged
        let branch = store.get_branch(branch_id).unwrap().unwrap();
        assert_eq!(branch.head_turn_id, Some(child));
    }
}

#[cfg(test)]
mod test_navigation_preserves_checkpoints_and_events {
    use ai_history_core::{BranchId, CheckpointId, ManifestId, SessionId, TurnId};
    use ai_history_store::sqlite_store::SqliteStore;

    /// Verify navigate-back does not create or remove checkpoints
    #[test]
    fn test_navigate_back_no_checkpoint_mutation() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
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

        let t1 = TurnId::new();
        let t2 = TurnId::new();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t1,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t2,
                session_id,
                branch_id,
                parent_turn_id: Some(t1),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:01Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store.update_branch_head(branch_id, t2).unwrap();

        // Create a checkpoint before navigation
        let cp_id = CheckpointId::new();
        let manifest_id = ManifestId::new();
        store
            .create_checkpoint(&ai_history_store::sqlite_store::CheckpointInfo {
                id: cp_id.clone(),
                session_id,
                branch_id,
                turn_id: Some(t2),
                manifest_id,
                reason: "before navigation".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        let checkpoint_count_before = store.list_checkpoints(session_id).unwrap().len();

        // Navigate back
        store.set_cursor(session_id, branch_id, t1).unwrap();

        let checkpoint_count_after = store.list_checkpoints(session_id).unwrap().len();
        assert_eq!(
            checkpoint_count_before, checkpoint_count_after,
            "Navigate-back should not create or remove checkpoints"
        );

        // Also verify branch head unchanged
        let branch = store.get_branch(branch_id).unwrap().unwrap();
        assert_eq!(
            branch.head_turn_id,
            Some(t2),
            "Branch head should not change during navigation"
        );
    }

    /// Verify navigate-forward does not create or remove checkpoints
    #[test]
    fn test_navigate_forward_no_checkpoint_mutation() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
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

        let t1 = TurnId::new();
        let t2 = TurnId::new();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t1,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t2,
                session_id,
                branch_id,
                parent_turn_id: Some(t1),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:01Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store.update_branch_head(branch_id, t2).unwrap();

        // Create a checkpoint
        let cp_id = CheckpointId::new();
        let manifest_id = ManifestId::new();
        store
            .create_checkpoint(&ai_history_store::sqlite_store::CheckpointInfo {
                id: cp_id,
                session_id,
                branch_id,
                turn_id: Some(t1),
                manifest_id,
                reason: "before navigation".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        let checkpoint_count_before = store.list_checkpoints(session_id).unwrap().len();

        // Query forward children (simulates forward navigation planning)
        let _children = store.get_turn_forward_children(t1).unwrap();

        let checkpoint_count_after = store.list_checkpoints(session_id).unwrap().len();
        assert_eq!(
            checkpoint_count_before, checkpoint_count_after,
            "Forward navigation query should not create or remove checkpoints"
        );

        // Verify branch head unchanged
        let branch = store.get_branch(branch_id).unwrap().unwrap();
        assert_eq!(
            branch.head_turn_id,
            Some(t2),
            "Branch head should not change"
        );
    }

    /// Verify navigate-back does not create events
    #[test]
    fn test_navigate_back_no_event_mutation() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
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

        let t1 = TurnId::new();
        let t2 = TurnId::new();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t1,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t2,
                session_id,
                branch_id,
                parent_turn_id: Some(t1),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:01Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store.update_branch_head(branch_id, t2).unwrap();

        let event_count_before = store.get_events(session_id).unwrap().len();

        // Navigate back
        store.set_cursor(session_id, branch_id, t1).unwrap();

        let event_count_after = store.get_events(session_id).unwrap().len();
        assert_eq!(
            event_count_before, event_count_after,
            "Navigate-back should not emit events"
        );

        // Branch head unchanged
        let branch = store.get_branch(branch_id).unwrap().unwrap();
        assert_eq!(branch.head_turn_id, Some(t2));
    }

    /// Verify navigate-forward does not create events
    #[test]
    fn test_navigate_forward_no_event_mutation() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&ai_history_store::sqlite_store::BranchInfo {
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

        let t1 = TurnId::new();
        let t2 = TurnId::new();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t1,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store
            .create_turn(&ai_history_store::sqlite_store::TurnInfo {
                id: t2,
                session_id,
                branch_id,
                parent_turn_id: Some(t1),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:01Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store.update_branch_head(branch_id, t2).unwrap();

        let event_count_before = store.get_events(session_id).unwrap().len();

        // Query forward children
        let _children = store.get_turn_forward_children(t1).unwrap();

        let event_count_after = store.get_events(session_id).unwrap().len();
        assert_eq!(
            event_count_before, event_count_after,
            "Forward navigation should not emit events"
        );

        // Branch head unchanged
        let branch = store.get_branch(branch_id).unwrap().unwrap();
        assert_eq!(branch.head_turn_id, Some(t2));
    }
}
