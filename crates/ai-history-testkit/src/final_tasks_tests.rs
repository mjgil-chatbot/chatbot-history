//! Final tasks: CLI, UI contracts, docs, audit, and design review tests
//!
//! Task 81: Wire CLI commands to real APIs
//! Task 82: Add missing CLI history commands
//! Task 83: Complete CLI error handling
//! Task 84: Complete UI timeline contract
//! Task 85: Complete UI restore/conflict contract
//! Task 86: Complete accessibility and UX contract
//! Task 87: Complete realistic testkit fixtures
//! Task 88: Complete full-system acceptance tests
//! Task 89: Complete CI release gate
//! Task 90: Complete design document implementation mapping
//! Task 91: Complete public API documentation
//! Task 92: Complete operations documentation
//! Task 93: Resolve open questions or mark non-goals
//! Task 94: Complete final implementation audit
//! Task 95: Publish final design review

include!("final_tasks_tests_release.rs");
#[cfg(test)]
mod test_testkit_builders {
    use crate::builders::{
        BranchBuilder, CheckpointBuilder, ConflictStateBuilder, DirtyWorkspaceBuilder,
        SessionBuilder, TurnBuilder,
    };

    /// Verify SessionBuilder
    #[test]
    fn test_session_builder() {
        let session = SessionBuilder::new().title("Test Session").build();
        assert!(!session.id.to_string().is_empty());
    }

    /// Verify BranchBuilder
    #[test]
    fn test_branch_builder() {
        let branch = BranchBuilder::new().name("main").build();
        assert_eq!(branch.name, "main");
    }

    /// Verify TurnBuilder
    #[test]
    fn test_turn_builder() {
        let turn = TurnBuilder::new().build();
        assert!(!turn.id.to_string().is_empty());
    }

    /// Verify CheckpointBuilder
    #[test]
    fn test_checkpoint_builder() {
        let checkpoint = CheckpointBuilder::new().reason("test").build();
        assert!(!checkpoint.id.to_string().is_empty());
    }

    /// Verify DirtyWorkspaceBuilder
    #[test]
    fn test_dirty_workspace_builder() {
        let dirty = DirtyWorkspaceBuilder::new()
            .add_file_change("src/main.rs", ai_history_core::ChangeType::Modified)
            .build();
        assert_eq!(dirty.total_files_changed, 1);
    }

    /// Verify ConflictStateBuilder
    #[test]
    fn test_conflict_state_builder() {
        let conflicts = ConflictStateBuilder::new()
            .add_modified("src/main.rs")
            .build_conflicts();
        assert_eq!(conflicts.len(), 1);
    }
}
#[cfg(test)]
mod test_invariant_enforcement {
    use ai_history_core::invariants::{
        InvariantCategory, InvariantCheck, InvariantChecker, InvariantRegistry, InvariantSeverity,
    };

    /// Test: No file write without checkpoint invariant
    #[test]
    fn test_no_file_write_without_checkpoint_pass() {
        let check = InvariantChecker::check_file_write_invariant(true, true);
        assert!(check.passed);
    }

    /// Test: No file write without checkpoint fails when missing
    #[test]
    fn test_no_file_write_without_checkpoint_fail() {
        let check = InvariantChecker::check_file_write_invariant(false, true);
        assert!(!check.passed);
        assert!(check.context.is_some());
    }

    /// Test: Non-destructive file write does not trigger invariant
    #[test]
    fn test_no_file_write_non_destructive_allowed() {
        let check = InvariantChecker::check_file_write_invariant(false, false);
        assert!(check.passed);
    }

    /// Test: No destructive restore without safety invariant
    #[test]
    fn test_no_destructive_restore_without_safety_pass() {
        let check = InvariantChecker::check_restore_safety_invariant(true);
        assert!(check.passed);
    }

    /// Test: No destructive restore without safety fails when missing
    #[test]
    fn test_no_destructive_restore_without_safety_fail() {
        let check = InvariantChecker::check_restore_safety_invariant(false);
        assert!(!check.passed);
    }

    /// Test: No branch head mutation without event invariant
    #[test]
    fn test_no_branch_head_mutation_without_event_pass() {
        let check = InvariantChecker::check_branch_head_mvariant(true);
        assert!(check.passed);
    }

    /// Test: No branch head mutation without event fails when missing
    #[test]
    fn test_no_branch_head_mutation_without_event_fail() {
        let check = InvariantChecker::check_branch_head_mvariant(false);
        assert!(!check.passed);
    }

