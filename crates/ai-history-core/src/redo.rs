//! Redo behavior for ai-history
//!
//! After rollback, abandoned future turns must remain reachable through branch references.
//! Redo must either move back to an existing branch head or create a new branch from restored point.

use crate::{BranchId, TurnId};
use serde::{Deserialize, Serialize};

/// Redo action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedoAction {
    /// Move current branch head to an existing branch
    MoveToBranch(BranchId),
    /// Create a new branch from restored point
    CreateNewBranch,
    /// Cannot redo - no abandoned futures available
    Unavailable,
}

impl RedoAction {
    pub fn display_name(&self) -> &'static str {
        match self {
            RedoAction::MoveToBranch(_) => "Move to Branch",
            RedoAction::CreateNewBranch => "Create New Branch",
            RedoAction::Unavailable => "Redo Unavailable",
        }
    }

    pub fn is_available(&self) -> bool {
        !matches!(self, RedoAction::Unavailable)
    }
}

/// Redo context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedoContext {
    pub available_branches: Vec<BranchId>,
    pub current_branch: BranchId,
    pub restored_turn_id: TurnId,
    pub abandoned_turn_ids: Vec<TurnId>,
}

impl RedoContext {
    pub fn new(current_branch: BranchId, restored_turn_id: TurnId) -> Self {
        Self {
            available_branches: Vec::new(),
            current_branch,
            restored_turn_id,
            abandoned_turn_ids: Vec::new(),
        }
    }

    pub fn with_abandoned_turns(mut self, turns: Vec<TurnId>) -> Self {
        self.abandoned_turn_ids = turns;
        self
    }

    pub fn with_available_branch(mut self, branch_id: BranchId) -> Self {
        self.available_branches.push(branch_id);
        self
    }

    /// Get recommended redo action
    pub fn recommend_redo_action(&self) -> RedoAction {
        // If there are branches at the restored turn, offer to move to the first one
        if !self.available_branches.is_empty() {
            return RedoAction::MoveToBranch(self.available_branches[0]);
        }

        // Otherwise, offer to create a new branch
        RedoAction::CreateNewBranch
    }

    /// Check if redo is possible
    pub fn can_redo(&self) -> bool {
        !self.abandoned_turn_ids.is_empty() || !self.available_branches.is_empty()
    }

    /// Get count of available redo targets
    pub fn redo_target_count(&self) -> usize {
        self.abandoned_turn_ids.len() + self.available_branches.len()
    }
}

/// Redo operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedoOp {
    pub action: RedoAction,
    pub from_turn_id: TurnId,
    pub target_branch_id: Option<BranchId>,
    pub new_branch_name: Option<String>,
}

impl RedoOp {
    pub fn move_to_branch(from_turn_id: TurnId, branch_id: BranchId) -> Self {
        Self {
            action: RedoAction::MoveToBranch(branch_id),
            from_turn_id,
            target_branch_id: Some(branch_id),
            new_branch_name: None,
        }
    }

    pub fn create_branch(from_turn_id: TurnId, name: String) -> Self {
        Self {
            action: RedoAction::CreateNewBranch,
            from_turn_id,
            target_branch_id: None,
            new_branch_name: Some(name),
        }
    }
}

/// Redo result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedoResult {
    pub success: bool,
    pub new_branch_id: Option<BranchId>,
    pub new_head_turn_id: Option<TurnId>,
    pub error: Option<String>,
}

