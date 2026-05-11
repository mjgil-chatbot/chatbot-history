//! Rollback and history product scope definition
//!
//! Specifies exactly which user actions are reversible and which are not.
//! This is the foundational specification for the rollback surface.

use serde::{Deserialize, Serialize};

/// Category of user action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionCategory {
    /// Actions that create new history entries
    Create,
    /// Actions that modify existing entries
    Modify,
    /// Actions that delete entries
    Delete,
    /// Actions that restore previous state
    Restore,
    /// Actions that branch/fork history
    Branch,
}

/// Reversibility level for an action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reversibility {
    /// Fully reversible - can undo/redo
    FullyReversible,
    /// Reversible with side effects - creates branch
    ReversibleWithBranch,
    /// Requires explicit checkpoint
    RequiresCheckpoint,
    /// Not directly reversible
    NotReversible,
    /// Action is itself a rollback
    IsRollback,
}

impl Reversibility {
    /// Check if this action can be undone
    pub fn can_undo(&self) -> bool {
        matches!(
            self,
            Self::FullyReversible | Self::ReversibleWithBranch | Self::RequiresCheckpoint
        )
    }

    /// Check if this action can be redone
    pub fn can_redo(&self) -> bool {
        matches!(
            self,
            Self::FullyReversible | Self::ReversibleWithBranch | Self::IsRollback
        )
    }
}

/// A defined action in the rollback surface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedAction {
    pub name: &'static str,
    pub category: ActionCategory,
    pub reversibility: Reversibility,
    pub requires_checkpoint: bool,
    pub creates_branch: bool,
    pub description: &'static str,
}

/// Rollback surface specification
#[derive(Debug, Clone, Default)]
pub struct RollbackSurface {
    pub reversible_actions: Vec<ScopedAction>,
    pub irreversible_actions: Vec<ScopedAction>,
    pub unsupported_operations: Vec<&'static str>,
}

impl RollbackSurface {
    /// Create a new rollback surface with all defined actions
    pub fn full() -> Self {
        Self {
            reversible_actions: Self::reversible_actions_list(),
            irreversible_actions: Self::irreversible_actions_list(),
            unsupported_operations: Self::unsupported_list(),
        }
    }

    fn reversible_actions_list() -> Vec<ScopedAction> {
        vec![
            // Chat turns
            ScopedAction {
                name: "send_prompt",
                category: ActionCategory::Create,
                reversibility: Reversibility::ReversibleWithBranch,
                requires_checkpoint: false,
                creates_branch: true,
                description: "Sending a user prompt creates a new turn, reversible by rollback",
            },
            ScopedAction {
                name: "stream_assistant_response",
                category: ActionCategory::Create,
                reversibility: Reversibility::FullyReversible,
                requires_checkpoint: false,
                creates_branch: false,
                description: "Assistant streaming responses are reversible",
            },
            ScopedAction {
                name: "edit_previous_prompt",
                category: ActionCategory::Modify,
                reversibility: Reversibility::ReversibleWithBranch,
                requires_checkpoint: false,
                creates_branch: true,
                description: "Editing a previous prompt creates a branch, original preserved",
            },
            // File operations
            ScopedAction {
                name: "tool_call_file_write",
                category: ActionCategory::Modify,
                reversibility: Reversibility::RequiresCheckpoint,
                requires_checkpoint: true,
                creates_branch: false,
                description: "File writes require pre-write checkpoint for safe rollback",
            },
            ScopedAction {
                name: "tool_call_file_delete",
                category: ActionCategory::Delete,
                reversibility: Reversibility::RequiresCheckpoint,
                requires_checkpoint: true,
                creates_branch: false,
                description: "File deletes require pre-delete checkpoint",
            },
            ScopedAction {
                name: "tool_call_file_rename",
                category: ActionCategory::Modify,
                reversibility: Reversibility::RequiresCheckpoint,
                requires_checkpoint: true,
                creates_branch: false,
                description: "File renames require pre-rename checkpoint",
            },
            // Checkpoint operations
            ScopedAction {
                name: "create_named_checkpoint",
                category: ActionCategory::Create,
                reversibility: Reversibility::IsRollback,
                requires_checkpoint: false,
                creates_branch: false,
                description: "Creating a checkpoint is itself an action that can be restored to",
            },
            ScopedAction {
                name: "create_safety_checkpoint",
                category: ActionCategory::Create,
                reversibility: Reversibility::IsRollback,
                requires_checkpoint: false,
                creates_branch: false,
                description: "Safety checkpoints are automatic before destructive actions",
            },
            // Branch operations
            ScopedAction {
                name: "create_branch",
                category: ActionCategory::Branch,
                reversibility: Reversibility::IsRollback,
                requires_checkpoint: false,
                creates_branch: false,
                description: "Branches are immutable history references",
            },
            ScopedAction {
                name: "fork_from_checkpoint",
                category: ActionCategory::Branch,
                reversibility: Reversibility::IsRollback,
                requires_checkpoint: false,
                creates_branch: true,
                description: "Forking creates a new branch from checkpoint",
            },
            ScopedAction {
                name: "switch_branch",
                category: ActionCategory::Modify,
                reversibility: Reversibility::FullyReversible,
                requires_checkpoint: false,
                creates_branch: false,
                description: "Switching branches is fully reversible",
            },
            // Restore operations
            ScopedAction {
                name: "restore_workspace",
                category: ActionCategory::Restore,
                reversibility: Reversibility::IsRollback,
                requires_checkpoint: false,
                creates_branch: false,
                description: "Workspace restore is itself a rollback action",
            },
            ScopedAction {
                name: "restore_conversation",
                category: ActionCategory::Restore,
                reversibility: Reversibility::IsRollback,
                requires_checkpoint: false,
                creates_branch: false,
                description: "Conversation restore is itself a rollback action",
            },
            ScopedAction {
                name: "restore_both",
                category: ActionCategory::Restore,
                reversibility: Reversibility::IsRollback,
                requires_checkpoint: false,
                creates_branch: false,
                description: "Full restore is itself a rollback action",
            },
            // Redo operations
            ScopedAction {
                name: "redo_on_abandoned_branch",
                category: ActionCategory::Create,
                reversibility: Reversibility::IsRollback,
                requires_checkpoint: false,
                creates_branch: false,
                description: "Redo on abandoned branch creates new turn",
            },
        ]
    }

