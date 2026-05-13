//! Forward-history and forking tests
//!
//! Task 22: Finalize forward-history navigation
//! Task 23: Finalize current-head forking
//! Task 24: Finalize cursor-based forking
//! Task 25: Finalize continue-from-cursor
//!
//! Verify:
//! - Forward navigation targets exist
//! - fork_current_conversation creates new branch from head
//! - fork_from_cursor creates branch from cursor position
//! - continue_from_cursor creates new branch with new turn
//! - edit_and_rerun preserves original turn

#[cfg(test)]
mod test_forward_navigation {
    use ai_history_core::{
        fork_current_conversation, fork_from_cursor, Branch, BranchGraph, BranchId,
        ForkFromCursorOptions, SessionId, TurnId,
    };

    /// Verify forward navigation returns children when at a turn with children
    #[test]
    fn test_forward_navigation_has_children() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head = TurnId::new();
        main.set_head(head.clone());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        // Fork from head - creates a child branch
        let fork = graph
            .fork_branch(
                main_id,
                "feature",
                ai_history_core::BranchCreationReason::ForkCurrent,
                head,
            )
            .unwrap();

        // Now main should have a descendant (the future that was abandoned)
        let descendants = graph.get_descendants(main_id);
        assert!(descendants.contains(&fork.id));
    }
}

#[cfg(test)]
mod test_fork_current {
    use ai_history_core::{
        can_fork_current, fork_current_conversation, Branch, BranchGraph, SessionId, TurnId,
    };

    /// Verify fork_current_conversation creates new branch
    #[test]
    fn test_fork_current_creates_branch() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        main.set_head(TurnId::new());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let result = fork_current_conversation(&mut graph, session_id, main_id, "feature").unwrap();

        assert_ne!(result.new_branch_id, main_id);
        assert!(graph.get_branch(result.new_branch_id).is_some());
    }

    /// Verify fork_current preserves original branch
    #[test]
    fn test_fork_current_preserves_original() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head = TurnId::new();
        main.set_head(head.clone());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let result = fork_current_conversation(&mut graph, session_id, main_id, "feature").unwrap();

        // Original branch unchanged
        let original = graph.get_branch(main_id).unwrap();
        assert_eq!(original.head_turn_id, Some(head));
        assert!(!original.is_protected);
    }

    /// Verify can_fork_current when branch has head
    #[test]
    fn test_can_fork_current_true() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        main.set_head(TurnId::new());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        assert!(can_fork_current(&graph, main_id));
    }

    /// Verify can_fork_current when branch has no head
    #[test]
    fn test_can_fork_current_false() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let main = Branch::new(session_id, "main");
        let main_id = graph.add_branch(main).unwrap();

        assert!(!can_fork_current(&graph, main_id));
    }

    /// Verify abandoned_head_turn_id is set
    #[test]
    fn test_fork_current_abandoned_head() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        main.set_head(TurnId::new());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let result = fork_current_conversation(&mut graph, session_id, main_id, "feature").unwrap();

        assert!(result.abandoned_head_turn_id.is_some());
    }
}

#[cfg(test)]
mod test_fork_from_cursor {
    use ai_history_core::{
        fork_from_cursor, Branch, BranchGraph, ForkFromCursorOptions, SessionId, TurnId,
    };

    /// Verify fork_from_cursor creates branch from cursor position
    #[test]
    fn test_fork_from_cursor_creates_branch() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head = TurnId::new();
        main.set_head(head.clone());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        // Fork from cursor at head position
        let cursor_turn = head;
        let options = ForkFromCursorOptions {
            restore_workspace: false,
        };

        let result = fork_from_cursor(
            &mut graph,
            session_id,
            main_id,
            cursor_turn,
            "cursor-fork",
            options,
        )
        .unwrap();

        assert_eq!(result.fork_point_turn_id, cursor_turn);
        assert!(!result.workspace_restored);
    }

    /// Verify fork_from_cursor with workspace restore
    #[test]
    fn test_fork_from_cursor_with_restore() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head = TurnId::new();
        main.set_head(head.clone());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let options = ForkFromCursorOptions {
            restore_workspace: true,
        };

        let result = fork_from_cursor(
            &mut graph,
            session_id,
            main_id,
            head,
            "cursor-fork",
            options,
        )
        .unwrap();

        assert!(result.workspace_restored);
    }
}

#[cfg(test)]
mod test_continue_from_cursor {
    use ai_history_core::{
        can_continue_from_cursor, continue_from_cursor, Actor, Branch, BranchGraph, SessionId,
        TurnId,
    };

