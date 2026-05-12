//! Branch-fork and navigation policy end-to-end tests
//!
//! Tests that verify:
//! - fork-current, fork-from-cursor, and continue-from-cursor preserve abandoned futures
//! - Ancestry is correctly maintained for all branch-fork operations
//! - Navigation commands do not touch workspace files (no file reads/writes)
//! - Dirty workspace policy is explicit for all branching commands
//!
//! These tests use the actual SQLite store for cursor/checkpoint operations
//! and the core BranchGraph for fork operations, verifying the integration.

use ai_history_core::{
    continue_from_cursor, fork_current_conversation, fork_from_cursor, Actor, Branch, BranchGraph,
    BranchId, ForkError, ForkFromCursorOptions, SessionId, TurnId,
};
use ai_history_snapshot::manifest::{FileEntry, Manifest};
use ai_history_store::sqlite_store::{
    BranchInfo, CheckpointInfo, ManifestInfo, SqliteStore, TurnInfo,
};
use tempfile::TempDir;

// =============================================================================
// fork-current end-to-end tests with BranchGraph
// =============================================================================

#[cfg(test)]
mod test_fork_current_e2e {
    use super::*;

    /// Verify fork-current: preserves abandoned future and correct ancestry
    #[test]
    fn test_fork_current_preserves_abandoned_future_and_ancestry() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head_turn = TurnId::new();
        main.set_head(head_turn.clone());
        let main_id = graph.add_branch(main).unwrap();

        // Fork current
        let result = fork_current_conversation(&mut graph, session_id, main_id, "feature").unwrap();

        // Verify: new branch preserves original head as abandoned future
        let fork_branch = graph.get_branch(result.new_branch_id).unwrap();
        assert!(
            fork_branch.abandoned_futures.contains(&head_turn),
            "fork-current must preserve original head as abandoned_future"
        );

        // Verify: correct ancestry (parent branch relationship)
        assert_eq!(
            fork_branch.parent_branch_id,
            Some(main_id),
            "fork-current must have original branch as parent"
        );

        // Verify: original branch head unchanged
        let main_branch = graph.get_branch(main_id).unwrap();
        assert_eq!(
            main_branch.head_turn_id,
            Some(head_turn),
            "fork-current must not modify original branch head"
        );

        // Verify: fork branch has no head yet (workspace not restored)
        assert!(
            fork_branch.head_turn_id.is_none(),
            "fork-current creates fork with no head (dirty workspace policy)"
        );
    }

    /// Policy test: fork-current does NOT restore workspace
    #[test]
    fn test_fork_current_no_workspace_restore_policy() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head_turn = TurnId::new();
        main.set_head(head_turn.clone());
        let main_id = graph.add_branch(main).unwrap();

        // Fork current
        let result = fork_current_conversation(&mut graph, session_id, main_id, "feature").unwrap();

        // Policy: fork creates branch with no head (no workspace restore)
        let fork_branch = graph.get_branch(result.new_branch_id).unwrap();
        assert!(
            fork_branch.head_turn_id.is_none(),
            "fork-current does not restore workspace (dirty workspace policy)"
        );

        // The original head becomes abandoned future, not workspace state
        assert!(
            result.abandoned_head_turn_id.is_some(),
            "original head preserved as abandoned_future metadata"
        );
    }
}

// =============================================================================
// fork-from-cursor end-to-end tests with BranchGraph
// =============================================================================

#[cfg(test)]
mod test_fork_from_cursor_e2e {
    use super::*;

    /// Verify fork-from-cursor: preserves cursor turn as abandoned future
    #[test]
    fn test_fork_from_cursor_preserves_abandoned_future_and_ancestry() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        // Create chain: t1 -> t2 -> t3 (head)
        let t1 = TurnId::new();
        let t2 = TurnId::new();
        let t3 = TurnId::new();
        main.set_head(t3.clone());
        let main_id = graph.add_branch(main).unwrap();

        // Fork from t1 (cursor position behind head)
        let result = fork_from_cursor(
            &mut graph,
            session_id,
            main_id,
            t1,
            "cursor-fork",
            ForkFromCursorOptions {
                restore_workspace: false,
            },
        )
        .unwrap();

