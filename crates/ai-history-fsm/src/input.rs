//! Input events for the FSM
//!
//! These events drive state transitions.

use ai_history_core::{BranchId, CheckpointId, SessionId, TurnId};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Input events that drive the FSM
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum InputEvent {
    /// User submitted a prompt
    UserPromptSubmitted {
        content: String,
        session_id: SessionId,
    },

    /// Assistant emitted a delta (partial response)
    AssistantDelta { text: String, turn_id: TurnId },

    /// Assistant completed response
    AssistantDone { turn_id: TurnId },

    /// A tool call was requested
    ToolCallRequested {
        tool_name: String,
        args: serde_json::Value,
        turn_id: TurnId,
    },

    /// Tool call completed
    ToolCallCompleted {
        result: serde_json::Value,
        turn_id: TurnId,
    },

    /// A file patch was proposed
    FilePatchProposed { patch_id: String, file_path: String },

    /// A file patch was applied
    FilePatchApplied { patch_id: String },

    /// A file patch was rejected
    FilePatchRejected { patch_id: String, reason: String },

    /// User requested checkpoint creation
    CheckpointRequested {
        name: Option<String>,
        reason: CheckpointReason,
    },

    /// Checkpoint was created
    CheckpointCreated { checkpoint_id: CheckpointId },

    /// User requested restore from checkpoint
    RestoreRequested {
        checkpoint_id: CheckpointId,
        mode: RestoreMode,
    },

    /// Restore completed
    RestoreCompleted,

    /// Restore was cancelled
    RestoreCancelled,

    /// Workspace conflict detected
    WorkspaceConflictDetected { files: Vec<String> },

    /// Conflict resolved
    ConflictResolved { resolution: ConflictResolution },

    /// Cancellation requested
    CancellationRequested,

    /// Error occurred
    Error { error: String, recoverable: bool },

    /// Invalid transition attempted
    InvalidTransition { from_state: String, event: String },

    /// Navigate backward in history
    NavigateBackward { steps: u32 },

    /// Navigate forward in history
    NavigateForward { steps: u32 },

    /// Fork conversation from current branch head
    ForkCurrentConversation { name: Option<String> },

    /// Fork conversation from cursor position
    ForkFromCursor {
        name: Option<String>,
        restore_workspace: bool,
    },

    /// Continue conversation from cursor position (edit-and-rerun)
    ContinueFromCursor { content: String },

    /// Switch to a different branch
    BranchSwitch { branch_id: BranchId },

    /// Request explicit completion of the current turn
    TurnFinalized { turn_id: TurnId },

    /// Restore workspace only (no conversation changes)
    RestoreWorkspaceOnly { checkpoint_id: CheckpointId },

    /// Conversation-only restore (no workspace changes)
    RestoreConversationOnly { checkpoint_id: CheckpointId },
}

/// Reason for checkpoint creation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckpointReason {
    /// Before user prompt
    BeforeUserPrompt,
    /// Before file write
    BeforeFileWrite,
    /// After assistant turn
    AfterAssistantTurn,
    /// Safety checkpoint
    Safety,
    /// Named checkpoint
    Named,
    /// Pre-restore safety
    PreRestore,
}

impl Default for CheckpointReason {
    fn default() -> Self {
        Self::Safety
    }
}

/// Mode for restore operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreMode {
    /// Restore conversation only
    ConversationOnly,
    /// Restore workspace only
    WorkspaceOnly,
    /// Restore both conversation and workspace
    Both,
    /// Fork from checkpoint (preserves current state)
    Fork,
}

impl Default for RestoreMode {
    fn default() -> Self {
        Self::Both
    }
}

/// How to resolve a workspace conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Keep current workspace state
    KeepCurrent,
    /// Restore checkpoint version
    RestoreCheckpoint,
    /// Attempt to merge changes
    Merge,
    /// Save current as safety snapshot and restore
    SaveCurrentAndRestore,
    /// Cancel the restore operation
    Cancel,
}

impl Default for ConflictResolution {
    fn default() -> Self {
        Self::RestoreCheckpoint
    }
}

impl Display for InputEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InputEvent::UserPromptSubmitted { .. } => write!(f, "UserPromptSubmitted"),
            InputEvent::AssistantDelta { .. } => write!(f, "AssistantDelta"),
            InputEvent::AssistantDone { .. } => write!(f, "AssistantDone"),
            InputEvent::ToolCallRequested { .. } => write!(f, "ToolCallRequested"),
            InputEvent::ToolCallCompleted { .. } => write!(f, "ToolCallCompleted"),
            InputEvent::FilePatchProposed { .. } => write!(f, "FilePatchProposed"),
            InputEvent::FilePatchApplied { .. } => write!(f, "FilePatchApplied"),
            InputEvent::FilePatchRejected { .. } => write!(f, "FilePatchRejected"),
            InputEvent::CheckpointRequested { .. } => write!(f, "CheckpointRequested"),
            InputEvent::CheckpointCreated { .. } => write!(f, "CheckpointCreated"),
            InputEvent::RestoreRequested { .. } => write!(f, "RestoreRequested"),
            InputEvent::RestoreCompleted => write!(f, "RestoreCompleted"),
            InputEvent::RestoreCancelled => write!(f, "RestoreCancelled"),
            InputEvent::WorkspaceConflictDetected { .. } => write!(f, "WorkspaceConflictDetected"),
            InputEvent::ConflictResolved { .. } => write!(f, "ConflictResolved"),
            InputEvent::CancellationRequested => write!(f, "CancellationRequested"),
            InputEvent::Error { .. } => write!(f, "Error"),
            InputEvent::InvalidTransition { .. } => write!(f, "InvalidTransition"),
            InputEvent::NavigateBackward { .. } => write!(f, "NavigateBackward"),
            InputEvent::NavigateForward { .. } => write!(f, "NavigateForward"),
            InputEvent::ForkCurrentConversation { .. } => write!(f, "ForkCurrentConversation"),
            InputEvent::ForkFromCursor { .. } => write!(f, "ForkFromCursor"),
            InputEvent::ContinueFromCursor { .. } => write!(f, "ContinueFromCursor"),
            InputEvent::BranchSwitch { .. } => write!(f, "BranchSwitch"),
            InputEvent::TurnFinalized { .. } => write!(f, "TurnFinalized"),
            InputEvent::RestoreWorkspaceOnly { .. } => write!(f, "RestoreWorkspaceOnly"),
            InputEvent::RestoreConversationOnly { .. } => write!(f, "RestoreConversationOnly"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_reason_default() {
        assert_eq!(CheckpointReason::default(), CheckpointReason::Safety);
    }

    #[test]
    fn test_restore_mode_default() {
        assert_eq!(RestoreMode::default(), RestoreMode::Both);
    }

    #[test]
    fn test_conflict_resolution_default() {
        assert_eq!(
            ConflictResolution::default(),
            ConflictResolution::RestoreCheckpoint
        );
    }

    #[test]
    fn test_input_event_serialization() {
        let event = InputEvent::UserPromptSubmitted {
            content: "Hello".to_string(),
            session_id: SessionId::new(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: InputEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn test_input_event_display() {
        let event = InputEvent::AssistantDone {
            turn_id: TurnId::new(),
        };
        assert_eq!(event.to_string(), "AssistantDone");
    }
}