impl RedoResult {
    pub fn success(branch_id: BranchId, head_turn_id: TurnId) -> Self {
        Self {
            success: true,
            new_branch_id: Some(branch_id),
            new_head_turn_id: Some(head_turn_id),
            error: None,
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            new_branch_id: None,
            new_head_turn_id: None,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redo_action_display() {
        let branch_id = BranchId::new();
        assert_eq!(
            RedoAction::MoveToBranch(branch_id).display_name(),
            "Move to Branch"
        );
        assert_eq!(
            RedoAction::CreateNewBranch.display_name(),
            "Create New Branch"
        );
        assert_eq!(RedoAction::Unavailable.display_name(), "Redo Unavailable");
    }

    #[test]
    fn test_redo_action_is_available() {
        let branch_id = BranchId::new();
        assert!(RedoAction::MoveToBranch(branch_id).is_available());
        assert!(RedoAction::CreateNewBranch.is_available());
        assert!(!RedoAction::Unavailable.is_available());
    }

    #[test]
    fn test_redo_context_new() {
        let branch = BranchId::new();
        let turn = TurnId::new();
        let ctx = RedoContext::new(branch, turn);

        assert_eq!(ctx.current_branch, branch);
        assert_eq!(ctx.restored_turn_id, turn);
        assert!(ctx.abandoned_turn_ids.is_empty());
        assert!(ctx.available_branches.is_empty());
    }

    #[test]
    fn test_redo_context_with_abandoned_turns() {
        let branch = BranchId::new();
        let turn = TurnId::new();
        let abandoned = vec![TurnId::new(), TurnId::new()];

        let ctx = RedoContext::new(branch, turn).with_abandoned_turns(abandoned.clone());

        assert_eq!(ctx.abandoned_turn_ids, abandoned);
    }

    #[test]
    fn test_redo_context_recommend_move_to_branch() {
        let branch = BranchId::new();
        let turn = TurnId::new();
        let available = BranchId::new();

        let ctx = RedoContext::new(branch, turn).with_available_branch(available);

        match ctx.recommend_redo_action() {
            RedoAction::MoveToBranch(b) => assert_eq!(b, available),
            _ => panic!("Expected MoveToBranch"),
        }
    }

    #[test]
    fn test_redo_context_recommend_create_branch() {
        let branch = BranchId::new();
        let turn = TurnId::new();

        let ctx = RedoContext::new(branch, turn);

        assert_eq!(ctx.recommend_redo_action(), RedoAction::CreateNewBranch);
    }

    #[test]
    fn test_redo_context_can_redo() {
        let branch = BranchId::new();
        let turn = TurnId::new();

        // No abandoned turns or branches
        let ctx1 = RedoContext::new(branch, turn);
        assert!(!ctx1.can_redo());

        // Has abandoned turns
        let ctx2 = RedoContext::new(branch, turn).with_abandoned_turns(vec![TurnId::new()]);
        assert!(ctx2.can_redo());

        // Has available branch
        let ctx3 = RedoContext::new(branch, turn).with_available_branch(BranchId::new());
        assert!(ctx3.can_redo());
    }

    #[test]
    fn test_redo_context_redo_target_count() {
        let branch = BranchId::new();
        let turn = TurnId::new();

        let ctx = RedoContext::new(branch, turn)
            .with_abandoned_turns(vec![TurnId::new(), TurnId::new()])
            .with_available_branch(BranchId::new());

        assert_eq!(ctx.redo_target_count(), 3);
    }

    #[test]
    fn test_redo_op_move_to_branch() {
        let turn = TurnId::new();
        let branch = BranchId::new();

        let op = RedoOp::move_to_branch(turn, branch);

        assert!(matches!(op.action, RedoAction::MoveToBranch(b) if b == branch));
        assert_eq!(op.from_turn_id, turn);
        assert_eq!(op.target_branch_id, Some(branch));
        assert!(op.new_branch_name.is_none());
    }

    #[test]
    fn test_redo_op_create_branch() {
        let turn = TurnId::new();

        let op = RedoOp::create_branch(turn, "feature-redo".to_string());

        assert!(matches!(op.action, RedoAction::CreateNewBranch));
        assert_eq!(op.from_turn_id, turn);
        assert!(op.target_branch_id.is_none());
        assert_eq!(op.new_branch_name, Some("feature-redo".to_string()));
    }

    #[test]
    fn test_redo_result_success() {
        let branch = BranchId::new();
        let turn = TurnId::new();
        let result = RedoResult::success(branch, turn);

        assert!(result.success);
        assert_eq!(result.new_branch_id, Some(branch));
        assert_eq!(result.new_head_turn_id, Some(turn));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_redo_result_failure() {
        let result = RedoResult::failure("No redo targets available".to_string());

        assert!(!result.success);
        assert!(result.new_branch_id.is_none());
        assert!(result.error.is_some());
        assert_eq!(result.error.unwrap(), "No redo targets available");
    }
}
