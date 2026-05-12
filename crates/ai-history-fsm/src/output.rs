//! Output actions emitted by the FSM

use ai_history_core::{BranchId, CheckpointId, SessionId, TurnId};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Output actions emitted by the FSM on transitions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OutputAction {
    /// Append an event to the event log
    AppendEvent { event: serde_json::Value },

    /// Create a new turn
    CreateTurn {
        session_id: SessionId,
        branch_id: BranchId,
        parent_turn_id: Option<TurnId>,
    },

    /// Capture a workspace snapshot
    CaptureWorkspaceSnapshot {
        reason: String,
        checkpoint_id: CheckpointId,
    },

    /// Apply a file patch
    ApplyPatch { patch_id: String },

    /// Restore workspace from checkpoint
    RestoreWorkspace {
        checkpoint_id: CheckpointId,
        mode: crate::input::RestoreMode,
    },

    /// Move branch head to a different turn
    MoveBranchHead { turn_id: TurnId },

    /// Create a new branch
    CreateBranch {
        from_turn_id: TurnId,
        name: Option<String>,
    },

    /// Show conflict resolution UI
    ShowConflictUI { files: Vec<String> },

    /// Emit a notification to the UI
    EmitNotification {
        message: String,
        severity: NotificationSeverity,
    },

    /// Mark the turn as complete
    FinalizeTurn { turn_id: TurnId },

    /// Cancel any pending operations
    CancelPending,

    /// Begin polling for tool result
    BeginToolPolling { tool_name: String },

    /// Emit error event
    EmitError { error: String },

    /// Navigate to a turn in history
    NavigateToTurn { turn_id: TurnId },

    /// Fork the current branch at the head
    ForkBranch {
        from_turn_id: TurnId,
        name: Option<String>,
    },

    /// Switch to a different branch
    SwitchBranch { branch_id: BranchId },

    /// Restore workspace without affecting conversation
    RestoreWorkspaceOnly { checkpoint_id: CheckpointId },

    /// Restore conversation without affecting workspace
    RestoreConversationOnly { checkpoint_id: CheckpointId },
}

/// Severity level for notifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Error,
    Success,
}

impl Default for NotificationSeverity {
    fn default() -> Self {
        Self::Info
    }
}

impl Display for OutputAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputAction::AppendEvent { .. } => write!(f, "AppendEvent"),
            OutputAction::CreateTurn { .. } => write!(f, "CreateTurn"),
            OutputAction::CaptureWorkspaceSnapshot { .. } => write!(f, "CaptureWorkspaceSnapshot"),
            OutputAction::ApplyPatch { .. } => write!(f, "ApplyPatch"),
            OutputAction::RestoreWorkspace { .. } => write!(f, "RestoreWorkspace"),
            OutputAction::MoveBranchHead { .. } => write!(f, "MoveBranchHead"),
            OutputAction::CreateBranch { .. } => write!(f, "CreateBranch"),
            OutputAction::ShowConflictUI { .. } => write!(f, "ShowConflictUI"),
            OutputAction::EmitNotification { .. } => write!(f, "EmitNotification"),
            OutputAction::FinalizeTurn { .. } => write!(f, "FinalizeTurn"),
            OutputAction::CancelPending => write!(f, "CancelPending"),
            OutputAction::BeginToolPolling { .. } => write!(f, "BeginToolPolling"),
            OutputAction::EmitError { .. } => write!(f, "EmitError"),
            OutputAction::NavigateToTurn { .. } => write!(f, "NavigateToTurn"),
            OutputAction::ForkBranch { .. } => write!(f, "ForkBranch"),
            OutputAction::SwitchBranch { .. } => write!(f, "SwitchBranch"),
            OutputAction::RestoreWorkspaceOnly { .. } => write!(f, "RestoreWorkspaceOnly"),
            OutputAction::RestoreConversationOnly { .. } => write!(f, "RestoreConversationOnly"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_severity_default() {
        assert_eq!(NotificationSeverity::default(), NotificationSeverity::Info);
    }

    #[test]
    fn test_output_action_serialization() {
        let action = OutputAction::CreateTurn {
            session_id: SessionId::new(),
            branch_id: BranchId::new(),
            parent_turn_id: None,
        };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: OutputAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn test_output_action_display() {
        let action = OutputAction::CancelPending;
        assert_eq!(action.to_string(), "CancelPending");
    }
}
