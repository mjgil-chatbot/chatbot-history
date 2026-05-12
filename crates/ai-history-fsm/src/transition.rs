//! Finite-state transducer transition function
//!
//! Pure transition functions with no side effects.
//! Maps (current_phase, input_event) -> (next_phase, output_actions)

use crate::input::InputEvent;
use crate::output::{NotificationSeverity, OutputAction};
use crate::state::Phase;
use ai_history_core::{BranchId, CheckpointId};
/// Transition result containing next state and actions
#[derive(Debug, Clone, PartialEq)]
pub struct TransitionResult {
    pub next_phase: Phase,
    pub actions: Vec<OutputAction>,
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