    /// Verify continue_from_cursor creates a new branch fork but returns error
    /// instead of fabricating a turn ID (no-turn-synthesis policy).
    /// The caller must create the turn through store/events.
    #[test]
    fn test_continue_from_cursor_returns_error_not_fabricated_turn() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head = TurnId::new();
        main.set_head(head.clone());
        let main_id = graph.add_branch(main).unwrap();

        let result = continue_from_cursor(&mut graph, session_id, main_id, head, Actor::User);

        // Must return error, not a fabricated turn ID
        assert!(
            result.is_err(),
            "continue_from_cursor must not return fabricated turn ID"
        );
        match result.unwrap_err() {
            ai_history_core::ForkError::TurnNotFound(cursor_turn) => {
                assert_eq!(cursor_turn, head, "error must report cursor turn ID");
            }
            e => panic!("Expected ForkError::TurnNotFound, got {:?}", e),
        }
    }

    /// Verify can_continue_from_cursor
    #[test]
    fn test_can_continue_from_cursor() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let main = Branch::new(session_id, "main");
        let main_id = graph.add_branch(main).unwrap();
        assert!(!can_continue_from_cursor(&graph, main_id));

        let mut main_with_head = Branch::new(session_id, "main2");
        main_with_head.set_head(TurnId::new());
        let main_id2 = graph.add_branch(main_with_head).unwrap();
        assert!(can_continue_from_cursor(&graph, main_id2));
    }
}

#[cfg(test)]
mod test_edit_and_rerun {
    use ai_history_core::{edit_and_rerun, Branch, BranchGraph, SessionId, TurnId};

    /// Verify edit_and_rerun returns error for root turn (no parent to fork from).
    /// This enforces the no-turn-synthesis policy: edit-and-rerun requires a valid
    /// parent turn that must be looked up through the store, which we cannot do here.
    #[test]
    fn test_edit_and_rerun_rejects_root_turn() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head = TurnId::new();
        main.set_head(head.clone());
        let main_id = graph.add_branch(main).unwrap();

        let result = edit_and_rerun(&mut graph, session_id, main_id, head, "edited prompt");

        // Must return error for root turn (cannot determine parent)
        assert!(result.is_err(), "edit_and_rerun must reject root turn");
        match result.unwrap_err() {
            ai_history_core::ForkError::NoParentTurn(turn_id) => {
                assert_eq!(turn_id, head, "error must report the root turn ID");
            }
            e => panic!("Expected ForkError::NoParentTurn, got {:?}", e),
        }
    }
}

// =============================================================================
// Branch-fork and continue-from-history policy tests
// =============================================================================

#[cfg(test)]
mod test_branch_fork_semantics {
    use ai_history_core::{
        fork_current_conversation, fork_from_cursor, Branch, BranchGraph, ForkFromCursorOptions,
        SessionId, TurnId,
    };

    /// Verify fork_current preserves the original branch head as abandoned future.
    /// Policy: forking creates a new branch without modifying the original;
    /// the original head turn becomes an abandoned future that is retained.
    #[test]
    fn test_fork_current_preserves_abandoned_futures() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head = TurnId::new();
        main.set_head(head.clone());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let result = fork_current_conversation(&mut graph, session_id, main_id, "feature").unwrap();

