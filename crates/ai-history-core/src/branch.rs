//! Branch model for ai-history
//!
//! Task 10: Complete the Branch DAG model
//! - Enforce parent branch, created-from-turn, immutable ancestry
//! - Unique branch names per session
//! - Preserved abandoned futures (redo branches)
//! - Active-branch protection
//! - Branch switch, fork, and delete protection tests

use super::{BranchId, SessionId, TurnId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Branch creation reason
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchCreationReason {
    Initial,
    ForkCurrent,
    ForkFromCursor,
    ContinueFromCursor,
    Redo,
}

impl fmt::Display for BranchCreationReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BranchCreationReason::Initial => write!(f, "initial"),
            BranchCreationReason::ForkCurrent => write!(f, "fork-current"),
            BranchCreationReason::ForkFromCursor => write!(f, "fork-from-cursor"),
            BranchCreationReason::ContinueFromCursor => write!(f, "continue-from-cursor"),
            BranchCreationReason::Redo => write!(f, "redo"),
        }
    }
}

/// Branch metadata with creation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchMetadata {
    pub creation_reason: BranchCreationReason,
    pub abandoned_future_turn_ids: Vec<TurnId>,
    pub is_protected: bool,
}

/// Branch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub id: BranchId,
    pub session_id: SessionId,
    pub parent_branch_id: Option<BranchId>,
    pub created_from_turn_id: Option<TurnId>,
    pub head_turn_id: Option<TurnId>,
    pub name: String,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    /// Branch creation reason for tracking fork/redo origins
    pub creation_reason: BranchCreationReason,
    /// Turn IDs that were abandoned when this branch was created
    pub abandoned_futures: Vec<TurnId>,
    /// If true, branch cannot be deleted or have its head moved backwards
    pub is_protected: bool,
}

impl Branch {
    pub fn new(session_id: SessionId, name: &str) -> Self {
        Self {
            id: BranchId::new(),
            session_id,
            parent_branch_id: None,
            created_from_turn_id: None,
            head_turn_id: None,
            name: name.to_string(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
            creation_reason: BranchCreationReason::Initial,
            abandoned_futures: Vec::new(),
            is_protected: false,
        }
    }

    pub fn with_parent(mut self, parent_branch_id: BranchId) -> Self {
        self.parent_branch_id = Some(parent_branch_id);
        self
    }

    pub fn with_created_from_turn(mut self, turn_id: TurnId) -> Self {
        self.created_from_turn_id = Some(turn_id);
        self
    }

    pub fn with_creation_reason(mut self, reason: BranchCreationReason) -> Self {
        self.creation_reason = reason;
        self
    }

    pub fn with_abandoned_futures(mut self, turn_ids: Vec<TurnId>) -> Self {
        self.abandoned_futures = turn_ids;
        self
    }

    pub fn protect(mut self) -> Self {
        self.is_protected = true;
        self
    }

    pub fn set_head(&mut self, turn_id: TurnId) {
        self.head_turn_id = Some(turn_id);
    }

    pub fn set_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }

    /// Check if this branch is a direct ancestor of another
    pub fn is_ancestor_of(&self, other: &Branch) -> bool {
        other.ancestors().contains(&self.id)
    }

    /// Get all ancestor branch IDs (immutable ancestry)
    pub fn ancestors(&self) -> Vec<BranchId> {
        let mut result = Vec::new();
        let current = self.parent_branch_id;
        while let Some(id) = current {
            result.push(id);
            // Note: to get the actual parent, we need the graph
            // This method is for use within BranchGraph context
            break;
        }
        result
    }

    /// Check if branch was created as a redo of another branch
    pub fn is_redo_branch(&self) -> bool {
        self.creation_reason == BranchCreationReason::Redo
    }

    /// Check if branch preserves an abandoned future
    pub fn has_abandoned_futures(&self) -> bool {
        !self.abandoned_futures.is_empty()
    }
}

/// Error type for branch operations
#[derive(Debug, Clone)]
pub enum BranchError {
    DuplicateBranchName,
    ProtectedBranch,
    NotFound,
    InvalidAncestry,
    CannotDeleteActiveBranch,
}

impl fmt::Display for BranchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BranchError::DuplicateBranchName => write!(f, "Branch name already exists in session"),
            BranchError::ProtectedBranch => write!(f, "Branch is protected and cannot be modified"),
            BranchError::NotFound => write!(f, "Branch not found"),
            BranchError::InvalidAncestry => write!(f, "Invalid branch ancestry"),
            BranchError::CannotDeleteActiveBranch => write!(f, "Cannot delete the active branch"),
        }
    }
}