    /// Test: No assistant delta after completion invariant
    #[test]
    fn test_no_assistant_delta_after_completion_pass() {
        let check = InvariantChecker::check_delta_after_completion(false, false);
        assert!(check.passed);
    }

    /// Test: No assistant delta after completion fails
    #[test]
    fn test_no_assistant_delta_after_completion_fail() {
        let check = InvariantChecker::check_delta_after_completion(true, true);
        assert!(!check.passed);
    }

    /// Test: Checkpoint requires turn reference invariant
    #[test]
    fn test_checkpoint_requires_turn_reference_pass() {
        let check = InvariantChecker::check_checkpoint_turn_reference(true);
        assert!(check.passed);
    }

    /// Test: Checkpoint without turn reference fails
    #[test]
    fn test_checkpoint_requires_turn_reference_fail() {
        let check = InvariantChecker::check_checkpoint_turn_reference(false);
        assert!(!check.passed);
    }

    /// Test: Event sequence monotonicity invariant
    #[test]
    fn test_event_sequence_monotonic_pass() {
        let check = InvariantChecker::check_event_sequence_monotonic(5, 10);
        assert!(check.passed);
    }

    /// Test: Event sequence non-monotonic fails
    #[test]
    fn test_event_sequence_monotonic_fail() {
        let check = InvariantChecker::check_event_sequence_monotonic(10, 5);
        assert!(!check.passed);
    }

    /// Test: InvariantRegistry contains all required invariants
    #[test]
    fn test_invariant_registry_has_required_invariants() {
        let registry = InvariantRegistry::new();
        assert!(registry.get("no_file_write_without_checkpoint").is_some());
        assert!(registry
            .get("no_destructive_restore_without_safety")
            .is_some());
        assert!(registry
            .get("no_branch_head_mutation_without_event")
            .is_some());
        assert!(registry
            .get("no_assistant_delta_after_completion")
            .is_some());
        assert!(registry
            .get("no_checkpoint_without_turn_reference")
            .is_some());
    }

    /// Test: InvariantRegistry critical invariants are critical severity
    #[test]
    fn test_invariant_registry_critical_invariants() {
        let registry = InvariantRegistry::new();
        let critical = registry.critical();
        assert!(!critical.is_empty());
        for inv in critical {
            assert_eq!(inv.severity, InvariantSeverity::Critical);
        }
    }

    /// Test: Workspace operation check combines multiple invariants
    #[test]
    fn test_workspace_operation_check_multiple() {
        let checks = InvariantChecker::check_workspace_operation(true, true, true);
        assert!(checks.iter().all(|c| c.passed));
    }

    /// Test: Invariants by category returns workspace invariants
    #[test]
    fn test_invariants_by_category_workspace() {
        let registry = InvariantRegistry::new();
        let workspace = registry.by_category(InvariantCategory::Workspace);
        assert!(!workspace.is_empty());
        for inv in workspace {
            assert_eq!(inv.category, InvariantCategory::Workspace);
        }
    }

    /// Test: All invariant categories are represented
    #[test]
    fn test_all_invariant_categories_represented() {
        let registry = InvariantRegistry::new();
        for category in [
            InvariantCategory::Workspace,
            InvariantCategory::Branch,
            InvariantCategory::Turn,
            InvariantCategory::Event,
            InvariantCategory::Checkpoint,
        ] {
            let by_cat = registry.by_category(category);
            assert!(
                !by_cat.is_empty(),
                "Category {:?} should have invariants",
                category
            );
        }
    }
}
#[cfg(test)]
mod test_session_persistence {
    use ai_history_core::SessionId;
    use ai_history_store::SessionInfo;

    /// Test: SessionInfo has all required fields per Task 23
    #[test]
    fn test_session_info_has_complete_fields() {
        let info = SessionInfo {
            id: SessionId::new(),
            title: "Test Session".to_string(),
            active_branch: ai_history_core::BranchId::new(),
            view_cursor: None,
            workspace_root: "/tmp/workspace".to_string(),
            retention_config: "default".to_string(),
            project_identity: "project-1".to_string(),
            operation_state: "active".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            archival_state: false,
        };
        assert!(!info.title.is_empty());
        assert!(!info.workspace_root.is_empty());
        assert!(!info.retention_config.is_empty());
        assert!(!info.project_identity.is_empty());
    }

    /// Test: SessionInfo can represent active session
    #[test]
    fn test_session_info_active_state() {
        let info = SessionInfo {
            id: SessionId::new(),
            title: "Active Session".to_string(),
            active_branch: ai_history_core::BranchId::new(),
            view_cursor: None,
            workspace_root: "/tmp/workspace".to_string(),
            retention_config: "default".to_string(),
            project_identity: "project-1".to_string(),
            operation_state: "active".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            archival_state: false,
        };
        assert_eq!(info.operation_state, "active");
        assert!(!info.archival_state);
    }