    fn irreversible_actions_list() -> Vec<ScopedAction> {
        vec![
            ScopedAction {
                name: "delete_named_checkpoint",
                category: ActionCategory::Delete,
                reversibility: Reversibility::NotReversible,
                requires_checkpoint: false,
                creates_branch: false,
                description: "Deleting a named checkpoint is permanent",
            },
            ScopedAction {
                name: "delete_session",
                category: ActionCategory::Delete,
                reversibility: Reversibility::NotReversible,
                requires_checkpoint: false,
                creates_branch: false,
                description: "Session deletion is permanent",
            },
            ScopedAction {
                name: "prune_blob",
                category: ActionCategory::Delete,
                reversibility: Reversibility::NotReversible,
                requires_checkpoint: false,
                creates_branch: false,
                description: "Blob pruning after GC is permanent",
            },
        ]
    }

    fn unsupported_list() -> Vec<&'static str> {
        vec![
            "Real-time collaboration on same session",
            "Concurrent writes to same file",
            "External file modifications during restore",
            "Network sync during offline operation",
            "Rollback across session boundaries",
            "Undo of undo across branch switch",
            "Restore to future state",
            "Modification of finalized assistant messages",
        ]
    }

    /// Check if an action is reversible
    pub fn is_reversible(&self, action_name: &str) -> Option<Reversibility> {
        self.reversible_actions
            .iter()
            .find(|a| a.name == action_name)
            .map(|a| a.reversibility)
    }

    /// Check if an action requires checkpoint
    pub fn requires_checkpoint(&self, action_name: &str) -> bool {
        self.reversible_actions
            .iter()
            .find(|a| a.name == action_name)
            .map(|a| a.requires_checkpoint)
            .unwrap_or(false)
    }

    /// Get all actions that require checkpoints
    pub fn checkpoint_required_actions(&self) -> Vec<&ScopedAction> {
        self.reversible_actions
            .iter()
            .filter(|a| a.requires_checkpoint)
            .collect()
    }

    /// Get all branch-creating actions
    pub fn branch_creating_actions(&self) -> Vec<&ScopedAction> {
        self.reversible_actions
            .iter()
            .filter(|a| a.creates_branch)
            .collect()
    }
}

/// Product scope validator
pub struct RollbackScopeValidator;

impl RollbackScopeValidator {
    /// Validate that an action is within the defined scope
    pub fn validate_action(action_name: &str, surface: &RollbackSurface) -> ValidationResult {
        // Check reversible actions
        if surface
            .reversible_actions
            .iter()
            .any(|a| a.name == action_name)
        {
            return ValidationResult::Valid(ActionCategory::Modify);
        }

        // Check irreversible actions
        if surface
            .irreversible_actions
            .iter()
            .any(|a| a.name == action_name)
        {
            return ValidationResult::Invalid(ActionCategory::Delete);
        }

        // Check unsupported
        if surface
            .unsupported_operations
            .iter()
            .any(|u| *u == action_name)
        {
            return ValidationResult::Unsupported;
        }

        ValidationResult::Unknown
    }

    /// Check if an action can be performed given current state
    pub fn can_perform(
        action_name: &str,
        surface: &RollbackSurface,
        has_checkpoint: bool,
    ) -> Result<(), UnsupportedActionError> {
        if surface
            .unsupported_operations
            .iter()
            .any(|u| *u == action_name)
        {
            return Err(UnsupportedActionError {
                action: action_name.to_string(),
                reason: "Action is outside the defined product scope".to_string(),
            });
        }

        if surface.requires_checkpoint(action_name) && !has_checkpoint {
            return Err(UnsupportedActionError {
                action: action_name.to_string(),
                reason: "Action requires a checkpoint before it can be performed".to_string(),
            });
        }

        Ok(())
    }
}