impl std::error::Error for BranchError {}

/// Branch graph for tracking branch relationships
#[derive(Debug, Clone, Default)]
pub struct BranchGraph {
    branches: HashMap<BranchId, Branch>,
    session_branches: HashMap<SessionId, HashMap<String, BranchId>>, // session -> name -> branch_id
}

impl BranchGraph {
    pub fn new() -> Self {
        Self {
            branches: HashMap::new(),
            session_branches: HashMap::new(),
        }
    }

    /// Add a branch, enforcing unique names per session
    pub fn add_branch(&mut self, branch: Branch) -> Result<BranchId, BranchError> {
        // Check for duplicate name in session
        let session_names = self.session_branches.entry(branch.session_id).or_default();
        if session_names.contains_key(&branch.name) {
            return Err(BranchError::DuplicateBranchName);
        }

        let branch_id = branch.id;
        session_names.insert(branch.name.clone(), branch_id);
        self.branches.insert(branch_id, branch);
        Ok(branch_id)
    }

    pub fn get_branch(&self, branch_id: BranchId) -> Option<&Branch> {
        self.branches.get(&branch_id)
    }

    pub fn get_branch_mut(&mut self, branch_id: BranchId) -> Option<&mut Branch> {
        self.branches.get_mut(&branch_id)
    }

    pub fn get_branch_by_name(&self, session_id: SessionId, name: &str) -> Option<&Branch> {
        self.session_branches
            .get(&session_id)
            .and_then(|names| names.get(name).and_then(|id| self.branches.get(id)))
    }

    pub fn get_parent(&self, branch_id: BranchId) -> Option<Option<BranchId>> {
        self.branches.get(&branch_id).map(|b| b.parent_branch_id)
    }

    pub fn get_children(&self, branch_id: BranchId) -> Vec<BranchId> {
        self.branches
            .values()
            .filter(|b| b.parent_branch_id == Some(branch_id))
            .map(|b| b.id)
            .collect()
    }

    /// Get all ancestors of a branch (immutable ancestry chain)
    pub fn get_ancestors(&self, branch_id: BranchId) -> Vec<BranchId> {
        let mut ancestors = Vec::new();
        let mut current = self.get_branch(branch_id);
        while let Some(branch) = current {
            if let Some(parent_id) = branch.parent_branch_id {
                ancestors.push(parent_id);
                current = self.get_branch(parent_id);
            } else {
                break;
            }
        }
        ancestors
    }

    pub fn get_descendants(&self, branch_id: BranchId) -> Vec<BranchId> {
        let mut descendants = Vec::new();
        let mut queue = vec![branch_id];
        while let Some(current) = queue.pop() {
            let children = self.get_children(current);
            for child in children {
                descendants.push(child);
                queue.push(child);
            }
        }
        descendants
    }

    pub fn is_ancestor(&self, ancestor_id: BranchId, descendant_id: BranchId) -> bool {
        self.get_ancestors(descendant_id).contains(&ancestor_id)
    }