        // Verify: abandoned future is the cursor turn
        let fork_branch = graph.get_branch(result.new_branch_id).unwrap();
        assert!(
            fork_branch.abandoned_futures.contains(&t1),
            "fork-from-cursor must preserve cursor turn as abandoned_future"
        );

        // Verify: correct ancestry
        assert_eq!(
            fork_branch.parent_branch_id,
            Some(main_id),
            "fork-from-cursor must have original branch as parent"
        );

        // Verify: original branch unchanged
        let main_branch = graph.get_branch(main_id).unwrap();
        assert_eq!(
            main_branch.head_turn_id,
            Some(t3),
            "Original branch head unchanged"
        );
    }

    /// Policy test: fork-from-cursor default does NOT restore workspace
    #[test]
    fn test_fork_from_cursor_default_no_restore() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head_turn = TurnId::new();
        main.set_head(head_turn.clone());
        let main_id = graph.add_branch(main).unwrap();

        // Fork from cursor without restore
        let result = fork_from_cursor(
            &mut graph,
            session_id,
            main_id,
            head_turn,
            "no-restore-fork",
            ForkFromCursorOptions {
                restore_workspace: false,
            },
        )
        .unwrap();

        assert!(
            !result.workspace_restored,
            "fork-from-cursor with restore_workspace=false must not restore workspace"
        );
        let fork_branch = graph.get_branch(result.new_branch_id).unwrap();
        assert!(
            fork_branch.head_turn_id.is_none(),
            "fork-from-cursor creates fork with no head when workspace not restored"
        );
    }
}

// =============================================================================
// continue-from-cursor end-to-end tests with BranchGraph
// =============================================================================

#[cfg(test)]
mod test_continue_from_cursor_e2e {
    use super::*;

    /// Verify continue-from-cursor: fork succeeds but turn creation returns error
    /// (no-turn-synthesis policy). The original head is preserved as abandoned
    /// future on the new fork branch.
    #[test]
    fn test_continue_from_cursor_fork_succeeds_turn_creation_fails() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let cursor_turn = TurnId::new();
        main.set_head(cursor_turn.clone());
        let main_id = graph.add_branch(main).unwrap();

        // Continue from cursor - fork succeeds, turn creation fails
        let result =
            continue_from_cursor(&mut graph, session_id, main_id, cursor_turn, Actor::User);

        // Must return error, not fabricated turn ID
        assert!(
            result.is_err(),
            "continue_from_cursor must not fabricate turn ID"
        );
        match result.unwrap_err() {
            ForkError::TurnNotFound(fork_point) => {
                assert_eq!(fork_point, cursor_turn, "error reports cursor turn");
            }
            e => panic!("Expected ForkError::TurnNotFound, got {:?}", e),
        }

        // Verify: original branch unchanged
        let main_branch = graph.get_branch(main_id).unwrap();
        assert_eq!(main_branch.head_turn_id, Some(cursor_turn.clone()));

        // Verify: a new branch was created (fork) as descendant of main
        let descendants = graph.get_descendants(main_id);
        assert!(
            descendants.len() >= 1,
            "continue_from_cursor creates fork branch (descendant exists)"
        );

        // Verify: the fork branch has the cursor turn as abandoned future
        // (we verify this by checking the first descendant has abandoned futures)
        let fork_branch_id = descendants.first().unwrap();
        let fork_branch = graph.get_branch(*fork_branch_id).unwrap();
        assert!(
            fork_branch.abandoned_futures.contains(&cursor_turn),
            "fork branch must preserve cursor turn as abandoned_future"
        );
        assert_eq!(
            fork_branch.parent_branch_id,
            Some(main_id),
            "fork branch must have original branch as parent"
        );
    }

    /// Policy test: continue-from-cursor does NOT restore workspace.
    /// The fork is created but workspace is left dirty (no auto-restore).
    #[test]
    fn test_continue_from_cursor_no_workspace_restore_policy() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let cursor_turn = TurnId::new();
        main.set_head(cursor_turn.clone());
        let main_id = graph.add_branch(main).unwrap();

        // Continue from cursor - fork succeeds, turn creation fails
        let result =
            continue_from_cursor(&mut graph, session_id, main_id, cursor_turn, Actor::User);

        // Policy: returns error instead of fabricated turn ID
        assert!(
            result.is_err(),
            "continue_from_cursor must not fabricate turn ID"
        );

        // Policy: workspace is NOT automatically restored (dirty workspace preserved)
        // Original branch head is unchanged
        let main_branch = graph.get_branch(main_id).unwrap();
        assert_eq!(
            main_branch.head_turn_id,
            Some(cursor_turn.clone()),
            "original branch head unchanged (workspace preserved as-is)"
        );

        // Verify: fork was created
        let descendants = graph.get_descendants(main_id);
        assert!(
            descendants.len() >= 1,
            "continue_from_cursor creates fork branch"
        );
    }
}