/// Result of action validation
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Valid(ActionCategory),
    Invalid(ActionCategory),
    Unsupported,
    Unknown,
}

/// Error for unsupported actions
#[derive(Debug, Clone)]
pub struct UnsupportedActionError {
    pub action: String,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reversibility_can_undo() {
        assert!(Reversibility::FullyReversible.can_undo());
        assert!(Reversibility::ReversibleWithBranch.can_undo());
        assert!(Reversibility::RequiresCheckpoint.can_undo());
        assert!(!Reversibility::NotReversible.can_undo());
        assert!(!Reversibility::IsRollback.can_undo());
    }

    #[test]
    fn test_reversibility_can_redo() {
        assert!(Reversibility::FullyReversible.can_redo());
        assert!(Reversibility::ReversibleWithBranch.can_redo());
        assert!(Reversibility::IsRollback.can_redo());
        assert!(!Reversibility::RequiresCheckpoint.can_redo());
        assert!(!Reversibility::NotReversible.can_redo());
    }

    #[test]
    fn test_rollback_surface_full() {
        let surface = RollbackSurface::full();
        assert!(!surface.reversible_actions.is_empty());
        assert!(!surface.irreversible_actions.is_empty());
        assert!(!surface.unsupported_operations.is_empty());
    }

    #[test]
    fn test_rollback_surface_is_reversible() {
        let surface = RollbackSurface::full();
        assert_eq!(
            surface.is_reversible("send_prompt"),
            Some(Reversibility::ReversibleWithBranch)
        );
        assert_eq!(
            surface.is_reversible("tool_call_file_write"),
            Some(Reversibility::RequiresCheckpoint)
        );
        assert_eq!(surface.is_reversible("delete_session"), None);
    }

    #[test]
    fn test_rollback_surface_requires_checkpoint() {
        let surface = RollbackSurface::full();
        assert!(surface.requires_checkpoint("tool_call_file_write"));
        assert!(!surface.requires_checkpoint("stream_assistant_response"));
    }

    #[test]
    fn test_checkpoint_required_actions() {
        let surface = RollbackSurface::full();
        let checkpoint_actions = surface.checkpoint_required_actions();
        assert!(!checkpoint_actions.is_empty());
        assert!(checkpoint_actions.iter().all(|a| a.requires_checkpoint));
    }

    #[test]
    fn test_branch_creating_actions() {
        let surface = RollbackSurface::full();
        let branch_actions = surface.branch_creating_actions();
        assert!(!branch_actions.is_empty());
        assert!(branch_actions.iter().any(|a| a.name == "send_prompt"));
        assert!(branch_actions
            .iter()
            .any(|a| a.name == "edit_previous_prompt"));
    }

    #[test]
    fn test_scope_validator_valid_action() {
        let surface = RollbackSurface::full();
        let result = RollbackScopeValidator::validate_action("send_prompt", &surface);
        assert!(matches!(result, ValidationResult::Valid(_)));
    }

    #[test]
    fn test_scope_validator_unsupported() {
        let surface = RollbackSurface::full();
        let result = RollbackScopeValidator::validate_action(
            "Real-time collaboration on same session",
            &surface,
        );
        assert!(matches!(result, ValidationResult::Unsupported));
    }

    #[test]
    fn test_scope_validator_unknown() {
        let surface = RollbackSurface::full();
        let result = RollbackScopeValidator::validate_action("unknown_action", &surface);
        assert!(matches!(result, ValidationResult::Unknown));
    }

    #[test]
    fn test_can_perform_with_checkpoint() {
        let surface = RollbackSurface::full();
        assert!(
            RollbackScopeValidator::can_perform("tool_call_file_write", &surface, true).is_ok()
        );
    }

    #[test]
    fn test_can_perform_without_checkpoint() {
        let surface = RollbackSurface::full();
        assert!(
            RollbackScopeValidator::can_perform("tool_call_file_write", &surface, false).is_err()
        );
    }

    #[test]
    fn test_unsupported_action_error() {
        let error = UnsupportedActionError {
            action: "test".to_string(),
            reason: "reason".to_string(),
        };
        assert_eq!(error.action, "test");
        assert_eq!(error.reason, "reason");
    }

    #[test]
    fn test_scoped_action_serialization() {
        let action = ScopedAction {
            name: "test",
            category: ActionCategory::Create,
            reversibility: Reversibility::FullyReversible,
            requires_checkpoint: false,
            creates_branch: true,
            description: "test action",
        };
        let json_str = serde_json::to_string(&action).unwrap();
        assert!(json_str.contains("\"test\""));
        assert!(json_str.contains("\"Create\""));
    }

    #[test]
    fn test_all_reversible_actions_have_names() {
        let surface = RollbackSurface::full();
        for action in &surface.reversible_actions {
            assert!(!action.name.is_empty());
            assert!(!action.description.is_empty());
        }
    }

    #[test]
    fn test_all_irreversible_actions_have_names() {
        let surface = RollbackSurface::full();
        for action in &surface.irreversible_actions {
            assert!(!action.name.is_empty());
            assert!(!action.description.is_empty());
        }
    }
}
