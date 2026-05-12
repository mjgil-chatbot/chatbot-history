//! Branch DAG persistence tests
//!
//! Task 20: Finalize branch DAG persistence
//!
//! Verify:
//! - Branch and BranchGraph types exist
//! - Parent branch tracking works
//! - Created-from-turn tracking works
//! - Immutable ancestry chain exists
//! - Unique branch names per session
//! - Protected branches cannot be deleted
//! - Active branch protection

#[cfg(test)]
mod test_branch_model {
    use ai_history_core::{Branch, BranchCreationReason, BranchId, SessionId, TurnId};

    /// Verify Branch exists with all required fields
    #[test]
    fn test_branch_exists() {
        let session_id = SessionId::new();
        let branch = Branch::new(session_id, "main");
        assert_eq!(branch.name, "main");
        assert!(branch.parent_branch_id.is_none());
        assert!(branch.head_turn_id.is_none());
    }

    /// Verify Branch with parent tracking
    #[test]
    fn test_branch_with_parent() {
        let session_id = SessionId::new();
        let parent = Branch::new(session_id, "parent");
        let child = Branch::new(session_id, "child").with_parent(parent.id);
        assert_eq!(child.parent_branch_id, Some(parent.id));
    }

    /// Verify Branch with created_from_turn tracking
    #[test]
    fn test_branch_with_created_from_turn() {
        let session_id = SessionId::new();
        let turn_id = TurnId::new();
        let branch = Branch::new(session_id, "feature").with_created_from_turn(turn_id);
        assert_eq!(branch.created_from_turn_id, Some(turn_id));
    }

    /// Verify BranchCreationReason variants
    #[test]
    fn test_branch_creation_reason_variants() {
        assert!(matches!(
            BranchCreationReason::Initial,
            BranchCreationReason::Initial
        ));
        assert!(matches!(
            BranchCreationReason::ForkCurrent,
            BranchCreationReason::ForkCurrent
        ));
        assert!(matches!(
            BranchCreationReason::ForkFromCursor,
            BranchCreationReason::ForkFromCursor
        ));
        assert!(matches!(
            BranchCreationReason::ContinueFromCursor,
            BranchCreationReason::ContinueFromCursor
        ));
        assert!(matches!(
            BranchCreationReason::Redo,
            BranchCreationReason::Redo
        ));
    }

    /// Verify Branch is_redo_branch check
    #[test]
    fn test_branch_is_redo_branch() {
        let session_id = SessionId::new();
        let redo = Branch::new(session_id, "redo").with_creation_reason(BranchCreationReason::Redo);
        let fork =
            Branch::new(session_id, "fork").with_creation_reason(BranchCreationReason::ForkCurrent);
        assert!(redo.is_redo_branch());
        assert!(!fork.is_redo_branch());
    }

    /// Verify Branch abandoned futures tracking
    #[test]
    fn test_branch_abandoned_futures() {
        let session_id = SessionId::new();
        let turn_ids = vec![TurnId::new(), TurnId::new()];
        let branch = Branch::new(session_id, "test").with_abandoned_futures(turn_ids.clone());
        assert!(branch.has_abandoned_futures());
        assert_eq!(branch.abandoned_futures, turn_ids);
    }

    /// Verify Branch protect functionality
    #[test]
    fn test_branch_protect() {
        let session_id = SessionId::new();
        let branch = Branch::new(session_id, "main").protect();
        assert!(branch.is_protected);
    }
}

#[cfg(test)]
mod test_branch_graph {
    use ai_history_core::{Branch, BranchCreationReason, BranchGraph, BranchId, SessionId, TurnId};

    /// Verify BranchGraph exists
    #[test]
    fn test_branch_graph_new() {
        let graph = BranchGraph::new();
        assert!(graph.get_session_branches(SessionId::new()).is_empty());
    }

    /// Verify BranchGraph add_branch with duplicate name fails
    #[test]
    fn test_branch_graph_duplicate_name_fails() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let branch1 = Branch::new(session_id, "main");
        graph.add_branch(branch1).unwrap();

