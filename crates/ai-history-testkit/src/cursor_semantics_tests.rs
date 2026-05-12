//! Timeline cursor semantics tests
//!
//! Task 21: Finalize timeline cursor semantics
//!
//! Verify:
//! - TimelineCursor exists with position tracking
//! - CursorPosition variants (AtHead, AtTurn)
//! - NavigationContext for managing cursor state
//! - ForwardTarget and AmbiguousForwardTargets for forward navigation
//! - Cursor navigation does NOT mutate history or workspace

#[cfg(test)]
mod test_cursor_position {
    use ai_history_core::{BranchId, CursorPosition, SessionId, TimelineCursor, TurnId};

    /// Verify TimelineCursor exists at head by default
    #[test]
    fn test_cursor_new_at_head() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let cursor = TimelineCursor::new(session_id, branch_id);
        assert!(cursor.is_at_head());
        assert!(cursor.current_turn_id().is_none());
    }

    /// Verify TimelineCursor at specific turn
    #[test]
    fn test_cursor_at_turn() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn_id = TurnId::new();
        let cursor = TimelineCursor::at_turn(session_id, branch_id, turn_id);
        assert!(cursor.is_at_turn(turn_id));
    }

    /// Verify go_to_head
    #[test]
    fn test_cursor_go_to_head() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut cursor = TimelineCursor::new(session_id, branch_id);
        let turn_id = TurnId::new();
        cursor.go_to_turn(turn_id);
        assert!(!cursor.is_at_head());

        cursor.go_to_head();
        assert!(cursor.is_at_head());
    }

    /// Verify go_to_turn
    #[test]
    fn test_cursor_go_to_turn() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut cursor = TimelineCursor::new(session_id, branch_id);
        let turn_id = TurnId::new();
        cursor.go_to_turn(turn_id);
        assert!(cursor.is_at_turn(turn_id));
        assert_eq!(cursor.current_turn_id(), Some(turn_id));
    }

    /// Verify navigate_backward returns parent
    #[test]
    fn test_cursor_navigate_backward() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let cursor = TimelineCursor::new(session_id, branch_id);
        let parent_id = TurnId::new();
        let result = cursor.navigate_backward(Some(parent_id));
        assert_eq!(result, Some(parent_id));
    }

    /// Verify navigate_backward returns None when no parent
    #[test]
    fn test_cursor_navigate_backward_no_parent() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let cursor = TimelineCursor::new(session_id, branch_id);
        let result = cursor.navigate_backward(None);
        assert!(result.is_none());
    }

    /// Verify navigate_forward returns registered children
    #[test]
    fn test_cursor_navigate_forward() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut cursor = TimelineCursor::new(session_id, branch_id);
        let parent_id = TurnId::new();
        let child1 = TurnId::new();
        let child2 = TurnId::new();
        cursor.register_children(parent_id, vec![child1.clone(), child2.clone()]);
        cursor.go_to_turn(parent_id);

        let forward = cursor.navigate_forward();
        assert_eq!(forward.len(), 2);
        assert!(forward.contains(&child1));
        assert!(forward.contains(&child2));
    }

    /// Verify navigate_forward at head returns empty
    #[test]
    fn test_cursor_navigate_forward_at_head() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let cursor = TimelineCursor::new(session_id, branch_id);
        let forward = cursor.navigate_forward();
        assert!(forward.is_empty());
    }
}

#[cfg(test)]
mod test_forward_targets {
    use ai_history_core::{AmbiguousForwardTargets, BranchId, ForwardTarget, TurnId};

    /// Verify ForwardTarget exists
    #[test]
    fn test_forward_target_exists() {
        let target = ForwardTarget {
            turn_id: TurnId::new(),
            branch_id: BranchId::new(),
            branch_name: Some("feature".to_string()),
            is_abandoned_future: true,
        };
        assert!(target.is_abandoned_future);
        assert_eq!(target.branch_name, Some("feature".to_string()));
    }

    /// Verify AmbiguousForwardTargets exists
    #[test]
    fn test_ambiguous_forward_targets_new() {
        let targets = AmbiguousForwardTargets::new(TurnId::new());
        assert!(targets.has_none());
        assert_eq!(targets.len(), 0);
    }

    /// Verify add_target increases count
    #[test]
    fn test_ambiguous_add_target() {
        let mut targets = AmbiguousForwardTargets::new(TurnId::new());
        targets.add_target(ForwardTarget {
            turn_id: TurnId::new(),
            branch_id: BranchId::new(),
            branch_name: None,
            is_abandoned_future: false,
        });
        assert!(!targets.has_none());
        assert_eq!(targets.len(), 1);
    }