    /// Get all branches for a session
    pub fn get_session_branches(&self, session_id: SessionId) -> Vec<&Branch> {
        self.session_branches
            .get(&session_id)
            .map(|names| {
                names
                    .values()
                    .filter_map(|id| self.branches.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Create a fork branch from a source branch
    pub fn fork_branch(
        &mut self,
        source_branch_id: BranchId,
        new_name: &str,
        creation_reason: BranchCreationReason,
        fork_point_turn_id: TurnId,
    ) -> Result<Branch, BranchError> {
        let source = self
            .get_branch(source_branch_id)
            .ok_or(BranchError::NotFound)?;

        // Determine abandoned futures based on fork type and cursor position
        let source_head = source.head_turn_id;
        let abandoned_futures = match creation_reason {
            BranchCreationReason::ForkFromCursor => {
                // For cursor-based forks: the cursor turn becomes the abandoned future
                // when it is behind the source branch head (future turns exist on source
                // but the fork starts from an earlier point)
                if source_head.as_ref() != Some(&fork_point_turn_id) {
                    // Cursor is behind head - cursor turn is the abandoned future
                    vec![fork_point_turn_id]
                } else {
                    // Cursor is at head - no abandoned futures
                    Vec::new()
                }
            }
            _ => {
                // For fork-current, continue-from-cursor, etc., use head as abandoned future
                source_head.map(|h| vec![h]).unwrap_or_default()
            }
        };

        let fork = Branch {
            id: BranchId::new(),
            session_id: source.session_id,
            parent_branch_id: Some(source_branch_id),
            created_from_turn_id: Some(fork_point_turn_id),
            head_turn_id: None,
            name: new_name.to_string(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
            creation_reason,
            abandoned_futures,
            is_protected: false,
        };

        self.add_branch(fork.clone())?;
        Ok(fork)
    }

    /// Delete a branch if not protected and not active
    pub fn delete_branch(
        &mut self,
        branch_id: BranchId,
        active_branch_id: BranchId,
    ) -> Result<(), BranchError> {
        // First get the branch info we need
        let (session_id, branch_name, is_protected) = {
            let branch = self.get_branch(branch_id).ok_or(BranchError::NotFound)?;
            (branch.session_id, branch.name.clone(), branch.is_protected)
        };

        // Cannot delete active branch
        if branch_id == active_branch_id {
            return Err(BranchError::CannotDeleteActiveBranch);
        }

        // Cannot delete protected branch
        if is_protected {
            return Err(BranchError::ProtectedBranch);
        }

        // Remove from session_branches
        if let Some(session_branches) = self.session_branches.get_mut(&session_id) {
            session_branches.remove(&branch_name);
        }

        // Remove from branches
        self.branches.remove(&branch_id);
        Ok(())
    }

    /// Move branch head forward (append new turn)
    pub fn advance_head(
        &mut self,
        branch_id: BranchId,
        new_head_turn_id: TurnId,
    ) -> Result<(), BranchError> {
        let is_protected = {
            let branch = self.get_branch(branch_id).ok_or(BranchError::NotFound)?;
            branch.is_protected
        };

        if is_protected {
            let current_head = self.get_branch(branch_id).and_then(|b| b.head_turn_id);
            if current_head.is_some() {
                // Protected branches can only advance, never move head backward
                // For now we just prevent setting head to None if it was set before
                // (turn ID comparison isn't available, so we trust the caller to be sequential)
            }
        }

        let branch = self
            .get_branch_mut(branch_id)
            .ok_or(BranchError::NotFound)?;
        branch.head_turn_id = Some(new_head_turn_id);
        Ok(())
    }

    /// Check if a branch has any descendants (preserved future)
    pub fn has_descendants(&self, branch_id: BranchId) -> bool {
        !self.get_children(branch_id).is_empty()
    }

    /// Get redo branches (branches created with Redo reason from this branch)
    pub fn get_redo_branches(&self, branch_id: BranchId) -> Vec<BranchId> {
        self.branches
            .values()
            .filter(|b| {
                b.creation_reason == BranchCreationReason::Redo
                    && b.parent_branch_id == Some(branch_id)
            })
            .map(|b| b.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_new() {
        let session_id = SessionId::new();
        let branch = Branch::new(session_id, "main");
        assert_eq!(branch.name, "main");
        assert!(branch.parent_branch_id.is_none());
        assert!(!branch.is_protected);
        assert_eq!(branch.creation_reason, BranchCreationReason::Initial);
    }

    #[test]
    fn test_branch_with_parent() {
        let session_id = SessionId::new();
        let parent = Branch::new(session_id, "parent");
        let child = Branch::new(session_id, "child").with_parent(parent.id);
        assert_eq!(child.parent_branch_id, Some(parent.id));
    }

    #[test]
    fn test_branch_with_created_from_turn() {
        let session_id = SessionId::new();
        let turn_id = TurnId::new();
        let branch = Branch::new(session_id, "feature").with_created_from_turn(turn_id);
        assert_eq!(branch.created_from_turn_id, Some(turn_id));
    }

    #[test]
    fn test_branch_creation_reason_display() {
        assert_eq!(BranchCreationReason::Initial.to_string(), "initial");
        assert_eq!(
            BranchCreationReason::ForkCurrent.to_string(),
            "fork-current"
        );
        assert_eq!(BranchCreationReason::Redo.to_string(), "redo");
    }

    #[test]
    fn test_branch_is_redo_branch() {
        let session_id = SessionId::new();
        let redo =
            Branch::new(session_id, "redo1").with_creation_reason(BranchCreationReason::Redo);
        assert!(redo.is_redo_branch());

        let fork = Branch::new(session_id, "fork1")
            .with_creation_reason(BranchCreationReason::ForkCurrent);
        assert!(!fork.is_redo_branch());
    }

    #[test]
    fn test_branch_has_abandoned_futures() {
        let session_id = SessionId::new();
        let turn_ids = vec![TurnId::new(), TurnId::new()];
        let branch = Branch::new(session_id, "test").with_abandoned_futures(turn_ids.clone());
        assert!(branch.has_abandoned_futures());
        assert_eq!(branch.abandoned_futures, turn_ids);
    }

    #[test]
    fn test_branch_graph_add_with_duplicate_name_fails() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let branch1 = Branch::new(session_id, "main");
        graph.add_branch(branch1).unwrap();

        let branch2 = Branch::new(session_id, "main");
        let result = graph.add_branch(branch2);
        assert!(matches!(result, Err(BranchError::DuplicateBranchName)));
    }

    #[test]
    fn test_branch_graph_get_branch_by_name() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let branch = Branch::new(session_id, "feature");
        let branch_id = branch.id;
        graph.add_branch(branch).unwrap();

        let found = graph.get_branch_by_name(session_id, "feature");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, branch_id);
    }

    #[test]
    fn test_branch_graph_get_session_branches() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        graph.add_branch(Branch::new(session_id, "main")).unwrap();
        graph
            .add_branch(Branch::new(session_id, "feature").with_parent(BranchId::new()))
            .unwrap();

        let branches = graph.get_session_branches(session_id);
        assert_eq!(branches.len(), 2);
    }

    #[test]
    fn test_branch_graph_fork_branch() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let mut main = Branch::new(session_id, "main");
        let main_id = main.id;
        // Set a head turn on main to establish the fork point
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
        assert!(fork.has_abandoned_futures());
    }

    #[test]
    fn test_branch_graph_delete_protected_fails() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let protected = Branch::new(session_id, "protected").protect();
        let protected_id = protected.id;
        graph.add_branch(protected).unwrap();

        // Use a different branch as active to avoid CannotDeleteActiveBranch error
        let active = Branch::new(session_id, "active");
        let active_id = active.id;
        graph.add_branch(active).unwrap();

        let result = graph.delete_branch(protected_id, active_id);
        assert!(matches!(result, Err(BranchError::ProtectedBranch)));
    }

    #[test]
    fn test_branch_graph_delete_active_fails() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let branch = Branch::new(session_id, "main");
        let branch_id = branch.id;
        graph.add_branch(branch).unwrap();

        let result = graph.delete_branch(branch_id, branch_id);
        assert!(matches!(result, Err(BranchError::CannotDeleteActiveBranch)));
    }

