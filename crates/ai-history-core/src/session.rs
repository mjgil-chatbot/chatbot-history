//! Session model for ai-history
//!
//! Task 9: Complete the Session model
//! - Add `view_cursor_turn_id` for cursor navigation
//! - Add workspace binding
//! - Add retention settings
//! - Add created/updated timestamps
//! - Add project identity

use super::{BranchId, SessionId, TurnId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub title: Option<String>,
    pub workspace_root: PathBuf,
    pub active_branch_id: BranchId,
    pub view_cursor_turn_id: Option<TurnId>,
    pub project_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub retention_policy: RetentionPolicy,
}

/// Retention policy for session data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetentionPolicy {
    /// Keep all data indefinitely
    Permanent,
    /// Keep for a number of days
    Days(u32),
    /// Keep a number of checkpoints
    Checkpoints(u32),
    /// Keep a number of turns
    Turns(u32),
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::Permanent
    }
}

impl RetentionPolicy {
    pub fn days(n: u32) -> Self {
        Self::Days(n)
    }

    pub fn checkpoints(n: u32) -> Self {
        Self::Checkpoints(n)
    }

    pub fn turns(n: u32) -> Self {
        Self::Turns(n)
    }
}

impl Session {
    pub fn new(workspace_root: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            title: None,
            workspace_root,
            active_branch_id: BranchId::new(),
            view_cursor_turn_id: None,
            project_id: None,
            created_at: now,
            updated_at: now,
            retention_policy: RetentionPolicy::default(),
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_project_id(mut self, project_id: &str) -> Self {
        self.project_id = Some(project_id.to_string());
        self
    }

    pub fn with_retention_policy(mut self, policy: RetentionPolicy) -> Self {
        self.retention_policy = policy;
        self
    }

    pub fn set_view_cursor(&mut self, turn_id: TurnId) {
        self.view_cursor_turn_id = Some(turn_id);
        self.updated_at = Utc::now();
    }

    pub fn clear_view_cursor(&mut self) {
        self.view_cursor_turn_id = None;
        self.updated_at = Utc::now();
    }

    pub fn set_active_branch(&mut self, branch_id: BranchId) {
        self.active_branch_id = branch_id;
        self.updated_at = Utc::now();
    }

    pub fn update(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Get the effective turn ID for operations (cursor if set, otherwise branch head)
    pub fn effective_turn_id(&self, branch_head_turn_id: Option<TurnId>) -> Option<TurnId> {
        self.view_cursor_turn_id.or(branch_head_turn_id)
    }

    /// Check if session is at branch head (cursor at head)
    pub fn is_at_branch_head(&self) -> bool {
        self.view_cursor_turn_id.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_dir() -> PathBuf {
        let dir = std::env::temp_dir().join("session_test");
        fs::create_dir_all(&dir).ok();
        dir
    }

    fn cleanup_test_dir(dir: PathBuf) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_session_new() {
        let dir = setup_test_dir();
        let session = Session::new(dir.clone());
        assert!(session.title.is_none());
        assert_eq!(session.workspace_root, dir);
        assert!(session.view_cursor_turn_id.is_none());
        cleanup_test_dir(dir);
    }

    #[test]
    fn test_session_with_title() {
        let dir = setup_test_dir();
        let session = Session::new(dir.clone()).with_title("Test");
        assert_eq!(session.title, Some("Test".to_string()));
        cleanup_test_dir(dir);
    }

    #[test]
    fn test_session_with_project_id() {
        let dir = setup_test_dir();
        let session = Session::new(dir.clone()).with_project_id("my-project");
        assert_eq!(session.project_id, Some("my-project".to_string()));
        cleanup_test_dir(dir);
    }

    #[test]
    fn test_session_with_retention_policy() {
        let dir = setup_test_dir();
        let session = Session::new(dir.clone()).with_retention_policy(RetentionPolicy::days(30));
        assert!(matches!(
            session.retention_policy,
            RetentionPolicy::Days(30)
        ));
        cleanup_test_dir(dir);
    }

    #[test]
    fn test_session_set_view_cursor() {
        let dir = setup_test_dir();
        let mut session = Session::new(dir.clone());
        let turn_id = TurnId::new();
        session.set_view_cursor(turn_id);
        assert_eq!(session.view_cursor_turn_id, Some(turn_id));
        cleanup_test_dir(dir);
    }

    #[test]
    fn test_session_clear_view_cursor() {
        let dir = setup_test_dir();
        let mut session = Session::new(dir.clone());
        let turn_id = TurnId::new();
        session.set_view_cursor(turn_id);
        session.clear_view_cursor();
        assert!(session.view_cursor_turn_id.is_none());
        cleanup_test_dir(dir);
    }

    #[test]
    fn test_session_set_active_branch() {
        let dir = setup_test_dir();
        let mut session = Session::new(dir.clone());
        let branch_id = BranchId::new();
        session.set_active_branch(branch_id);
        assert_eq!(session.active_branch_id, branch_id);
        cleanup_test_dir(dir);
    }

    #[test]
    fn test_session_effective_turn_id_cursor_takes_precedence() {
        let dir = setup_test_dir();
        let mut session = Session::new(dir.clone());
        let cursor_turn = TurnId::new();
        let branch_head = TurnId::new();
        session.set_view_cursor(cursor_turn);
        let effective = session.effective_turn_id(Some(branch_head));
        assert_eq!(effective, Some(cursor_turn));
        cleanup_test_dir(dir);
    }

    #[test]
    fn test_session_effective_turn_id_falls_back_to_branch_head() {
        let dir = setup_test_dir();
        let mut session = Session::new(dir.clone());
        let branch_head = TurnId::new();
        // No cursor set
        let effective = session.effective_turn_id(Some(branch_head));
        assert_eq!(effective, Some(branch_head));
        cleanup_test_dir(dir);
    }

    #[test]
    fn test_session_is_at_branch_head() {
        let dir = setup_test_dir();
        let mut session = Session::new(dir.clone());
        assert!(session.is_at_branch_head()); // No cursor set

        let turn_id = TurnId::new();
        session.set_view_cursor(turn_id);
        assert!(!session.is_at_branch_head());
        cleanup_test_dir(dir);
    }

    #[test]
    fn test_session_id_roundtrip() {
        let id = SessionId::new();
        let s = id.to_string();
        let parsed: SessionId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_retention_policy_default() {
        assert!(matches!(
            RetentionPolicy::default(),
            RetentionPolicy::Permanent
        ));
    }

    #[test]
    fn test_retention_policy_helpers() {
        let p1 = RetentionPolicy::days(7);
        assert!(matches!(p1, RetentionPolicy::Days(7)));

        let p2 = RetentionPolicy::checkpoints(5);
        assert!(matches!(p2, RetentionPolicy::Checkpoints(5)));

        let p3 = RetentionPolicy::turns(100);
        assert!(matches!(p3, RetentionPolicy::Turns(100)));
    }
}