    /// Verify has_multiple detection
    #[test]
    fn test_has_multiple() {
        let mut targets = AmbiguousForwardTargets::new(TurnId::new());
        assert!(!targets.has_multiple());

        targets.add_target(ForwardTarget {
            turn_id: TurnId::new(),
            branch_id: BranchId::new(),
            branch_name: None,
            is_abandoned_future: false,
        });
        targets.add_target(ForwardTarget {
            turn_id: TurnId::new(),
            branch_id: BranchId::new(),
            branch_name: None,
            is_abandoned_future: false,
        });
        assert!(targets.has_multiple());
    }

    /// Verify get_single when single target
    #[test]
    fn test_get_single() {
        let mut targets = AmbiguousForwardTargets::new(TurnId::new());
        let turn_id = TurnId::new();
        targets.add_target(ForwardTarget {
            turn_id,
            branch_id: BranchId::new(),
            branch_name: None,
            is_abandoned_future: false,
        });
        assert_eq!(targets.get_single(), Some(turn_id));
    }

    /// Verify get_single returns None when multiple
    #[test]
    fn test_get_single_ambiguous() {
        let mut targets = AmbiguousForwardTargets::new(TurnId::new());
        targets.add_target(ForwardTarget {
            turn_id: TurnId::new(),
            branch_id: BranchId::new(),
            branch_name: None,
            is_abandoned_future: false,
        });
        targets.add_target(ForwardTarget {
            turn_id: TurnId::new(),
            branch_id: BranchId::new(),
            branch_name: None,
            is_abandoned_future: false,
        });
        assert!(targets.get_single().is_none());
    }

    /// Verify current_branch_targets filter
    #[test]
    fn test_current_branch_targets() {
        let mut targets = AmbiguousForwardTargets::new(TurnId::new());
        let branch_id = BranchId::new();
        targets.add_target(ForwardTarget {
            turn_id: TurnId::new(),
            branch_id,
            branch_name: None,
            is_abandoned_future: false,
        });
        targets.add_target(ForwardTarget {
            turn_id: TurnId::new(),
            branch_id,
            branch_name: None,
            is_abandoned_future: true,
        });

        let current = targets.current_branch_targets();
        let abandoned = targets.abandoned_future_targets();
        assert_eq!(current.len(), 1);
        assert_eq!(abandoned.len(), 1);
    }
}

#[cfg(test)]
mod test_navigation_context {
    use ai_history_core::{BranchId, NavigationContext, SessionId, TurnId};
    use std::collections::HashMap;

    /// Verify NavigationContext at branch head
    #[test]
    fn test_navigation_context_at_head() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let ctx = NavigationContext::new(session_id, branch_id, Some(TurnId::new()));
        assert!(ctx.is_cursor_at_branch_head());
    }

    /// Verify effective_turn_id with cursor set
    #[test]
    fn test_effective_turn_id_cursor_precedence() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let branch_head = TurnId::new();
        let cursor_turn = TurnId::new();

        let mut ctx = NavigationContext::new(session_id, branch_id, Some(branch_head));
        ctx.cursor.go_to_turn(cursor_turn);

        assert_eq!(ctx.effective_turn_id(), Some(cursor_turn));
    }

    /// Verify effective_turn_id falls back to branch head
    #[test]
    fn test_effective_turn_id_fallback() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let branch_head = TurnId::new();

        let ctx = NavigationContext::new(session_id, branch_id, Some(branch_head));
        assert_eq!(ctx.effective_turn_id(), Some(branch_head));
    }

    /// Verify return_to_head
    #[test]
    fn test_return_to_head() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut ctx = NavigationContext::new(session_id, branch_id, Some(TurnId::new()));

        ctx.cursor.go_to_turn(TurnId::new());
        assert!(!ctx.is_cursor_at_branch_head());

        ctx.return_to_head();
        assert!(ctx.is_cursor_at_branch_head());
    }

    /// Verify navigate_backward_to
    #[test]
    fn test_navigate_backward_to() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut ctx = NavigationContext::new(session_id, branch_id, Some(TurnId::new()));

        let parent = TurnId::new();
        ctx.navigate_backward_to(parent);
        assert_eq!(ctx.cursor.current_turn_id(), Some(parent));
        assert!(!ctx.is_cursor_at_branch_head());
    }

    /// Verify can_navigate_backward
    #[test]
    fn test_can_navigate_backward() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut ctx = NavigationContext::new(session_id, branch_id, Some(TurnId::new()));

        assert!(!ctx.can_navigate_backward()); // At head

        ctx.cursor.go_to_turn(TurnId::new());
        assert!(ctx.can_navigate_backward());
    }
}
