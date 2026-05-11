//! Rollback modes for ai-history
//!
//! Implements four rollback modes:
//! - Conversation-only
//! - Workspace-only
//! - Both conversation and workspace
//! - Fork-from-checkpoint

use crate::{BranchId, CheckpointId};
use serde::{Deserialize, Serialize};

/// Rollback mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RollbackMode {
    /// Restore only conversation history
    ConversationOnly,
    /// Restore only workspace files
    WorkspaceOnly,
    /// Restore both conversation and workspace
    Both,
    /// Fork from checkpoint (keep current state)
    Fork,
}

impl RollbackMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            RollbackMode::ConversationOnly => "Chat Only",
            RollbackMode::WorkspaceOnly => "Files Only",
            RollbackMode::Both => "Both",
            RollbackMode::Fork => "Fork",
        }
    }

    pub fn restores_conversation(&self) -> bool {
        matches!(self, RollbackMode::ConversationOnly | RollbackMode::Both)
    }

    pub fn restores_workspace(&self) -> bool {
        matches!(self, RollbackMode::WorkspaceOnly | RollbackMode::Both)
    }

    pub fn creates_fork(&self) -> bool {
        matches!(self, RollbackMode::Fork)
    }
}

impl Default for RollbackMode {
    fn default() -> Self {
        Self::Both
    }
}

/// Rollback operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackOp {
    pub mode: RollbackMode,
    pub target_checkpoint_id: CheckpointId,
    pub safety_checkpoint_id: Option<CheckpointId>,
    pub current_branch_id: BranchId,
    pub new_branch_id: Option<BranchId>,
}

impl RollbackOp {
    pub fn new(
        mode: RollbackMode,
        target_checkpoint_id: CheckpointId,
        current_branch_id: BranchId,
    ) -> Self {
        Self {
            mode,
            target_checkpoint_id,
            safety_checkpoint_id: None,
            current_branch_id,
            new_branch_id: None,
        }
    }

    pub fn with_safety_checkpoint(mut self, safety_id: CheckpointId) -> Self {
        self.safety_checkpoint_id = Some(safety_id);
        self
    }

    pub fn with_new_branch(mut self, branch_id: BranchId) -> Self {
        self.new_branch_id = Some(branch_id);
        self
    }

    pub fn preserves_workspace(&self) -> bool {
        self.mode == RollbackMode::ConversationOnly || self.mode == RollbackMode::Fork
    }

    pub fn preserves_conversation(&self) -> bool {
        self.mode == RollbackMode::WorkspaceOnly || self.mode == RollbackMode::Fork
    }
}

/// Rollback result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    pub success: bool,
    pub mode: RollbackMode,
    pub new_branch_id: Option<BranchId>,
    pub turns_reverted: usize,
    pub files_restored: usize,
    pub error: Option<String>,
}

impl RollbackResult {
    pub fn success(mode: RollbackMode, new_branch_id: Option<BranchId>) -> Self {
        Self {
            success: true,
            mode,
            new_branch_id,
            turns_reverted: 0,
            files_restored: 0,
            error: None,
        }
    }

    pub fn failure(mode: RollbackMode, error: String) -> Self {
        Self {
            success: false,
            mode,
            new_branch_id: None,
            turns_reverted: 0,
            files_restored: 0,
            error: Some(error),
        }
    }

    pub fn with_turns(mut self, count: usize) -> Self {
        self.turns_reverted = count;
        self
    }

    pub fn with_files(mut self, count: usize) -> Self {
        self.files_restored = count;
        self
    }
}

/// Redo behavior after rollback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedoBehavior {
    /// Move back to existing branch head
    MoveToExistingHead,
    /// Create new branch from restored point
    CreateNewBranch,
    /// Cannot redo (no abandoned futures)
    Unavailable,
}

