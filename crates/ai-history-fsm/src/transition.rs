//! Finite-state transducer transition function
//!
//! Pure transition functions with no side effects.
//! Maps (current_phase, input_event) -> (next_phase, output_actions)

use crate::input::InputEvent;
use crate::output::{NotificationSeverity, OutputAction};
use crate::state::Phase;
use ai_history_core::{BranchId, CheckpointId};

#[cfg(test)]
use crate::input::{CheckpointReason, RestoreMode};

/// Transition result containing next state and actions
#[derive(Debug, Clone, PartialEq)]
pub struct TransitionResult {
    pub next_phase: Phase,
    pub actions: Vec<OutputAction>,
}

impl Default for TransitionResult {
    fn default() -> Self {
        Self {
            next_phase: Phase::Error,
            actions: vec![],
        }
    }
}

/// The finite-state transducer transition function
///
/// This is a pure function with no side effects. It maps
/// (current_phase, runtime_state, input_event) -> (next_phase, output_actions)
pub fn transition(
    current_phase: Phase,
    runtime_state: &RuntimeState,
    input: &InputEvent,
) -> TransitionResult {
    match (current_phase, input) {
        // =====================
        // Idle state transitions
        // =====================
        (
            Phase::Idle,
            InputEvent::UserPromptSubmitted {
                content,
                session_id,
            },
        ) => {
            let branch_id = runtime_state.branch_id.unwrap_or_else(BranchId::new);
            TransitionResult {
                next_phase: Phase::CreatingTurn,
                actions: vec![
                    OutputAction::CaptureWorkspaceSnapshot {
                        reason: "before_user_prompt".to_string(),
                        checkpoint_id: CheckpointId::new(),
                    },
                    OutputAction::CreateTurn {
                        session_id: *session_id,
                        branch_id,
                        parent_turn_id: None,
                    },
                    OutputAction::AppendEvent {
                        event: serde_json::json!({
                            "type": "UserPromptSubmitted",
                            "content": content
                        }),
                    },
                ],
            }
        }

        (
            Phase::Idle,
            InputEvent::RestoreRequested {
                checkpoint_id,
                mode,
            },
        ) => TransitionResult {
            next_phase: Phase::Restoring,
            actions: vec![
                OutputAction::CaptureWorkspaceSnapshot {
                    reason: "safety_before_restore".to_string(),
                    checkpoint_id: CheckpointId::new(),
                },
                OutputAction::RestoreWorkspace {
                    checkpoint_id: *checkpoint_id,
                    mode: *mode,
                },
                OutputAction::EmitNotification {
                    message: format!("Restoring from checkpoint {:?}", mode),
                    severity: NotificationSeverity::Info,
                },
            ],
        },

        (Phase::Idle, InputEvent::CheckpointRequested { name: _, reason }) => TransitionResult {
            next_phase: Phase::Checkpointing,
            actions: vec![OutputAction::CaptureWorkspaceSnapshot {
                reason: format!("{:?}", reason),
                checkpoint_id: CheckpointId::new(),
            }],
        },

        (
            Phase::Idle,
            InputEvent::Error {
                error,
                recoverable: true,
            },
        ) => TransitionResult {
            next_phase: Phase::Error,
            actions: vec![OutputAction::EmitError {
                error: error.clone(),
            }],
        },

        (Phase::Idle, InputEvent::CancellationRequested) => TransitionResult {
            next_phase: Phase::Cancelled,
            actions: vec![],
        },

        // ===============================
        // Navigation transitions (Idle)
        // ===============================
        (Phase::Idle, InputEvent::NavigateBackward { steps: _ }) => {
            TransitionResult {
                next_phase: Phase::Idle,
                actions: vec![OutputAction::NavigateToTurn {
                    turn_id: ai_history_core::TurnId::new(), // Cursor implementation provides actual turn
                }],
            }
        }

        (Phase::Idle, InputEvent::NavigateForward { steps: _ }) => {
            TransitionResult {
                next_phase: Phase::Idle,
                actions: vec![OutputAction::NavigateToTurn {
                    turn_id: ai_history_core::TurnId::new(), // Cursor implementation provides actual turn
                }],
            }
        }

        // ===============================
        // Fork transitions (Idle)
        // ===============================
        (Phase::Idle, InputEvent::ForkCurrentConversation { name }) => {
            let _branch_id = runtime_state.branch_id.unwrap_or_else(BranchId::new);
            let head_turn_id = runtime_state
                .current_turn_id
                .unwrap_or_else(ai_history_core::TurnId::new);
            TransitionResult {
                next_phase: Phase::Idle,
                actions: vec![
                    OutputAction::ForkBranch {
                        from_turn_id: head_turn_id,
                        name: name.clone(),
                    },
                    OutputAction::EmitNotification {
                        message: format!(
                            "Forked conversation{}",
                            name.as_ref()
                                .map(|n| format!(" as '{}'", n))
                                .unwrap_or_default()
                        ),
                        severity: NotificationSeverity::Info,
                    },
                ],
            }
        }

        (
            Phase::Idle,
            InputEvent::ForkFromCursor {
                name,
                restore_workspace,
            },
        ) => {
            let cursor_turn_id = runtime_state
                .current_turn_id
                .unwrap_or_else(ai_history_core::TurnId::new);
            let _branch_id = runtime_state.branch_id.unwrap_or_else(BranchId::new);
            let mut actions = vec![OutputAction::ForkBranch {
                from_turn_id: cursor_turn_id,
                name: name.clone(),
            }];
            if *restore_workspace {
                actions.push(OutputAction::RestoreWorkspaceOnly {
                    checkpoint_id: CheckpointId::new(),
                });
            }
            TransitionResult {
                next_phase: Phase::Idle,
                actions,
            }
        }

        (Phase::Idle, InputEvent::ContinueFromCursor { content }) => {
            let cursor_turn_id = runtime_state
                .current_turn_id
                .unwrap_or_else(ai_history_core::TurnId::new);
            let session_id = runtime_state
                .session_id
                .unwrap_or_else(ai_history_core::SessionId::new);
            let branch_id = runtime_state.branch_id.unwrap_or_else(BranchId::new);
            TransitionResult {
                next_phase: Phase::CreatingTurn,
                actions: vec![
                    OutputAction::CaptureWorkspaceSnapshot {
                        reason: "before_continue_from_cursor".to_string(),
                        checkpoint_id: CheckpointId::new(),
                    },
                    OutputAction::ForkBranch {
                        from_turn_id: cursor_turn_id,
                        name: None,
                    },
                    OutputAction::CreateTurn {
                        session_id,
                        branch_id,
                        parent_turn_id: Some(cursor_turn_id),
                    },
                    OutputAction::AppendEvent {
                        event: serde_json::json!({
                            "type": "UserPromptSubmitted",
                            "content": content
                        }),
                    },
                ],
            }
        }

        // ===============================
        // Branch switch (Idle)
        // ===============================
        (Phase::Idle, InputEvent::BranchSwitch { branch_id }) => TransitionResult {
            next_phase: Phase::Idle,
            actions: vec![
                OutputAction::SwitchBranch {
                    branch_id: *branch_id,
                },
                OutputAction::EmitNotification {
                    message: format!("Switched to branch {:?}", branch_id),
                    severity: NotificationSeverity::Info,
                },
            ],
        },

        // ===============================
        // Restore-only modes (Idle)
        // ===============================
        (Phase::Idle, InputEvent::RestoreWorkspaceOnly { checkpoint_id }) => TransitionResult {
            next_phase: Phase::Restoring,
            actions: vec![
                OutputAction::CaptureWorkspaceSnapshot {
                    reason: "safety_before_restore".to_string(),
                    checkpoint_id: CheckpointId::new(),
                },
                OutputAction::RestoreWorkspaceOnly {
                    checkpoint_id: *checkpoint_id,
                },
                OutputAction::EmitNotification {
                    message: "Restoring workspace only...".to_string(),
                    severity: NotificationSeverity::Info,
                },
            ],
        },

        (Phase::Idle, InputEvent::RestoreConversationOnly { checkpoint_id }) => TransitionResult {
            next_phase: Phase::Restoring,
            actions: vec![
                OutputAction::CaptureWorkspaceSnapshot {
                    reason: "safety_before_conversation_restore".to_string(),
                    checkpoint_id: CheckpointId::new(),
                },
                OutputAction::RestoreConversationOnly {
                    checkpoint_id: *checkpoint_id,
                },
                OutputAction::EmitNotification {
                    message: "Restoring conversation only...".to_string(),
                    severity: NotificationSeverity::Info,
                },
            ],
        },

        // ===========================
        // CreatingTurn state transitions
        // ===========================
        (Phase::CreatingTurn, InputEvent::AssistantDelta { text, turn_id }) => TransitionResult {
            next_phase: Phase::StreamingAssistant,
            actions: vec![OutputAction::AppendEvent {
                event: serde_json::json!({
                    "type": "AssistantDelta",
                    "text": text,
                    "turn_id": turn_id
                }),
            }],
        },

        (
            Phase::CreatingTurn,
            InputEvent::ToolCallRequested {
                tool_name,
                args,
                turn_id,
            },
        ) => TransitionResult {
            next_phase: Phase::AwaitingToolResult,
            actions: vec![
                OutputAction::AppendEvent {
                    event: serde_json::json!({
                        "type": "ToolCallRequested",
                        "tool_name": tool_name,
                        "args": args,
                        "turn_id": turn_id
                    }),
                },
                OutputAction::BeginToolPolling {
                    tool_name: tool_name.clone(),
                },
            ],
        },

        (Phase::CreatingTurn, InputEvent::AssistantDone { turn_id }) => TransitionResult {
            next_phase: Phase::Idle,
            actions: vec![
                OutputAction::CaptureWorkspaceSnapshot {
                    reason: "after_assistant_turn".to_string(),
                    checkpoint_id: CheckpointId::new(),
                },
                OutputAction::FinalizeTurn { turn_id: *turn_id },
                OutputAction::AppendEvent {
                    event: serde_json::json!({
                        "type": "AssistantDone",
                        "turn_id": turn_id
                    }),
                },
            ],
        },

        (Phase::CreatingTurn, InputEvent::TurnFinalized { turn_id }) => TransitionResult {
            next_phase: Phase::Idle,
            actions: vec![
                OutputAction::CaptureWorkspaceSnapshot {
                    reason: "after_turn_finalized".to_string(),
                    checkpoint_id: CheckpointId::new(),
                },
                OutputAction::FinalizeTurn { turn_id: *turn_id },
                OutputAction::AppendEvent {
                    event: serde_json::json!({
                        "type": "TurnFinalized",
                        "turn_id": turn_id
                    }),
                },
            ],
        },

        (
            Phase::CreatingTurn,
            InputEvent::Error {
                error,
                recoverable: false,
            },
        ) => TransitionResult {
            next_phase: Phase::Error,
            actions: vec![OutputAction::EmitError {
                error: error.clone(),
            }],
        },

        // ================================
        // StreamingAssistant state transitions
        // ================================
        (Phase::StreamingAssistant, InputEvent::TurnFinalized { turn_id }) => TransitionResult {
            next_phase: Phase::Idle,
            actions: vec![
                OutputAction::CaptureWorkspaceSnapshot {
                    reason: "after_turn_finalized".to_string(),
                    checkpoint_id: CheckpointId::new(),
                },
                OutputAction::FinalizeTurn { turn_id: *turn_id },
                OutputAction::AppendEvent {
                    event: serde_json::json!({
                        "type": "TurnFinalized",
                        "turn_id": turn_id
                    }),
                },
            ],
        },

        (Phase::StreamingAssistant, InputEvent::AssistantDelta { text, turn_id }) => {
            TransitionResult {
                next_phase: Phase::StreamingAssistant,
                actions: vec![OutputAction::AppendEvent {
                    event: serde_json::json!({
                        "type": "AssistantDelta",
                        "text": text,
                        "turn_id": turn_id
                    }),
                }],
            }
        }

        (
            Phase::StreamingAssistant,
            InputEvent::FilePatchProposed {
                patch_id,
                file_path,
            },
        ) => TransitionResult {
            next_phase: Phase::ApplyingPatch,
            actions: vec![
                OutputAction::CaptureWorkspaceSnapshot {
                    reason: "before_file_patch".to_string(),
                    checkpoint_id: CheckpointId::new(),
                },
                OutputAction::ApplyPatch {
                    patch_id: patch_id.clone(),
                },
                OutputAction::AppendEvent {
                    event: serde_json::json!({
                        "type": "FilePatchProposed",
                        "patch_id": patch_id,
                        "file_path": file_path
                    }),
                },
            ],
        },

        (
            Phase::StreamingAssistant,
            InputEvent::ToolCallRequested {
                tool_name,
                args,
                turn_id,
            },
        ) => TransitionResult {
            next_phase: Phase::AwaitingToolResult,
            actions: vec![
                OutputAction::AppendEvent {
                    event: serde_json::json!({
                        "type": "ToolCallRequested",
                        "tool_name": tool_name,
                        "args": args,
                        "turn_id": turn_id
                    }),
                },
                OutputAction::BeginToolPolling {
                    tool_name: tool_name.clone(),
                },
            ],
        },

        (Phase::StreamingAssistant, InputEvent::AssistantDone { turn_id }) => TransitionResult {
            next_phase: Phase::Idle,
            actions: vec![
                OutputAction::CaptureWorkspaceSnapshot {
                    reason: "after_assistant_turn".to_string(),
                    checkpoint_id: CheckpointId::new(),
                },
                OutputAction::FinalizeTurn { turn_id: *turn_id },
                OutputAction::AppendEvent {
                    event: serde_json::json!({
                        "type": "AssistantDone",
                        "turn_id": turn_id
                    }),
                },
            ],
        },

        (Phase::StreamingAssistant, InputEvent::CancellationRequested) => TransitionResult {
            next_phase: Phase::Cancelled,
            actions: vec![OutputAction::CancelPending],
        },

        // =================================
        // AwaitingToolResult state transitions
        // =================================
        (Phase::AwaitingToolResult, InputEvent::ToolCallCompleted { result, turn_id }) => {
            TransitionResult {
                next_phase: Phase::StreamingAssistant,
                actions: vec![OutputAction::AppendEvent {
                    event: serde_json::json!({
                        "type": "ToolCallCompleted",
                        "result": result,
                        "turn_id": turn_id
                    }),
                }],
            }
        }

        (
            Phase::AwaitingToolResult,
            InputEvent::ToolCallRequested {
                tool_name,
                args,
                turn_id,
            },
        ) => TransitionResult {
            next_phase: Phase::AwaitingToolResult,
            actions: vec![
                OutputAction::AppendEvent {
                    event: serde_json::json!({
                        "type": "ToolCallRequested",
                        "tool_name": tool_name,
                        "args": args,
                        "turn_id": turn_id
                    }),
                },
                OutputAction::BeginToolPolling {
                    tool_name: tool_name.clone(),
                },
            ],
        },

        (Phase::AwaitingToolResult, InputEvent::CancellationRequested) => TransitionResult {
            next_phase: Phase::Cancelled,
            actions: vec![OutputAction::CancelPending],
        },

        // ============================
        // ApplyingPatch state transitions
        // ============================
        (Phase::ApplyingPatch, InputEvent::FilePatchApplied { patch_id }) => TransitionResult {
            next_phase: Phase::StreamingAssistant,
            actions: vec![
                OutputAction::CaptureWorkspaceSnapshot {
                    reason: "after_file_patch".to_string(),
                    checkpoint_id: CheckpointId::new(),
                },
                OutputAction::AppendEvent {
                    event: serde_json::json!({
                        "type": "FilePatchApplied",
                        "patch_id": patch_id
                    }),
                },
            ],
        },

        (Phase::ApplyingPatch, InputEvent::FilePatchRejected { patch_id, reason }) => {
            TransitionResult {
                next_phase: Phase::StreamingAssistant,
                actions: vec![OutputAction::AppendEvent {
                    event: serde_json::json!({
                        "type": "FilePatchRejected",
                        "patch_id": patch_id,
                        "reason": reason
                    }),
                }],
            }
        }

        (Phase::ApplyingPatch, InputEvent::CancellationRequested) => TransitionResult {
            next_phase: Phase::Cancelled,
            actions: vec![OutputAction::CancelPending],
        },

        // ============================
        // Checkpointing state transitions
        // ============================
        (Phase::Checkpointing, InputEvent::CheckpointCreated { checkpoint_id }) => {
            TransitionResult {
                next_phase: Phase::Idle,
                actions: vec![OutputAction::EmitNotification {
                    message: format!("Checkpoint created: {:?}", checkpoint_id),
                    severity: NotificationSeverity::Success,
                }],
            }
        }

        (
            Phase::Checkpointing,
            InputEvent::Error {
                error,
                recoverable: true,
            },
        ) => TransitionResult {
            next_phase: Phase::Idle,
            actions: vec![OutputAction::EmitError {
                error: error.clone(),
            }],
        },

        (Phase::Checkpointing, InputEvent::CancellationRequested) => TransitionResult {
            next_phase: Phase::Cancelled,
            actions: vec![OutputAction::EmitNotification {
                message: "Checkpoint cancelled".to_string(),
                severity: NotificationSeverity::Warning,
            }],
        },

        // ========================
        // Restoring state transitions
        // ========================
        (Phase::Restoring, InputEvent::WorkspaceConflictDetected { files }) => TransitionResult {
            next_phase: Phase::ConflictResolution,
            actions: vec![OutputAction::ShowConflictUI {
                files: files.clone(),
            }],
        },

        (Phase::Restoring, InputEvent::RestoreCompleted) => TransitionResult {
            next_phase: Phase::Idle,
            actions: vec![OutputAction::EmitNotification {
                message: "Restore completed".to_string(),
                severity: NotificationSeverity::Success,
            }],
        },

        (Phase::Restoring, InputEvent::RestoreCancelled) => TransitionResult {
            next_phase: Phase::Idle,
            actions: vec![OutputAction::EmitNotification {
                message: "Restore cancelled".to_string(),
                severity: NotificationSeverity::Warning,
            }],
        },

        (Phase::Restoring, InputEvent::CancellationRequested) => TransitionResult {
            next_phase: Phase::Cancelled,
            actions: vec![OutputAction::EmitNotification {
                message: "Restore cancelled".to_string(),
                severity: NotificationSeverity::Warning,
            }],
        },

        (
            Phase::Restoring,
            InputEvent::Error {
                error,
                recoverable: true,
            },
        ) => TransitionResult {
            next_phase: Phase::Idle,
            actions: vec![OutputAction::EmitError {
                error: error.clone(),
            }],
        },

        // ================================
        // ConflictResolution state transitions
        // ================================
        (Phase::ConflictResolution, InputEvent::CancellationRequested) => TransitionResult {
            next_phase: Phase::Cancelled,
            actions: vec![OutputAction::EmitNotification {
                message: "Conflict resolution cancelled".to_string(),
                severity: NotificationSeverity::Warning,
            }],
        },

        (Phase::ConflictResolution, InputEvent::ConflictResolved { resolution }) => {
            let next_phase = match resolution {
                crate::input::ConflictResolution::Cancel => Phase::Idle,
                _ => Phase::Restoring,
            };
            TransitionResult {
                next_phase,
                actions: vec![],
            }
        }

        // ========================
        // Cancelled state transitions
        // ========================
        (
            Phase::Cancelled,
            InputEvent::UserPromptSubmitted {
                content,
                session_id,
            },
        ) => {
            let branch_id = runtime_state.branch_id.unwrap_or_else(BranchId::new);
            TransitionResult {
                next_phase: Phase::CreatingTurn,
                actions: vec![
                    OutputAction::CaptureWorkspaceSnapshot {
                        reason: "before_user_prompt".to_string(),
                        checkpoint_id: CheckpointId::new(),
                    },
                    OutputAction::CreateTurn {
                        session_id: *session_id,
                        branch_id,
                        parent_turn_id: None,
                    },
                    OutputAction::AppendEvent {
                        event: serde_json::json!({
                            "type": "UserPromptSubmitted",
                            "content": content
                        }),
                    },
                ],
            }
        }

        // ========================
        // Error state transitions
        // ========================
        (
            Phase::Error,
            InputEvent::UserPromptSubmitted {
                content,
                session_id,
            },
        ) => {
            let branch_id = runtime_state.branch_id.unwrap_or_else(BranchId::new);
            TransitionResult {
                next_phase: Phase::CreatingTurn,
                actions: vec![
                    OutputAction::CaptureWorkspaceSnapshot {
                        reason: "before_user_prompt".to_string(),
                        checkpoint_id: CheckpointId::new(),
                    },
                    OutputAction::CreateTurn {
                        session_id: *session_id,
                        branch_id,
                        parent_turn_id: None,
                    },
                    OutputAction::AppendEvent {
                        event: serde_json::json!({
                            "type": "UserPromptSubmitted",
                            "content": content
                        }),
                    },
                ],
            }
        }

        // ========================
        // Invalid transitions
        // ========================
        _ => TransitionResult {
            next_phase: Phase::Error,
            actions: vec![OutputAction::EmitError {
                error: format!("Invalid transition: {:?} + {:?}", current_phase, input),
            }],
        },
    }
}