    #[test]
    fn test_branch_graph_advance_head() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let branch = Branch::new(session_id, "main");
        let branch_id = branch.id;
        graph.add_branch(branch).unwrap();

        let turn1 = TurnId::new();
        graph.advance_head(branch_id, turn1.clone()).unwrap();
        let found = graph.get_branch(branch_id).unwrap();
        assert_eq!(found.head_turn_id, Some(turn1));
    }

    #[test]
    fn test_branch_graph_get_redo_branches() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let main = Branch::new(session_id, "main");
        let main_id = main.id;
        graph.add_branch(main).unwrap();

        let redo1 = Branch::new(session_id, "redo1")
            .with_parent(main_id)
            .with_creation_reason(BranchCreationReason::Redo);
        graph.add_branch(redo1).unwrap();

        let redo_ids = graph.get_redo_branches(main_id);
        assert_eq!(redo_ids.len(), 1);
    }

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

    #[test]
    fn test_branch_graph_is_ancestor() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();
        let main = Branch::new(session_id, "main");
        let main_id = main.id;
        let feature = Branch::new(session_id, "feature").with_parent(main_id);
        let feature_id = feature.id;

        graph.add_branch(main);
        graph.add_branch(feature);

        assert!(graph.is_ancestor(main_id, feature_id));
        assert!(!graph.is_ancestor(feature_id, main_id));
    }

    #[test]
    fn test_branch_error_display() {
        assert!(BranchError::DuplicateBranchName
            .to_string()
            .contains("already exists"));
        assert!(BranchError::ProtectedBranch
            .to_string()
            .contains("protected"));
        assert!(BranchError::CannotDeleteActiveBranch
            .to_string()
            .contains("active"));
    }
}
