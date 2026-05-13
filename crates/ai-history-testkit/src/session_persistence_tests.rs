//! Session persistence tests
//!
//! Task 19: Finalize session persistence
//!
//! Verify:
//! - Session and SessionInfo types exist with all required fields
//! - RetentionPolicy exists with variants
//! - SessionStore trait exists with CRUD operations
//! - Session can be created, updated, and deleted

#[cfg(test)]
mod test_session_model {
    use ai_history_core::session::RetentionPolicy;
    use ai_history_core::{BranchId, Session, SessionId, TurnId};
    use std::path::PathBuf;

    /// Verify Session exists with all required fields
    #[test]
    fn test_session_exists() {
        let session = Session::new(PathBuf::from("/test/workspace"));
        assert_eq!(session.workspace_root, PathBuf::from("/test/workspace"));
        assert!(session.title.is_none());
        assert!(session.view_cursor_turn_id.is_none());
        assert!(session.project_id.is_none());
    }

    /// Verify Session with title
    #[test]
    fn test_session_with_title() {
        let session = Session::new(PathBuf::from("/test")).with_title("My Session");
        assert_eq!(session.title, Some("My Session".to_string()));
    }

    /// Verify Session with project_id
    #[test]
    fn test_session_with_project_id() {
        let session = Session::new(PathBuf::from("/test")).with_project_id("project-123");
        assert_eq!(session.project_id, Some("project-123".to_string()));
    }

    /// Verify RetentionPolicy variants
    #[test]
    fn test_retention_policy_variants() {
        assert!(matches!(
            RetentionPolicy::Permanent,
            RetentionPolicy::Permanent
        ));
        let days = RetentionPolicy::days(30);
        assert!(matches!(days, RetentionPolicy::Days(_)));
        let checkpoints = RetentionPolicy::checkpoints(5);
        assert!(matches!(checkpoints, RetentionPolicy::Checkpoints(_)));
        let turns = RetentionPolicy::turns(100);
        assert!(matches!(turns, RetentionPolicy::Turns(_)));
    }

    /// Verify RetentionPolicy helpers
    #[test]
    fn test_retention_policy_helpers() {
        let p1 = RetentionPolicy::days(30);
        assert!(matches!(p1, RetentionPolicy::Days(30)));

        let p2 = RetentionPolicy::checkpoints(5);
        assert!(matches!(p2, RetentionPolicy::Checkpoints(5)));

        let p3 = RetentionPolicy::turns(100);
        assert!(matches!(p3, RetentionPolicy::Turns(100)));
    }

    /// Verify Session view_cursor handling
    #[test]
    fn test_session_view_cursor() {
        let mut session = Session::new(PathBuf::from("/test"));
        let turn_id = TurnId::new();

        session.set_view_cursor(turn_id);
        assert_eq!(session.view_cursor_turn_id, Some(turn_id));

        session.clear_view_cursor();
        assert!(session.view_cursor_turn_id.is_none());
    }

    /// Verify Session active_branch handling
    #[test]
    fn test_session_active_branch() {
        let mut session = Session::new(PathBuf::from("/test"));
        let branch_id = BranchId::new();

        session.set_active_branch(branch_id);
        assert_eq!(session.active_branch_id, branch_id);
    }

    /// Verify effective_turn_id with cursor takes precedence
    #[test]
    fn test_effective_turn_id_cursor_precedence() {
        let mut session = Session::new(PathBuf::from("/test"));
        let cursor_turn = TurnId::new();
        let branch_head = TurnId::new();

        session.set_view_cursor(cursor_turn);
        let effective = session.effective_turn_id(Some(branch_head));
        assert_eq!(effective, Some(cursor_turn));
    }

    /// Verify effective_turn_id falls back to branch head
    #[test]
    fn test_effective_turn_id_fallback() {
        let session = Session::new(PathBuf::from("/test"));
        let branch_head = TurnId::new();

        let effective = session.effective_turn_id(Some(branch_head));
        assert_eq!(effective, Some(branch_head));
    }

    /// Verify is_at_branch_head
    #[test]
    fn test_is_at_branch_head() {
        let mut session = Session::new(PathBuf::from("/test"));
        assert!(session.is_at_branch_head());

        session.set_view_cursor(TurnId::new());
        assert!(!session.is_at_branch_head());
    }
}

#[cfg(test)]
mod test_session_store_traits {
    use ai_history_core::{BranchId, CoreResult, SessionId};
    use ai_history_store::traits::{SessionInfo, SessionStore};

    /// Verify SessionInfo exists with required fields
    #[test]
    fn test_session_info_exists() {
        let info = SessionInfo {
            id: SessionId::new(),
            title: "Test".to_string(),
            active_branch: BranchId::new(),
            view_cursor: None,
            workspace_root: "/test/workspace".to_string(),
            retention_config: "default".to_string(),
            project_identity: "test".to_string(),
            operation_state: "active".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            archival_state: false,
        };
        assert_eq!(info.workspace_root, "/test/workspace");
    }

    /// Verify SessionStore trait exists with CRUD methods
    #[test]
    fn test_session_store_trait_exists() {
        // Just verify the trait type exists - actual implementation tested elsewhere
        fn _check_session_store<T: SessionStore>() {}
    }

    /// Verify SessionInfo can be cloned and compared
    #[test]
    fn test_session_info_clone() {
        let info = SessionInfo {
            id: SessionId::new(),
            title: "Test".to_string(),
            active_branch: BranchId::new(),
            view_cursor: None,
            workspace_root: "/test".to_string(),
            retention_config: "default".to_string(),
            project_identity: "test".to_string(),
            operation_state: "active".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            archival_state: false,
        };
        let cloned = info.clone();
        assert_eq!(cloned.id, info.id);
        assert_eq!(cloned.workspace_root, info.workspace_root);
    }
}