/// Extended runtime state combining finite phase with external registers
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeState {
    pub phase: Phase,
    pub session_id: Option<ai_history_core::SessionId>,
    pub branch_id: Option<BranchId>,
    pub current_turn_id: Option<ai_history_core::TurnId>,
    pub current_checkpoint_id: Option<ai_history_core::CheckpointId>,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            session_id: None,
            branch_id: None,
            current_turn_id: None,
            current_checkpoint_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::ConflictResolution;
    use ai_history_core::{BranchId, SessionId, TurnId};

    fn make_runtime_state() -> RuntimeState {
        RuntimeState::default()
    }

    #[test]
    fn test_idle_user_prompt_transition() {
        let session_id = SessionId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::UserPromptSubmitted {
                content: "Hello".to_string(),
                session_id,
            },
        );
        assert_eq!(result.next_phase, Phase::CreatingTurn);
        assert!(!result.actions.is_empty());
    }

    #[test]
    fn test_idle_restore_transition() {
        let checkpoint_id = CheckpointId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::RestoreRequested {
                checkpoint_id,
                mode: RestoreMode::Both,
            },
        );
        assert_eq!(result.next_phase, Phase::Restoring);
    }

    #[test]
    fn test_streaming_assistant_delta() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::StreamingAssistant,
            &runtime,
            &InputEvent::AssistantDelta {
                text: "Hello".to_string(),
                turn_id,
            },
        );
        assert_eq!(result.next_phase, Phase::StreamingAssistant);
        assert!(!result.actions.is_empty());
    }

    #[test]
    fn test_streaming_to_done() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::StreamingAssistant,
            &runtime,
            &InputEvent::AssistantDone { turn_id },
        );
        assert_eq!(result.next_phase, Phase::Idle);
    }

    #[test]
    fn test_streaming_tool_request() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::StreamingAssistant,
            &runtime,
            &InputEvent::ToolCallRequested {
                tool_name: "write_file".to_string(),
                args: serde_json::json!({}),
                turn_id,
            },
        );
        assert_eq!(result.next_phase, Phase::AwaitingToolResult);
    }

    #[test]
    fn test_awaiting_tool_to_streaming() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::AwaitingToolResult,
            &runtime,
            &InputEvent::ToolCallCompleted {
                result: serde_json::json!({"status": "ok"}),
                turn_id,
            },
        );
        assert_eq!(result.next_phase, Phase::StreamingAssistant);
    }

    #[test]
    fn test_conflict_resolution_cancel() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::ConflictResolution,
            &runtime,
            &InputEvent::ConflictResolved {
                resolution: ConflictResolution::Cancel,
            },
        );
        assert_eq!(result.next_phase, Phase::Idle);
    }

    #[test]
    fn test_cancelled_to_creating_turn() {
        let session_id = SessionId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Cancelled,
            &runtime,
            &InputEvent::UserPromptSubmitted {
                content: "Hello".to_string(),
                session_id,
            },
        );
        assert_eq!(result.next_phase, Phase::CreatingTurn);
    }

    #[test]
    fn test_invalid_transition() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::AssistantDone { turn_id },
        );
        assert_eq!(result.next_phase, Phase::Error);
    }

    #[test]
    fn test_runtime_state_default() {
        let state = RuntimeState::default();
        assert_eq!(state.phase, Phase::Idle);
        assert!(state.session_id.is_none());
    }

    #[test]
    fn test_cancellation_during_streaming() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::StreamingAssistant,
            &runtime,
            &InputEvent::CancellationRequested,
        );
        assert_eq!(result.next_phase, Phase::Cancelled);
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::CancelPending)));
    }

    #[test]
    fn test_cancellation_during_tool_execution() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::AwaitingToolResult,
            &runtime,
            &InputEvent::CancellationRequested,
        );
        assert_eq!(result.next_phase, Phase::Cancelled);
    }

    #[test]
    fn test_cancellation_during_patch_application() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::ApplyingPatch,
            &runtime,
            &InputEvent::CancellationRequested,
        );
        assert_eq!(result.next_phase, Phase::Cancelled);
    }

    #[test]
    fn test_cancellation_preserves_partial_events() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::StreamingAssistant,
            &runtime,
            &InputEvent::CancellationRequested,
        );
        assert_eq!(result.next_phase, Phase::Cancelled);
        let has_cancel = result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::CancelPending));
        assert!(has_cancel);
    }

    #[test]
    fn test_cancellation_during_checkpointing() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Checkpointing,
            &runtime,
            &InputEvent::CancellationRequested,
        );
        assert_eq!(result.next_phase, Phase::Cancelled);
    }

    #[test]
    fn test_cancellation_during_conflict_resolution() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::ConflictResolution,
            &runtime,
            &InputEvent::CancellationRequested,
        );
        assert_eq!(result.next_phase, Phase::Cancelled);
        let has_warning = result.actions.iter().any(|a| {
            matches!(
                a, OutputAction::EmitNotification { message, severity }
                if message.contains("cancelled") && *severity == NotificationSeverity::Warning
            )
        });
        assert!(has_warning);
    }

    #[test]
    fn test_cancellation_during_restore() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Restoring,
            &runtime,
            &InputEvent::CancellationRequested,
        );
        assert_eq!(result.next_phase, Phase::Cancelled);
    }

    #[test]
    fn test_error_during_checkpointing_recoverable() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Checkpointing,
            &runtime,
            &InputEvent::Error {
                error: "checkpoint failed".to_string(),
                recoverable: true,
            },
        );
        assert_eq!(result.next_phase, Phase::Idle);
    }

    #[test]
    fn test_error_during_checkpointing_not_recoverable() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Checkpointing,
            &runtime,
            &InputEvent::Error {
                error: "fatal error".to_string(),
                recoverable: false,
            },
        );
        assert_eq!(result.next_phase, Phase::Error);
    }

    #[test]
    fn test_restore_requested_from_idle() {
        let checkpoint_id = ai_history_core::CheckpointId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::RestoreRequested {
                checkpoint_id,
                mode: RestoreMode::Both,
            },
        );
        assert_eq!(result.next_phase, Phase::Restoring);
    }

    #[test]
    fn test_conflict_during_restore_transitions_to_conflict_resolution() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Restoring,
            &runtime,
            &InputEvent::WorkspaceConflictDetected {
                files: vec!["file1.rs".to_string(), "file2.rs".to_string()],
            },
        );
        assert_eq!(result.next_phase, Phase::ConflictResolution);
    }

    #[test]
    fn test_conflict_resolved_cancel_returns_to_idle() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::ConflictResolution,
            &runtime,
            &InputEvent::ConflictResolved {
                resolution: crate::input::ConflictResolution::Cancel,
            },
        );
        assert_eq!(result.next_phase, Phase::Idle);
    }

    #[test]
    fn test_conflict_resolved_restorecheckpoint_resumes_restoring() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::ConflictResolution,
            &runtime,
            &InputEvent::ConflictResolved {
                resolution: crate::input::ConflictResolution::RestoreCheckpoint,
            },
        );
        assert_eq!(result.next_phase, Phase::Restoring);
    }

    #[test]
    fn test_assistant_done_transitions_to_idle() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::StreamingAssistant,
            &runtime,
            &InputEvent::AssistantDone { turn_id },
        );
        assert_eq!(result.next_phase, Phase::Idle);
    }

    #[test]
    fn test_assistant_done_captures_workspace_snapshot() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::StreamingAssistant,
            &runtime,
            &InputEvent::AssistantDone { turn_id },
        );
        let has_snapshot = result.actions.iter().any(|a| {
            matches!(a, OutputAction::CaptureWorkspaceSnapshot { reason, .. }
                if reason == "after_assistant_turn")
        });
        assert!(has_snapshot);
    }

    #[test]
    fn test_invalid_transition_from_idle_to_assistant_done() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::AssistantDone { turn_id },
        );
        assert_eq!(result.next_phase, Phase::Error);
    }

    #[test]
    fn test_invalid_transition_from_idle_to_tool_completed() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::ToolCallCompleted {
                result: serde_json::json!({}),
                turn_id,
            },
        );
        assert_eq!(result.next_phase, Phase::Error);
    }

    // ===============================
    // New transition tests for Task 25/26
    // ===============================

    #[test]
    fn test_fork_current_conversation() {
        let mut runtime = make_runtime_state();
        runtime.branch_id = Some(BranchId::new());
        runtime.current_turn_id = Some(ai_history_core::TurnId::new());

        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::ForkCurrentConversation {
                name: Some("feature-x".to_string()),
            },
        );
        assert_eq!(result.next_phase, Phase::Idle);
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::ForkBranch { .. })));
    }

    #[test]
    fn test_fork_from_cursor_without_workspace_restore() {
        let mut runtime = make_runtime_state();
        runtime.branch_id = Some(BranchId::new());
        runtime.current_turn_id = Some(ai_history_core::TurnId::new());

        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::ForkFromCursor {
                name: None,
                restore_workspace: false,
            },
        );
        assert_eq!(result.next_phase, Phase::Idle);
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::ForkBranch { .. })));
    }

    #[test]
    fn test_fork_from_cursor_with_workspace_restore() {
        let mut runtime = make_runtime_state();
        runtime.branch_id = Some(BranchId::new());
        runtime.current_turn_id = Some(ai_history_core::TurnId::new());

        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::ForkFromCursor {
                name: None,
                restore_workspace: true,
            },
        );
        assert_eq!(result.next_phase, Phase::Idle);
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::ForkBranch { .. })));
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::RestoreWorkspaceOnly { .. })));
    }

    #[test]
    fn test_continue_from_cursor() {
        let mut runtime = make_runtime_state();
        runtime.session_id = Some(SessionId::new());
        runtime.branch_id = Some(BranchId::new());
        runtime.current_turn_id = Some(ai_history_core::TurnId::new());

        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::ContinueFromCursor {
                content: "Hello again".to_string(),
            },
        );
        assert_eq!(result.next_phase, Phase::CreatingTurn);
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::ForkBranch { .. })));
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::CreateTurn { .. })));
    }

    #[test]
    fn test_branch_switch() {
        let branch_id = BranchId::new();
        let runtime = make_runtime_state();

        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::BranchSwitch { branch_id },
        );
        assert_eq!(result.next_phase, Phase::Idle);
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::SwitchBranch { .. })));
    }

    #[test]
    fn test_turn_finalized() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();

        let result = transition(
            Phase::CreatingTurn,
            &runtime,
            &InputEvent::TurnFinalized { turn_id },
        );
        assert_eq!(result.next_phase, Phase::Idle);
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::FinalizeTurn { .. })));
    }

    #[test]
    fn test_restore_workspace_only() {
        let checkpoint_id = CheckpointId::new();
        let runtime = make_runtime_state();

        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::RestoreWorkspaceOnly { checkpoint_id },
        );
        assert_eq!(result.next_phase, Phase::Restoring);
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::RestoreWorkspaceOnly { .. })));
    }

    #[test]
    fn test_restore_conversation_only() {
        let checkpoint_id = CheckpointId::new();
        let runtime = make_runtime_state();

        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::RestoreConversationOnly { checkpoint_id },
        );
        assert_eq!(result.next_phase, Phase::Restoring);
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::RestoreConversationOnly { .. })));
    }

    #[test]
    fn test_navigate_backward() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::NavigateBackward { steps: 1 },
        );
        assert_eq!(result.next_phase, Phase::Idle);
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::NavigateToTurn { .. })));
    }

    #[test]
    fn test_navigate_forward() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::NavigateForward { steps: 1 },
        );
        assert_eq!(result.next_phase, Phase::Idle);
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::NavigateToTurn { .. })));
    }

    #[test]
    fn test_runtime_state_uses_branch_id_type() {
        let mut state = RuntimeState::default();
        let branch_id = BranchId::new();
        state.branch_id = Some(branch_id);
        assert_eq!(state.branch_id, Some(branch_id));
    }

    #[test]
    fn test_output_action_create_turn_uses_branch_id() {
        let action = OutputAction::CreateTurn {
            session_id: SessionId::new(),
            branch_id: BranchId::new(),
            parent_turn_id: None,
        };
        match action {
            OutputAction::CreateTurn { branch_id, .. } => {
                assert!(matches!(branch_id, BranchId { .. }));
            }
            _ => panic!("Expected CreateTurn variant"),
        }
    }

    // =====================================================================
    // Negative transition tests - invalid event/state combinations
    // =====================================================================

    #[test]
    fn test_invalid_tool_completed_from_idle() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::ToolCallCompleted {
                result: serde_json::json!({"status": "ok"}),
                turn_id,
            },
        );
        // ToolCallCompleted is invalid in Idle - should transition to Error
        assert_eq!(result.next_phase, Phase::Error);
        let has_error = result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::EmitError { .. }));
        assert!(
            has_error,
            "Expected EmitError action for invalid transition"
        );
    }

    #[test]
    fn test_invalid_assistant_done_from_idle() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::AssistantDone { turn_id },
        );
        assert_eq!(result.next_phase, Phase::Error);
    }

    #[test]
    fn test_invalid_file_patch_applied_from_streaming() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::StreamingAssistant,
            &runtime,
            &InputEvent::FilePatchApplied {
                patch_id: "patch-123".to_string(),
            },
        );
        // FilePatchApplied is only valid in ApplyingPatch state
        assert_eq!(result.next_phase, Phase::Error);
    }

    #[test]
    fn test_invalid_file_patch_rejected_from_streaming() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::StreamingAssistant,
            &runtime,
            &InputEvent::FilePatchRejected {
                patch_id: "patch-123".to_string(),
                reason: "syntax error".to_string(),
            },
        );
        assert_eq!(result.next_phase, Phase::Error);
    }

    #[test]
    fn test_invalid_checkpoint_created_from_idle() {
        let checkpoint_id = CheckpointId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::CheckpointCreated { checkpoint_id },
        );
        // CheckpointCreated is only valid in Checkpointing state
        assert_eq!(result.next_phase, Phase::Error);
    }

    #[test]
    fn test_invalid_restore_completed_from_idle() {
        let runtime = make_runtime_state();
        let result = transition(Phase::Idle, &runtime, &InputEvent::RestoreCompleted);
        // RestoreCompleted is only valid in Restoring state
        assert_eq!(result.next_phase, Phase::Error);
    }

    #[test]
    fn test_invalid_conflict_resolved_from_idle() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::ConflictResolved {
                resolution: ConflictResolution::Cancel,
            },
        );
        // ConflictResolved is only valid in ConflictResolution state
        assert_eq!(result.next_phase, Phase::Error);
    }

    #[test]
    fn test_invalid_workspace_conflict_from_idle() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::WorkspaceConflictDetected {
                files: vec!["file.rs".to_string()],
            },
        );
        // WorkspaceConflictDetected is only valid in Restoring state
        assert_eq!(result.next_phase, Phase::Error);
    }

    #[test]
    fn test_invalid_assistant_delta_from_idle() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::AssistantDelta {
                text: "Hello".to_string(),
                turn_id,
            },
        );
        // AssistantDelta is only valid in CreatingTurn or StreamingAssistant
        assert_eq!(result.next_phase, Phase::Error);
    }

    #[test]
    fn test_invalid_tool_requested_from_idle() {
        let turn_id = ai_history_core::TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::ToolCallRequested {
                tool_name: "write_file".to_string(),
                args: serde_json::json!({}),
                turn_id,
            },
        );
        // ToolCallRequested is only valid in CreatingTurn or StreamingAssistant
        assert_eq!(result.next_phase, Phase::Error);
    }

    #[test]
    fn test_invalid_file_patch_proposed_from_idle() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::FilePatchProposed {
                patch_id: "patch-123".to_string(),
                file_path: "src/main.rs".to_string(),
            },
        );
        // FilePatchProposed is only valid in StreamingAssistant
        assert_eq!(result.next_phase, Phase::Error);
    }

    #[test]
    fn test_all_invalid_transitions_produce_error_phase() {
        // Test that ALL invalid transitions go to Error phase (not some other state)
        let runtime = make_runtime_state();
        let invalid_combos = vec![
            (
                Phase::Idle,
                InputEvent::ToolCallCompleted {
                    result: serde_json::json!({}),
                    turn_id: TurnId::new(),
                },
            ),
            (
                Phase::Idle,
                InputEvent::AssistantDone {
                    turn_id: TurnId::new(),
                },
            ),
            (
                Phase::Idle,
                InputEvent::FilePatchApplied {
                    patch_id: "p".to_string(),
                },
            ),
            (
                Phase::Idle,
                InputEvent::FilePatchRejected {
                    patch_id: "p".to_string(),
                    reason: "r".to_string(),
                },
            ),
            (
                Phase::Idle,
                InputEvent::CheckpointCreated {
                    checkpoint_id: CheckpointId::new(),
                },
            ),
            (Phase::Idle, InputEvent::RestoreCompleted),
            (
                Phase::Idle,
                InputEvent::ConflictResolved {
                    resolution: ConflictResolution::Cancel,
                },
            ),
            (
                Phase::Idle,
                InputEvent::WorkspaceConflictDetected { files: vec![] },
            ),
            (
                Phase::CreatingTurn,
                InputEvent::CheckpointCreated {
                    checkpoint_id: CheckpointId::new(),
                },
            ),
            (Phase::CreatingTurn, InputEvent::RestoreCompleted),
            (Phase::StreamingAssistant, InputEvent::RestoreCompleted),
            (
                Phase::AwaitingToolResult,
                InputEvent::UserPromptSubmitted {
                    content: "test".to_string(),
                    session_id: SessionId::new(),
                },
            ),
        ];

        for (phase, input) in invalid_combos {
            let result = transition(phase, &runtime, &input);
            assert_eq!(
                result.next_phase,
                Phase::Error,
                "Invalid transition {:?} + {:?} should go to Error, got {:?}",
                phase,
                input,
                result.next_phase
            );
        }
    }

    // =====================================================================
    // Checkpoint trigger tests
    // =====================================================================

    #[test]
    fn test_checkpoint_request_captures_workspace_snapshot() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::CheckpointRequested {
                name: Some("test-checkpoint".to_string()),
                reason: CheckpointReason::Named,
            },
        );
        // Should emit CaptureWorkspaceSnapshot action
        let has_capture = result.actions.iter().any(|a| {
            matches!(a, OutputAction::CaptureWorkspaceSnapshot { reason, .. }
                if reason.contains("Named"))
        });
        assert!(
            has_capture,
            "Expected CaptureWorkspaceSnapshot action for checkpoint request"
        );
    }

    #[test]
    fn test_restore_request_captures_safety_checkpoint() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::RestoreRequested {
                checkpoint_id: CheckpointId::new(),
                mode: RestoreMode::Both,
            },
        );
        // Should emit safety checkpoint before restore
        let has_safety = result.actions.iter().any(|a| {
            matches!(a, OutputAction::CaptureWorkspaceSnapshot { reason, .. }
                if reason.contains("safety"))
        });
        assert!(
            has_safety,
            "Restore should capture safety checkpoint before proceeding"
        );
    }

    // =====================================================================
    // Fork flow tests
    // =====================================================================

    #[test]
    fn test_fork_current_conversation_action_content() {
        let mut runtime = make_runtime_state();
        let branch_id = BranchId::new();
        let turn_id = TurnId::new();
        runtime.branch_id = Some(branch_id);
        runtime.current_turn_id = Some(turn_id);

        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::ForkCurrentConversation {
                name: Some("feature-x".to_string()),
            },
        );

        // Verify ForkBranch action has correct from_turn_id
        let fork_action = result
            .actions
            .iter()
            .find(|a| matches!(a, OutputAction::ForkBranch { .. }));
        assert!(fork_action.is_some(), "Expected ForkBranch action");
        if let OutputAction::ForkBranch { from_turn_id, name } = fork_action.unwrap() {
            assert_eq!(*from_turn_id, turn_id);
            assert_eq!(name.as_ref(), Some(&"feature-x".to_string()));
        }
    }

    #[test]
    fn test_continue_from_cursor_creates_new_branch_and_turn() {
        let mut runtime = make_runtime_state();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn_id = TurnId::new();
        runtime.session_id = Some(session_id);
        runtime.branch_id = Some(branch_id);
        runtime.current_turn_id = Some(turn_id);

        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::ContinueFromCursor {
                content: "Continue this".to_string(),
            },
        );

        // Verify both ForkBranch and CreateTurn are emitted
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::ForkBranch { .. })));
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::CreateTurn { .. })));
        // Should also capture workspace snapshot
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::CaptureWorkspaceSnapshot { .. })));
    }

    // =====================================================================
    // Navigation flow tests
    // =====================================================================

    #[test]
    fn test_navigate_backward_steps_parameter() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::NavigateBackward { steps: 5 },
        );
        // Should emit NavigateToTurn action
        let nav_action = result
            .actions
            .iter()
            .find(|a| matches!(a, OutputAction::NavigateToTurn { .. }));
        assert!(
            nav_action.is_some(),
            "Expected NavigateToTurn action for NavigateBackward"
        );
    }

    #[test]
    fn test_navigate_forward_steps_parameter() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::NavigateForward { steps: 3 },
        );
        let nav_action = result
            .actions
            .iter()
            .find(|a| matches!(a, OutputAction::NavigateToTurn { .. }));
        assert!(
            nav_action.is_some(),
            "Expected NavigateToTurn action for NavigateForward"
        );
    }

    // =====================================================================
    // Redo behavior test (turn finalized during streaming)
    // =====================================================================

    #[test]
    fn test_turn_finalized_during_streaming_transitions_to_idle() {
        let turn_id = TurnId::new();
        let runtime = make_runtime_state();
        let result = transition(
            Phase::StreamingAssistant,
            &runtime,
            &InputEvent::TurnFinalized { turn_id },
        );
        assert_eq!(result.next_phase, Phase::Idle);
        // Should finalize the turn
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, OutputAction::FinalizeTurn { .. })));
    }

    // =====================================================================
    // Error handling invariant - no panics on unexpected events
    // =====================================================================

    #[test]
    fn test_no_panic_on_unexpected_assistant_delta_in_awaiting_tool() {
        let turn_id = TurnId::new();
        let runtime = make_runtime_state();
        // This should be handled gracefully (stays in AwaitingToolResult or goes to Error)
        let result = transition(
            Phase::AwaitingToolResult,
            &runtime,
            &InputEvent::AssistantDelta {
                text: "extra".to_string(),
                turn_id,
            },
        );
        // Should not panic - either stays in AwaitingToolResult or goes to Error
        assert!(matches!(
            result.next_phase,
            Phase::AwaitingToolResult | Phase::Error
        ));
    }

    #[test]
    fn test_no_panic_on_unexpected_restore_cancelled_in_streaming() {
        let runtime = make_runtime_state();
        let result = transition(
            Phase::StreamingAssistant,
            &runtime,
            &InputEvent::RestoreCancelled,
        );
        // Should not panic - should go to Error since RestoreCancelled is invalid in StreamingAssistant
        assert_eq!(result.next_phase, Phase::Error);
    }
}