impl RedoBehavior {
    pub fn can_redo(&self) -> bool {
        matches!(
            self,
            RedoBehavior::MoveToExistingHead | RedoBehavior::CreateNewBranch
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rollback_mode_display() {
        assert_eq!(RollbackMode::ConversationOnly.display_name(), "Chat Only");
        assert_eq!(RollbackMode::WorkspaceOnly.display_name(), "Files Only");
        assert_eq!(RollbackMode::Both.display_name(), "Both");
        assert_eq!(RollbackMode::Fork.display_name(), "Fork");
    }

    #[test]
    fn test_rollback_mode_restores_conversation() {
        assert!(RollbackMode::ConversationOnly.restores_conversation());
        assert!(RollbackMode::Both.restores_conversation());
        assert!(!RollbackMode::WorkspaceOnly.restores_conversation());
        assert!(!RollbackMode::Fork.restores_conversation());
    }

    #[test]
    fn test_rollback_mode_restores_workspace() {
        assert!(RollbackMode::WorkspaceOnly.restores_workspace());
        assert!(RollbackMode::Both.restores_workspace());
        assert!(!RollbackMode::ConversationOnly.restores_workspace());
        assert!(!RollbackMode::Fork.restores_workspace());
    }

    #[test]
    fn test_rollback_mode_creates_fork() {
        assert!(!RollbackMode::ConversationOnly.creates_fork());
        assert!(!RollbackMode::WorkspaceOnly.creates_fork());
        assert!(!RollbackMode::Both.creates_fork());
        assert!(RollbackMode::Fork.creates_fork());
    }

    #[test]
    fn test_rollback_op_new() {
        let mode = RollbackMode::Both;
        let target = CheckpointId::new();
        let current_branch = BranchId::new();

        let op = RollbackOp::new(mode, target, current_branch);

        assert_eq!(op.mode, mode);
        assert_eq!(op.target_checkpoint_id, target);
        assert_eq!(op.current_branch_id, current_branch);
        assert!(op.safety_checkpoint_id.is_none());
        assert!(op.new_branch_id.is_none());
    }

    #[test]
    fn test_rollback_op_with_safety() {
        let op = RollbackOp::new(RollbackMode::Both, CheckpointId::new(), BranchId::new());
        let safety = CheckpointId::new();
        let op = op.with_safety_checkpoint(safety);

        assert_eq!(op.safety_checkpoint_id, Some(safety));
    }

    #[test]
    fn test_rollback_op_with_new_branch() {
        let op = RollbackOp::new(RollbackMode::Fork, CheckpointId::new(), BranchId::new());
        let new_branch = BranchId::new();
        let op = op.with_new_branch(new_branch);

        assert_eq!(op.new_branch_id, Some(new_branch));
    }

    #[test]
    fn test_rollback_op_preserves_workspace() {
        let branch = BranchId::new();
        let checkpoint = CheckpointId::new();

        let op1 = RollbackOp::new(RollbackMode::ConversationOnly, checkpoint, branch);
        assert!(op1.preserves_workspace());

        let op2 = RollbackOp::new(RollbackMode::Fork, checkpoint, branch);
        assert!(op2.preserves_workspace());

        let op3 = RollbackOp::new(RollbackMode::Both, checkpoint, branch);
        assert!(!op3.preserves_workspace());
    }

    #[test]
    fn test_rollback_result_success() {
        let result = RollbackResult::success(RollbackMode::Both, None);
        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_rollback_result_failure() {
        let result = RollbackResult::failure(RollbackMode::Both, "Test error".to_string());
        assert!(!result.success);
        assert!(result.error.is_some());
        assert_eq!(result.error.unwrap(), "Test error");
    }

    #[test]
    fn test_rollback_result_with_counts() {
        let result = RollbackResult::success(RollbackMode::Both, None)
            .with_turns(5)
            .with_files(3);

        assert_eq!(result.turns_reverted, 5);
        assert_eq!(result.files_restored, 3);
    }

    #[test]
    fn test_redo_behavior_can_redo() {
        assert!(RedoBehavior::MoveToExistingHead.can_redo());
        assert!(RedoBehavior::CreateNewBranch.can_redo());
        assert!(!RedoBehavior::Unavailable.can_redo());
    }

    #[test]
    fn test_rollback_mode_default() {
        assert_eq!(RollbackMode::default(), RollbackMode::Both);
    }
}
