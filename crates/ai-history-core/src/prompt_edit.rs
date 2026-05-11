//! Prompt editing behavior for ai-history
//!
//! Editing an earlier user prompt creates a new branch from that prompt's parent turn.
//! The edited prompt becomes a new message and turn.

use crate::{BranchId, TurnId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A prompt edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptEdit {
    pub original_turn_id: TurnId,
    pub new_content: String,
    pub edited_at: DateTime<Utc>,
    pub new_turn_id: TurnId,
    pub new_branch_id: BranchId,
}

impl PromptEdit {
    pub fn new(
        original_turn_id: TurnId,
        new_content: String,
        new_turn_id: TurnId,
        new_branch_id: BranchId,
    ) -> Self {
        Self {
            original_turn_id,
            new_content,
            edited_at: Utc::now(),
            new_turn_id,
            new_branch_id,
        }
    }

    /// Get the parent turn ID (the turn before the edited prompt)
    pub fn parent_turn_id(&self) -> TurnId {
        self.original_turn_id
    }
}

/// Prompt edit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptEditResult {
    pub success: bool,
    pub new_branch_id: BranchId,
    pub new_turn_id: TurnId,
    pub original_turn_id: TurnId,
    pub error: Option<String>,
}

impl PromptEditResult {
    pub fn success(original: TurnId, branch: BranchId, turn: TurnId) -> Self {
        Self {
            success: true,
            new_branch_id: branch,
            new_turn_id: turn,
            original_turn_id: original,
            error: None,
        }
    }

    pub fn failure(original: TurnId, error: String) -> Self {
        Self {
            success: false,
            new_branch_id: BranchId::new(),
            new_turn_id: TurnId::new(),
            original_turn_id: original,
            error: Some(error),
        }
    }
}

/// Rules for prompt editing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptEditRule {
    /// Can edit any user prompt
    EditAny,
    /// Can only edit the most recent prompt
    EditRecentOnly,
    /// Cannot edit prompts (read-only history)
    NoEdits,
}

impl Default for PromptEditRule {
    fn default() -> Self {
        Self::EditRecentOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_edit_new() {
        let original = TurnId::new();
        let new_turn = TurnId::new();
        let new_branch = BranchId::new();

        let edit = PromptEdit::new(
            original,
            "Updated prompt content".to_string(),
            new_turn,
            new_branch,
        );

        assert_eq!(edit.original_turn_id, original);
        assert_eq!(edit.new_content, "Updated prompt content");
        assert_eq!(edit.new_turn_id, new_turn);
        assert_eq!(edit.new_branch_id, new_branch);
        assert!(edit.edited_at <= chrono::Utc::now());
    }

    #[test]
    fn test_prompt_edit_parent_turn_id() {
        let original = TurnId::new();
        let edit = PromptEdit::new(original, "test".to_string(), TurnId::new(), BranchId::new());

        assert_eq!(edit.parent_turn_id(), original);
    }

    #[test]
    fn test_prompt_edit_result_success() {
        let original = TurnId::new();
        let branch = BranchId::new();
        let turn = TurnId::new();

        let result = PromptEditResult::success(original, branch, turn);

        assert!(result.success);
        assert_eq!(result.original_turn_id, original);
        assert_eq!(result.new_branch_id, branch);
        assert_eq!(result.new_turn_id, turn);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_prompt_edit_result_failure() {
        let original = TurnId::new();
        let result = PromptEditResult::failure(original, "Cannot edit turn".to_string());

        assert!(!result.success);
        assert_eq!(result.original_turn_id, original);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_prompt_edit_rule_default() {
        assert_eq!(PromptEditRule::default(), PromptEditRule::EditRecentOnly);
    }

    #[test]
    fn test_prompt_edit_rule_display() {
        assert_eq!(format!("{:?}", PromptEditRule::EditAny), "EditAny");
        assert_eq!(
            format!("{:?}", PromptEditRule::EditRecentOnly),
            "EditRecentOnly"
        );
        assert_eq!(format!("{:?}", PromptEditRule::NoEdits), "NoEdits");
    }
}