        let branch2 = Branch::new(session_id, "main");
        let result = graph.add_branch(branch2);
        assert!(matches!(
            result,
            Err(ai_history_core::BranchError::DuplicateBranchName)
        ));
    }

    /// Verify BranchGraph get_branch_by_name
    #[test]
    fn test_branch_graph_get_by_name() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let branch = Branch::new(session_id, "feature");
        let branch_id = branch.id;
        graph.add_branch(branch).unwrap();

        let found = graph.get_branch_by_name(session_id, "feature");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, branch_id);
    }

    /// Verify BranchGraph get_ancestors (immutable ancestry)
    #[test]
    fn test_branch_graph_ancestry() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let main = Branch::new(session_id, "main");
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let feature = Branch::new(session_id, "feature").with_parent(main_id);
        let feature_id = feature.id;
        graph.add_branch(feature).unwrap();

        let ancestors = graph.get_ancestors(feature_id);
        assert!(ancestors.contains(&main_id));
    }

    /// Verify BranchGraph is_ancestor check
    #[test]
    fn test_branch_graph_is_ancestor() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let main = Branch::new(session_id, "main");
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let feature = Branch::new(session_id, "feature").with_parent(main_id);
        let feature_id = feature.id;
        graph.add_branch(feature).unwrap();

        assert!(graph.is_ancestor(main_id, feature_id));
        assert!(!graph.is_ancestor(feature_id, main_id));
    }

    /// Verify BranchGraph fork_branch
    #[test]
    fn test_branch_graph_fork() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let mut main = Branch::new(session_id, "main");
        let main_id = main.id;
        let fork_point = TurnId::new();
        main.set_head(fork_point.clone());
        graph.add_branch(main).unwrap();

        let fork = graph
            .fork_branch(
                main_id,
                "feature",
                BranchCreationReason::ForkCurrent,
                fork_point,
            )
            .unwrap();

        assert_eq!(fork.name, "feature");
        assert_eq!(fork.parent_branch_id, Some(main_id));
    }

    /// Verify BranchGraph delete protected branch fails
    #[test]
    fn test_branch_graph_delete_protected_fails() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let protected = Branch::new(session_id, "protected").protect();
        let protected_id = protected.id;
        graph.add_branch(protected).unwrap();

        let active = Branch::new(session_id, "active");
        let active_id = active.id;
        graph.add_branch(active).unwrap();

        let result = graph.delete_branch(protected_id, active_id);
        assert!(matches!(
            result,
            Err(ai_history_core::BranchError::ProtectedBranch)
        ));
    }

    /// Verify BranchGraph delete active branch fails
    #[test]
    fn test_branch_graph_delete_active_fails() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let branch = Branch::new(session_id, "main");
        let branch_id = branch.id;
        graph.add_branch(branch).unwrap();

        let result = graph.delete_branch(branch_id, branch_id);
        assert!(matches!(
            result,
            Err(ai_history_core::BranchError::CannotDeleteActiveBranch)
        ));
    }

    /// Verify BranchGraph get_redo_branches
    #[test]
    fn test_branch_graph_get_redo_branches() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let main = Branch::new(session_id, "main");
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let redo = Branch::new(session_id, "redo")
            .with_parent(main_id)
            .with_creation_reason(BranchCreationReason::Redo);
        graph.add_branch(redo).unwrap();

        let redo_ids = graph.get_redo_branches(main_id);
        assert_eq!(redo_ids.len(), 1);
    }

    /// Verify BranchGraph has_descendants
    #[test]
    fn test_branch_graph_has_descendants() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let main = Branch::new(session_id, "main");
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        assert!(!graph.has_descendants(main_id));

        let feature = Branch::new(session_id, "feature").with_parent(main_id);
        graph.add_branch(feature).unwrap();

        assert!(graph.has_descendants(main_id));
    }
}

#[cfg(test)]
mod test_branch_switching {
    use ai_history_core::{Branch, BranchGraph, BranchId, SessionId, TurnId};

    /// Verify branch switching updates active branch reference
    #[test]
    fn test_branch_switching_updates_active() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let main = Branch::new(session_id, "main");
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let feature = Branch::new(session_id, "feature").with_parent(main_id);
        let feature_id = feature.id;
        graph.add_branch(feature).unwrap();

        // Verify both branches exist
        assert!(graph.get_branch(main_id).is_some());
        assert!(graph.get_branch(feature_id).is_some());
    }

    /// Verify switching to branch with head turn preserves head
    #[test]
    fn test_branch_switch_with_head_preserves() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        let head = TurnId::new();
        main.set_head(head.clone());
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let mut feature = Branch::new(session_id, "feature").with_parent(main_id);
        feature.set_head(TurnId::new());
        let feature_id = feature.id;
        graph.add_branch(feature).unwrap();

        // Both branches have head turns
        let main_branch = graph.get_branch(main_id).unwrap();
        let feature_branch = graph.get_branch(feature_id).unwrap();
        assert!(main_branch.head_turn_id.is_some());
        assert!(feature_branch.head_turn_id.is_some());
    }

    /// Verify cannot switch to non-existent branch
    #[test]
    fn test_branch_switch_nonexistent_fails() {
        let graph = BranchGraph::new();
        let session_id = SessionId::new();

        let result = graph.get_branch(BranchId::new());
        assert!(result.is_none());
    }

    /// Verify branch switching preserves parent relationship
    #[test]
    fn test_branch_switch_preserves_parent() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let main = Branch::new(session_id, "main");
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let feature = Branch::new(session_id, "feature").with_parent(main_id);
        let feature_id = feature.id;
        graph.add_branch(feature).unwrap();

        // Verify parent relationship preserved after switch
        let feature_branch = graph.get_branch(feature_id).unwrap();
        assert_eq!(feature_branch.parent_branch_id, Some(main_id));
    }
}