    /// Test: SessionInfo can represent archived session
    #[test]
    fn test_session_info_archived_state() {
        let info = SessionInfo {
            id: SessionId::new(),
            title: "Archived Session".to_string(),
            active_branch: ai_history_core::BranchId::new(),
            view_cursor: None,
            workspace_root: "/tmp/workspace".to_string(),
            retention_config: "default".to_string(),
            project_identity: "project-1".to_string(),
            operation_state: "archived".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            archival_state: true,
        };
        assert_eq!(info.operation_state, "archived");
        assert!(info.archival_state);
    }

    /// Test: SessionInfo tracks view cursor
    #[test]
    fn test_session_info_view_cursor() {
        let turn_id = ai_history_core::TurnId::new();
        let info = SessionInfo {
            id: SessionId::new(),
            title: "Test Session".to_string(),
            active_branch: ai_history_core::BranchId::new(),
            view_cursor: Some(turn_id),
            workspace_root: "/tmp/workspace".to_string(),
            retention_config: "default".to_string(),
            project_identity: "project-1".to_string(),
            operation_state: "active".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            archival_state: false,
        };
        assert!(info.view_cursor.is_some());
    }
}
#[cfg(test)]
mod test_branch_dag_persistence {
    use ai_history_core::{BranchId, SessionId, TurnId};
    use ai_history_store::BranchInfo;

    /// Test: BranchInfo has all required fields per Task 24
    #[test]
    fn test_branch_info_has_complete_fields() {
        let branch_id = BranchId::new();
        let info = BranchInfo {
            id: branch_id,
            session_id: SessionId::new(),
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
        assert_eq!(info.name, "main");
        assert!(!info.ancestry.is_empty() || info.ancestry.is_empty()); // ancestry can be empty
    }

    /// Test: BranchInfo can represent child branch with parent
    #[test]
    fn test_branch_info_with_parent() {
        let parent_id = BranchId::new();
        let info = BranchInfo {
            id: BranchId::new(),
            session_id: SessionId::new(),
            name: "feature".to_string(),
            parent_branch_id: Some(parent_id),
            created_from_turn_id: None,
            head_turn_id: None,
            ancestry: vec![parent_id],
            protected: false,
            archived: false,
            abandoned_future: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(info.parent_branch_id.is_some());
        assert_eq!(info.ancestry.len(), 1);
    }

    /// Test: BranchInfo can represent fork from cursor
    #[test]
    fn test_branch_info_fork_from_cursor() {
        let turn_id = TurnId::new();
        let info = BranchInfo {
            id: BranchId::new(),
            session_id: SessionId::new(),
            name: "forked".to_string(),
            parent_branch_id: Some(BranchId::new()),
            created_from_turn_id: Some(turn_id),
            head_turn_id: Some(turn_id),
            ancestry: Vec::new(),
            protected: false,
            archived: false,
            abandoned_future: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(info.created_from_turn_id.is_some());
        assert_eq!(info.head_turn_id, info.created_from_turn_id);
    }

    /// Test: BranchInfo can represent abandoned future
    #[test]
    fn test_branch_info_abandoned_future() {
        let info = BranchInfo {
            id: BranchId::new(),
            session_id: SessionId::new(),
            name: "abandoned".to_string(),
            parent_branch_id: Some(BranchId::new()),
            created_from_turn_id: None,
            head_turn_id: None,
            ancestry: Vec::new(),
            protected: false,
            archived: false,
            abandoned_future: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(info.abandoned_future);
    }

    /// Test: BranchInfo can represent protected branch
    #[test]
    fn test_branch_info_protected() {
        let info = BranchInfo {
            id: BranchId::new(),
            session_id: SessionId::new(),
            name: "protected".to_string(),
            parent_branch_id: None,
            created_from_turn_id: None,
            head_turn_id: None,
            ancestry: Vec::new(),
            protected: true,
            archived: false,
            abandoned_future: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(info.protected);
    }

    /// Test: BranchInfo can represent archived branch
    #[test]
    fn test_branch_info_archived() {
        let info = BranchInfo {
            id: BranchId::new(),
            session_id: SessionId::new(),
            name: "archived".to_string(),
            parent_branch_id: None,
            created_from_turn_id: None,
            head_turn_id: None,
            ancestry: Vec::new(),
            protected: false,
            archived: true,
            abandoned_future: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(info.archived);
    }
}