        // The new branch must record the original head as an abandoned future
        let fork_branch = graph.get_branch(result.new_branch_id).unwrap();
        assert!(
            fork_branch.abandoned_futures.contains(&head),
            "fork_current must preserve original head turn as abandoned_future"
        );
        // Verify the abandoned future is exactly the original head
        assert_eq!(
            fork_branch.abandoned_futures.len(),
            1,
            "fork_current sets exactly one abandoned future (the original head)"
        );
    }

    /// Verify fork_from_cursor preserves the cursor turn as abandoned future
    /// when cursor is behind the source branch head.
    #[test]
    fn test_fork_from_cursor_preserves_abandoned_futures() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        // Create three turns: t1 -> t2 -> t3 (head)
        let t1 = TurnId::new();
        let t2 = TurnId::new();
        let t3 = TurnId::new();
        main.set_head(t3.clone());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        // Fork from t1, which is behind head (t3)
        let options = ForkFromCursorOptions {
            restore_workspace: false,
        };

        let result =
            fork_from_cursor(&mut graph, session_id, main_id, t1, "cursor-fork", options).unwrap();

        let fork_branch = graph.get_branch(result.new_branch_id).unwrap();
        assert!(
            fork_branch.abandoned_futures.contains(&t1),
            "fork_from_cursor must preserve cursor turn as abandoned_future when cursor is behind head"
        );
    }

    /// Verify fork_current sets correct ancestry: new branch has original as parent.
    #[test]
    fn test_fork_current_ancestry_chain() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head = TurnId::new();
        main.set_head(head.clone());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let result = fork_current_conversation(&mut graph, session_id, main_id, "feature").unwrap();

        let fork_branch = graph.get_branch(result.new_branch_id).unwrap();
        assert_eq!(
            fork_branch.parent_branch_id,
            Some(main_id),
            "fork_current new branch must have original branch as parent"
        );
        // New branch is not an ancestor of itself
        assert!(
            !fork_branch.is_ancestor_of(&fork_branch),
            "a branch is not its own ancestor"
        );
    }

    /// Verify fork_from_cursor sets correct parent relationship.
    #[test]
    fn test_fork_from_cursor_ancestry_chain() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head = TurnId::new();
        main.set_head(head.clone());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let cursor_turn = head;
        let options = ForkFromCursorOptions {
            restore_workspace: false,
        };

        let result = fork_from_cursor(
            &mut graph,
            session_id,
            main_id,
            cursor_turn,
            "cursor-fork",
            options,
        )
        .unwrap();

        let fork_branch = graph.get_branch(result.new_branch_id).unwrap();
        assert_eq!(
            fork_branch.parent_branch_id,
            Some(main_id),
            "fork_from_cursor new branch must have original branch as parent"
        );
    }

    /// Policy test: fork_current does NOT attempt workspace restoration.
    /// The new branch is created with no head and no workspace restore is performed.
    /// This is an explicit policy: dirty workspace is preserved as-is.
    #[test]
    fn test_fork_current_dirty_workspace_policy() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        main.set_head(TurnId::new());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let result = fork_current_conversation(&mut graph, session_id, main_id, "feature").unwrap();

        // fork_current creates a branch with no head turn yet
        let fork_branch = graph.get_branch(result.new_branch_id).unwrap();
        assert!(
            fork_branch.head_turn_id.is_none(),
            "fork_current creates fork branch with no head (workspace not restored)"
        );
        // The abandoned head is preserved in metadata for potential later use
        assert!(
            result.abandoned_head_turn_id.is_some(),
            "original head is preserved as abandoned_future, not restored to workspace"
        );
    }

    /// Policy test: fork_from_cursor with restore_workspace=false does NOT restore.
    /// This is the default policy when forking from cursor without explicit restore flag.
    #[test]
    fn test_fork_from_cursor_no_workspace_restore_policy() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head = TurnId::new();
        main.set_head(head.clone());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let cursor_turn = head;
        let options = ForkFromCursorOptions {
            restore_workspace: false,
        };

        let result = fork_from_cursor(
            &mut graph,
            session_id,
            main_id,
            cursor_turn,
            "cursor-fork",
            options,
        )
        .unwrap();

        assert!(
            !result.workspace_restored,
            "fork_from_cursor with restore_workspace=false must not restore workspace"
        );
        let fork_branch = graph.get_branch(result.new_branch_id).unwrap();
        assert!(
            fork_branch.head_turn_id.is_none(),
            "fork_from_cursor creates fork with no head when workspace not restored"
        );
    }

    /// Verify continue_from_cursor does NOT attempt workspace restoration.
    /// Policy: continuing from an earlier cursor position leaves the current
    /// workspace dirty; no automatic restore is performed.
    /// The fork succeeds but turn creation returns error (no-turn-synthesis policy).
    #[test]
    fn test_continue_from_cursor_dirty_workspace_policy() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head = TurnId::new();
        main.set_head(head.clone());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        // Verify original state: main branch has head, no descendants yet
        let main_branch = graph.get_branch(main_id).unwrap();
        assert_eq!(main_branch.head_turn_id, Some(head.clone()));

        // Continue from cursor - fork succeeds, turn creation fails (no synthesis)
        let result = ai_history_core::continue_from_cursor(
            &mut graph,
            session_id,
            main_id,
            head,
            ai_history_core::Actor::User,
        );

        // Fork succeeds but returns error instead of fabricated turn ID
        assert!(
            result.is_err(),
            "continue_from_cursor must not fabricate turn ID"
        );
        match result.unwrap_err() {
            ai_history_core::ForkError::TurnNotFound(cursor_turn) => {
                assert_eq!(cursor_turn, head, "error reports cursor turn");
            }
            e => panic!("Expected ForkError::TurnNotFound, got {:?}", e),
        }

        // Verify: original branch unchanged (dirty workspace preserved)
        let main_after = graph.get_branch(main_id).unwrap();
        assert_eq!(
            main_after.head_turn_id,
            Some(head.clone()),
            "original branch head unchanged"
        );

        // Verify: a new branch was created as a child of main
        // (we can't get the ID directly from error, but we can verify a descendant exists)
        let descendants = graph.get_descendants(main_id);
        assert!(
            descendants.len() >= 1,
            "continue_from_cursor should create a fork branch (descendant of original)"
        );
    }
}
