//! Active session and branch resolution helpers
//!
//! Provides a unified API for resolving the active session and branch
//! across CLI commands, using persisted app state with sensible fallbacks.

use crate::{BranchInfo, SqliteStore};
use ai_history_core::{BranchId, CoreError, CoreResult, SessionId};

/// Resolution result for active session and branch
#[derive(Debug, Clone)]
pub struct ActiveSessionResolution {
    pub session: crate::sqlite_store::SessionInfo,
    pub active_branch: Option<BranchInfo>,
    pub was_created: bool,
}

/// Result of resolving active branch specifically
#[derive(Debug, Clone)]
pub struct ActiveBranchResolution {
    pub session_id: SessionId,
    pub branch: BranchInfo,
    pub was_created: bool,
}

impl SqliteStore {
    /// Resolve the active session, creating one if none exists.
    ///
    /// Uses the persisted active_session_id from app_state if available,
    /// otherwise falls back to the first session in the store.
    /// If no sessions exist, creates a new session with the given workspace root.
    pub fn resolve_active_session(
        &self,
        workspace_root: &str,
    ) -> CoreResult<ActiveSessionResolution> {
        // Try to get persisted active session
        if let Some(active_id) = self.get_active_session_id()? {
            if let Some(session) = self.get_session(active_id)? {
                let active_branch = self
                    .get_active_branch_id(active_id)?
                    .and_then(|bid| self.get_branch(bid).unwrap_or(None));
                return Ok(ActiveSessionResolution {
                    session,
                    active_branch,
                    was_created: false,
                });
            }
        }

        // Fall back to first session
        let sessions = self.list_sessions()?;
        if let Some(session) = sessions.first() {
            self.set_active_session_id(session.id)?;

            let active_branch = self
                .get_active_branch_id(session.id)?
                .and_then(|bid| self.get_branch(bid).unwrap_or(None));
            return Ok(ActiveSessionResolution {
                session: session.clone(),
                active_branch,
                was_created: false,
            });
        }

        // No sessions exist - create one
        let session_id = SessionId::new();
        self.create_session(session_id, workspace_root)?;
        self.set_active_session_id(session_id)?;

        let session = self
            .get_session(session_id)?
            .ok_or_else(|| CoreError::Storage("Failed to create session".to_string()))?;

        Ok(ActiveSessionResolution {
            session,
            active_branch: None,
            was_created: true,
        })
    }

    /// Resolve the active branch for a given session, creating "main" if none exists.
    ///
    /// Uses the persisted active_branch_id from app_state if available,
    /// otherwise falls back to the "main" branch.
    /// If no "main" branch exists, creates one.
    pub fn resolve_active_branch(
        &self,
        session_id: SessionId,
    ) -> CoreResult<ActiveBranchResolution> {
        // Try to get persisted active branch
        if let Some(active_id) = self.get_active_branch_id(session_id)? {
            if let Some(branch) = self.get_branch(active_id)? {
                return Ok(ActiveBranchResolution {
                    session_id,
                    branch,
                    was_created: false,
                });
            }
        }

        // Fall back to "main" branch
        let branches = self.list_branches(session_id)?;
        if let Some(branch) = branches.iter().find(|b| b.name == "main") {
            self.set_active_branch_id(session_id, branch.id)?;
            return Ok(ActiveBranchResolution {
                session_id,
                branch: branch.clone(),
                was_created: false,
            });
        }

        // No "main" branch - create one
        let branch_id = BranchId::new();
        let branch_info = BranchInfo {
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
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.create_branch(&branch_info)?;
        self.set_active_branch_id(session_id, branch_id)?;

        Ok(ActiveBranchResolution {
            session_id,
            branch: branch_info,
            was_created: true,
        })
    }

    /// Get the active session or error if none exists (no auto-create)
    pub fn get_active_session(&self) -> CoreResult<crate::sqlite_store::SessionInfo> {
        if let Some(active_id) = self.get_active_session_id()? {
            if let Some(session) = self.get_session(active_id)? {
                return Ok(session);
            }
        }

        let sessions = self.list_sessions()?;
        sessions.first().cloned().ok_or_else(|| {
            CoreError::Storage("No active session and no sessions exist".to_string())
        })
    }

    /// Get the active branch for a session, or error if none exists (no auto-create of "main")
    pub fn get_active_branch(&self, session_id: SessionId) -> CoreResult<BranchInfo> {
        if let Some(active_id) = self.get_active_branch_id(session_id)? {
            if let Some(branch) = self.get_branch(active_id)? {
                return Ok(branch);
            }
        }

        let branches = self.list_branches(session_id)?;
        branches
            .iter()
            .find(|b| b.name == "main")
            .cloned()
            .ok_or_else(|| {
                CoreError::Storage("No active branch and no main branch found".to_string())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> SqliteStore {
        SqliteStore::in_memory().unwrap()
    }

    #[test]
    fn test_resolve_active_session_creates_when_empty() {
        let store = create_test_store();
        let result = store.resolve_active_session("/tmp/test").unwrap();
        assert!(result.was_created);
        assert_eq!(result.session.workspace_root, "/tmp/test");
        assert!(result.active_branch.is_none());
    }

    #[test]
    fn test_resolve_active_session_returns_existing() {
        let store = create_test_store();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp/test").unwrap();
        store.set_active_session_id(session_id).unwrap();

        let result = store.resolve_active_session("/tmp/other").unwrap();
        assert!(!result.was_created);
        assert_eq!(result.session.id, session_id);
    }

    #[test]
    fn test_resolve_active_session_falls_back_to_first() {
        let store = create_test_store();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp/test").unwrap();
        // Don't set active_session_id

        let result = store.resolve_active_session("/tmp/other").unwrap();
        assert!(!result.was_created);
        assert_eq!(result.session.id, session_id);
    }

    #[test]
    fn test_resolve_active_branch_creates_main() {
        let store = create_test_store();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp/test").unwrap();

        let result = store.resolve_active_branch(session_id).unwrap();
        assert!(result.was_created);
        assert_eq!(result.branch.name, "main");
        assert!(result.branch.parent_branch_id.is_none());
    }

    #[test]
    fn test_resolve_active_branch_returns_main() {
        let store = create_test_store();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp/test").unwrap();

        let main_branch_id = BranchId::new();
        store
            .create_branch(&BranchInfo {
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

        let result = store.resolve_active_branch(session_id).unwrap();
        assert!(!result.was_created);
        assert_eq!(result.branch.name, "main");
    }

    #[test]
    fn test_get_active_session_error_when_empty() {
        let store = create_test_store();
        let result = store.get_active_session();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_active_branch_error_when_no_main() {
        let store = create_test_store();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp/test").unwrap();

        let result = store.get_active_branch(session_id);
        assert!(result.is_err());
    }
}