// =============================================================================
// Navigation workspace isolation tests using SQLite store
// =============================================================================

#[cfg(test)]
mod test_navigation_workspace_isolation {
    use super::*;

    /// Verify navigate-back does not read or write workspace files
    #[test]
    fn test_navigate_back_workspace_isolation() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path();
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store
            .create_session(session_id, workspace_root.to_str().unwrap())
            .unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                head_turn_id: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                created_from_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
            })
            .unwrap();

        // Create turn chain
        let t1 = TurnId::new();
        let t2 = TurnId::new();
        store
            .create_turn(&TurnInfo {
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
            .create_turn(&TurnInfo {
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

        // Create workspace file
        let test_file = workspace_root.join("test.txt");
        std::fs::write(&test_file, b"sensitive content").unwrap();
        let original_mtime = std::fs::metadata(&test_file).unwrap().modified().unwrap();

        // Navigate back
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.set_cursor(session_id, branch_id, t1).unwrap();

        // Verify: file unchanged (not touched by navigation)
        let new_mtime = std::fs::metadata(&test_file).unwrap().modified().unwrap();
        assert_eq!(
            original_mtime, new_mtime,
            "navigate-back must not modify workspace files"
        );

        // Verify content unchanged
        assert_eq!(std::fs::read(&test_file).unwrap(), b"sensitive content");
    }

    /// Verify forward navigation queries do not touch workspace
    #[test]
    fn test_forward_query_workspace_isolation() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path();
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store
            .create_session(session_id, workspace_root.to_str().unwrap())
            .unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                head_turn_id: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                created_from_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
            })
            .unwrap();

        // Create turns with forward children
        let parent = TurnId::new();
        let child = TurnId::new();
        store
            .create_turn(&TurnInfo {
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
            .create_turn(&TurnInfo {
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

        // Create and modify workspace file
        let test_file = workspace_root.join("code.rs");
        std::fs::write(&test_file, b"// original").unwrap();
        let original_mtime = std::fs::metadata(&test_file).unwrap().modified().unwrap();

        // Query forward children (simulates forward navigation planning)
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _children = store.get_turn_forward_children(parent).unwrap();

        // Verify: workspace unchanged
        let new_mtime = std::fs::metadata(&test_file).unwrap().modified().unwrap();
        assert_eq!(
            original_mtime, new_mtime,
            "forward query must not modify workspace"
        );
        assert_eq!(std::fs::read(&test_file).unwrap(), b"// original");
    }

    /// Verify navigate-forward does not touch workspace when reporting ambiguity
    #[test]
    fn test_ambiguous_forward_workspace_isolation() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path();
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store
            .create_session(session_id, workspace_root.to_str().unwrap())
            .unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                head_turn_id: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                created_from_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
            })
            .unwrap();

        // Create turn with multiple forward children (ambiguous case)
        let parent = TurnId::new();
        let child1 = TurnId::new();
        let child2 = TurnId::new();

        store
            .create_turn(&TurnInfo {
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
            .create_turn(&TurnInfo {
                id: child1,
                session_id,
                branch_id,
                parent_turn_id: Some(parent),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:01Z".to_string(),
                finalized_at: None,
            })
            .unwrap();
        store
            .create_turn(&TurnInfo {
                id: child2,
                session_id,
                branch_id,
                parent_turn_id: Some(parent),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:02Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        // Create workspace file
        let test_file = workspace_root.join("important.txt");
        std::fs::write(&test_file, b"important data").unwrap();
        let original_mtime = std::fs::metadata(&test_file).unwrap().modified().unwrap();

        // Query forward children - should detect ambiguity
        std::thread::sleep(std::time::Duration::from_millis(10));
        let children = store.get_turn_forward_children(parent).unwrap();
        assert_eq!(children.len(), 2, "Should detect ambiguous forward path");

        // Verify: workspace unchanged after ambiguity detection
        let new_mtime = std::fs::metadata(&test_file).unwrap().modified().unwrap();
        assert_eq!(
            original_mtime, new_mtime,
            "ambiguity detection must not modify workspace"
        );
        assert_eq!(std::fs::read(&test_file).unwrap(), b"important data");
    }
}

// =============================================================================
// Ambiguous forward history reporting tests
// =============================================================================

#[cfg(test)]
mod test_ambiguous_forward_history_reporting {
    use super::*;

    /// Verify ambiguous forward history is detected and reported with branch context
    #[test]
    fn test_ambiguous_forward_reports_cross_branch_targets() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let main_branch_id = BranchId::new();
        let fork_branch_id = BranchId::new();

        store.create_session(session_id, "/tmp/test").unwrap();
        store
            .create_branch(&BranchInfo {
                id: main_branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                head_turn_id: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                created_from_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
            })
            .unwrap();
        store
            .create_branch(&BranchInfo {
                id: fork_branch_id,
                session_id,
                name: "feature".to_string(),
                parent_branch_id: Some(main_branch_id),
                head_turn_id: None,
                created_at: "2024-01-01T00:00:02Z".to_string(),
                created_from_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
            })
            .unwrap();

        // Create parent turn
        let parent = TurnId::new();
        store
            .create_turn(&TurnInfo {
                id: parent,
                session_id,
                branch_id: main_branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        // Create child on main branch
        let child_on_main = TurnId::new();
        store
            .create_turn(&TurnInfo {
                id: child_on_main,
                session_id,
                branch_id: main_branch_id,
                parent_turn_id: Some(parent),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:01Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        // Create child on fork branch
        let child_on_fork = TurnId::new();
        store
            .create_turn(&TurnInfo {
                id: child_on_fork,
                session_id,
                branch_id: fork_branch_id,
                parent_turn_id: Some(parent),
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:02Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        // Explicitly set branch heads to establish the persisted precondition.
        // create_turn() does NOT advance branch heads; this is why the original
        // assertion was broken (it assumed implicit head mutation).
        store
            .update_branch_head(main_branch_id, child_on_main)
            .unwrap();
        store
            .update_branch_head(fork_branch_id, child_on_fork)
            .unwrap();

        // Get forward children - should return both (ambiguous)
        let children = store.get_turn_forward_children(parent).unwrap();
        assert_eq!(
            children.len(),
            2,
            "Should have 2 forward children (ambiguous)"
        );

        // Verify both branches are represented
        let turn_on_main = store.get_turn(children[0]).unwrap().unwrap();
        let turn_on_fork = store.get_turn(children[1]).unwrap().unwrap();
        assert!(
            (turn_on_main.branch_id == main_branch_id && turn_on_fork.branch_id == fork_branch_id)
                || (turn_on_main.branch_id == fork_branch_id
                    && turn_on_fork.branch_id == main_branch_id),
            "Forward children should span multiple branches"
        );

        // Verify: no state mutation occurred
        let main = store.get_branch(main_branch_id).unwrap().unwrap();
        assert_eq!(
            main.head_turn_id,
            Some(child_on_main),
            "Branch head unchanged"
        );
    }

    /// Verify single forward path does not trigger ambiguity report
    #[test]
    fn test_single_forward_no_ambiguity_reporting() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp/test").unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                head_turn_id: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                created_from_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
            })
            .unwrap();

        let parent = TurnId::new();
        let child = TurnId::new();

        store
            .create_turn(&TurnInfo {
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
            .create_turn(&TurnInfo {
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
        assert_eq!(children.len(), 1, "Single forward path has no ambiguity");
    }
}
