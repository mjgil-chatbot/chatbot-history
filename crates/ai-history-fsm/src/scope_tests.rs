//! FSM scope verification tests
//!
//! Task 7: Verify FSM supports all required workflows:
//! - prompt, stream, tool call, patch, checkpoint, restore
//! - conflict, cancellation, navigation, branch switch
//! - fork, import/export, GC, doctor

use crate::input::{CheckpointReason, ConflictResolution, InputEvent, RestoreMode};
use crate::output::OutputAction;
use crate::state::Phase;
use crate::transition::{transition, RuntimeState, TransitionResult};
use ai_history_core::{BranchId, CheckpointId, SessionId, TurnId};

/// Helper to create runtime state for tests
fn make_runtime() -> RuntimeState {
    RuntimeState {
        phase: Phase::Idle,
        session_id: Some(SessionId::new()),
        branch_id: Some(BranchId::new()),
        current_turn_id: Some(TurnId::new()),
        current_checkpoint_id: None,
    }
}

/// Verify all phases support cancellation
#[test]
fn test_all_phases_support_cancellation() {
    let phases = vec![
        Phase::Idle,
        Phase::CreatingTurn,
        Phase::StreamingAssistant,
        Phase::AwaitingToolResult,
        Phase::ApplyingPatch,
        Phase::Checkpointing,
        Phase::Restoring,
        Phase::ConflictResolution,
        Phase::Cancelled,
        Phase::Error,
    ];

    for phase in phases {
        let runtime = make_runtime();
        let result = transition(phase, &runtime, &InputEvent::CancellationRequested);
        // All phases should either go to Cancelled or stay in Error
        assert!(
            matches!(result.next_phase, Phase::Cancelled | Phase::Error),
            "Phase {:?} should handle cancellation gracefully",
            phase
        );
    }
}

/// Verify prompt workflow: idle -> creatingturn -> streaming -> idle
#[test]
fn test_prompt_workflow() {
    let runtime = make_runtime();
    let session_id = SessionId::new();

    // Start with user prompt
    let result = transition(
        Phase::Idle,
        &runtime,
        &InputEvent::UserPromptSubmitted {
            content: "Hello".to_string(),
            session_id,
        },
    );
    assert_eq!(result.next_phase, Phase::CreatingTurn);
    assert!(result
        .actions
        .iter()
        .any(|a| matches!(a, OutputAction::CreateTurn { .. })));

    let turn_id = runtime.current_turn_id.unwrap();

    // Assistant starts streaming
    let result = transition(
        Phase::CreatingTurn,
        &runtime,
        &InputEvent::AssistantDelta {
            text: "Hi there!".to_string(),
            turn_id,
        },
    );
    assert_eq!(result.next_phase, Phase::StreamingAssistant);

    // Assistant finishes
    let result = transition(
        Phase::StreamingAssistant,
        &runtime,
        &InputEvent::AssistantDone { turn_id },
    );
    assert_eq!(result.next_phase, Phase::Idle);
}

/// Verify tool call workflow: streaming -> awaiting -> streaming
#[test]
fn test_tool_call_workflow() {
    let runtime = make_runtime();
    let turn_id = runtime.current_turn_id.unwrap();

    // Tool call requested during streaming
    let result = transition(
        Phase::StreamingAssistant,
        &runtime,
        &InputEvent::ToolCallRequested {
            tool_name: "write_file".to_string(),
            args: serde_json::json!({"path": "test.rs", "content": "fn main() {}"}),
            turn_id,
        },
    );
    assert_eq!(result.next_phase, Phase::AwaitingToolResult);
    assert!(result
        .actions
        .iter()
        .any(|a| matches!(a, OutputAction::BeginToolPolling { .. })));

    // Tool completes
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

/// Verify patch workflow: streaming -> applying -> streaming
#[test]
fn test_patch_workflow() {
    let runtime = make_runtime();
    let turn_id = runtime.current_turn_id.unwrap();

    // File patch proposed
    let result = transition(
        Phase::StreamingAssistant,
        &runtime,
        &InputEvent::FilePatchProposed {
            patch_id: "patch-123".to_string(),
            file_path: "src/main.rs".to_string(),
        },
    );
    assert_eq!(result.next_phase, Phase::ApplyingPatch);
    assert!(result
        .actions
        .iter()
        .any(|a| matches!(a, OutputAction::CaptureWorkspaceSnapshot { .. })));

    // Patch applied
    let result = transition(
        Phase::ApplyingPatch,
        &runtime,
        &InputEvent::FilePatchApplied {
            patch_id: "patch-123".to_string(),
        },
    );
    assert_eq!(result.next_phase, Phase::StreamingAssistant);

    // Patch rejected
    let result = transition(
        Phase::ApplyingPatch,
        &runtime,
        &InputEvent::FilePatchRejected {
            patch_id: "patch-124".to_string(),
            reason: "syntax error".to_string(),
        },
    );
    assert_eq!(result.next_phase, Phase::StreamingAssistant);
}

/// Verify checkpoint workflow: idle -> checkpointing -> idle
#[test]
fn test_checkpoint_workflow() {
    let runtime = make_runtime();

    // Checkpoint requested
    let result = transition(
        Phase::Idle,
        &runtime,
        &InputEvent::CheckpointRequested {
            name: Some("my-checkpoint".to_string()),
            reason: CheckpointReason::Named,
        },
    );
    assert_eq!(result.next_phase, Phase::Checkpointing);
    assert!(result
        .actions
        .iter()
        .any(|a| matches!(a, OutputAction::CaptureWorkspaceSnapshot { .. })));

    // Checkpoint created
    let checkpoint_id = CheckpointId::new();
    let result = transition(
        Phase::Checkpointing,
        &runtime,
        &InputEvent::CheckpointCreated { checkpoint_id },
    );
    assert_eq!(result.next_phase, Phase::Idle);
}

/// Verify restore workflow: idle -> restoring -> idle
#[test]
fn test_restore_workflow() {
    let runtime = make_runtime();
    let checkpoint_id = CheckpointId::new();

    // Restore requested (triggers safety checkpoint first)
    let result = transition(
        Phase::Idle,
        &runtime,
        &InputEvent::RestoreRequested {
            checkpoint_id,
            mode: RestoreMode::Both,
        },
    );
    assert_eq!(result.next_phase, Phase::Restoring);
    assert!(result
        .actions
        .iter()
        .any(|a| matches!(a, OutputAction::CaptureWorkspaceSnapshot { .. })));
    assert!(result
        .actions
        .iter()
        .any(|a| matches!(a, OutputAction::RestoreWorkspace { .. })));

    // Restore completed
    let result = transition(Phase::Restoring, &runtime, &InputEvent::RestoreCompleted);
    assert_eq!(result.next_phase, Phase::Idle);

    // Restore cancelled
    let result = transition(Phase::Restoring, &runtime, &InputEvent::RestoreCancelled);
    assert_eq!(result.next_phase, Phase::Idle);
}

/// Verify conflict resolution workflow
#[test]
fn test_conflict_resolution_workflow() {
    let runtime = make_runtime();

    // Conflict detected during restore
    let result = transition(
        Phase::Restoring,
        &runtime,
        &InputEvent::WorkspaceConflictDetected {
            files: vec!["file1.rs".to_string(), "file2.rs".to_string()],
        },
    );
    assert_eq!(result.next_phase, Phase::ConflictResolution);
    assert!(result
        .actions
        .iter()
        .any(|a| matches!(a, OutputAction::ShowConflictUI { .. })));

    // Conflict resolved by keeping current (resumes restore)
    let result = transition(
        Phase::ConflictResolution,
        &runtime,
        &InputEvent::ConflictResolved {
            resolution: ConflictResolution::KeepCurrent,
        },
    );
    assert_eq!(result.next_phase, Phase::Restoring);

    // Conflict resolved by restoring checkpoint
    let result = transition(
        Phase::ConflictResolution,
        &runtime,
        &InputEvent::ConflictResolved {
            resolution: ConflictResolution::RestoreCheckpoint,
        },
    );
    assert_eq!(result.next_phase, Phase::Restoring);

    // Conflict resolved by merging
    let result = transition(
        Phase::ConflictResolution,
        &runtime,
        &InputEvent::ConflictResolved {
            resolution: ConflictResolution::Merge,
        },
    );
    assert_eq!(result.next_phase, Phase::Restoring);

    // Conflict resolved by cancelling
    let result = transition(
        Phase::ConflictResolution,
        &runtime,
        &InputEvent::ConflictResolved {
            resolution: ConflictResolution::Cancel,
        },
    );
    assert_eq!(result.next_phase, Phase::Idle);
}

/// Verify navigation workflow
#[test]
fn test_navigation_workflow() {
    let runtime = make_runtime();

    // Navigate backward
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

    // Navigate forward
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

/// Verify branch switch workflow
#[test]
fn test_branch_switch_workflow() {
    let runtime = make_runtime();
    let branch_id = BranchId::new();

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

/// Verify fork workflow: fork-current-conversation, fork-from-cursor, continue-from-cursor
#[test]
fn test_fork_workflow() {
    let mut runtime = make_runtime();
    runtime.branch_id = Some(BranchId::new());
    runtime.current_turn_id = Some(TurnId::new());

    // Fork current conversation
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

    // Fork from cursor (no workspace restore)
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

    // Fork from cursor (with workspace restore)
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

    // Continue from cursor (creates new turn on fork)
    let result = transition(
        Phase::Idle,
        &runtime,
        &InputEvent::ContinueFromCursor {
            content: "Continue this".to_string(),
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

/// Verify restore-only modes (workspace-only, conversation-only)
#[test]
fn test_restore_modes_workflow() {
    let runtime = make_runtime();
    let checkpoint_id = CheckpointId::new();

    // Restore workspace only
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

    // Restore conversation only
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

/// Verify turn finalization workflow
#[test]
fn test_turn_finalization_workflow() {
    let runtime = make_runtime();
    let turn_id = runtime.current_turn_id.unwrap();

    // Finalize during creating turn
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

    // Finalize during streaming
    let result = transition(
        Phase::StreamingAssistant,
        &runtime,
        &InputEvent::TurnFinalized { turn_id },
    );
    assert_eq!(result.next_phase, Phase::Idle);
    assert!(result
        .actions
        .iter()
        .any(|a| matches!(a, OutputAction::FinalizeTurn { .. })));
}

/// Verify error recovery
#[test]
fn test_error_recovery_workflow() {
    let runtime = make_runtime();

    // Recoverable error in idle
    let result = transition(
        Phase::Idle,
        &runtime,
        &InputEvent::Error {
            error: "temp failure".to_string(),
            recoverable: true,
        },
    );
    assert_eq!(result.next_phase, Phase::Error);

    // User can continue after error
    let result = transition(
        Phase::Error,
        &runtime,
        &InputEvent::UserPromptSubmitted {
            content: "retry".to_string(),
            session_id: SessionId::new(),
        },
    );
    assert_eq!(result.next_phase, Phase::CreatingTurn);
}

/// Verify cancelled state can recover
#[test]
fn test_cancelled_recovery_workflow() {
    let runtime = make_runtime();

    // Cancellation
    let result = transition(Phase::Idle, &runtime, &InputEvent::CancellationRequested);
    assert_eq!(result.next_phase, Phase::Cancelled);

    // User can continue after cancellation
    let result = transition(
        Phase::Cancelled,
        &runtime,
        &InputEvent::UserPromptSubmitted {
            content: "continue".to_string(),
            session_id: SessionId::new(),
        },
    );
    assert_eq!(result.next_phase, Phase::CreatingTurn);
}

/// Verify all states have valid transitions (exhaustive test)
#[test]
fn test_exhaustive_state_coverage() {
    let phases = vec![
        Phase::Idle,
        Phase::CreatingTurn,
        Phase::StreamingAssistant,
        Phase::AwaitingToolResult,
        Phase::ApplyingPatch,
        Phase::Checkpointing,
        Phase::Restoring,
        Phase::ConflictResolution,
        Phase::Cancelled,
        Phase::Error,
    ];

    // Key events to test from each state
    let events = vec![
        InputEvent::UserPromptSubmitted {
            content: "test".to_string(),
            session_id: SessionId::new(),
        },
        InputEvent::CancellationRequested,
        InputEvent::Error {
            error: "test".to_string(),
            recoverable: true,
        },
        InputEvent::AssistantDone {
            turn_id: TurnId::new(),
        },
        InputEvent::TurnFinalized {
            turn_id: TurnId::new(),
        },
    ];

    for phase in phases {
        for event in &events {
            let runtime = make_runtime();
            let result = transition(phase, &runtime, event);
            // Verify transition produces a valid result (not panics)
            assert!(
                matches!(
                    result.next_phase,
                    Phase::Idle
                        | Phase::CreatingTurn
                        | Phase::StreamingAssistant
                        | Phase::AwaitingToolResult
                        | Phase::ApplyingPatch
                        | Phase::Checkpointing
                        | Phase::Restoring
                        | Phase::ConflictResolution
                        | Phase::Cancelled
                        | Phase::Error
                ),
                "Invalid next_phase {:?} for {:?} + {:?}",
                result.next_phase,
                phase,
                event
            );
        }
    }
}

/// Verify all restore modes are handled
#[test]
fn test_restore_modes_complete() {
    let runtime = make_runtime();
    let checkpoint_id = CheckpointId::new();

    let modes = vec![
        RestoreMode::Both,
        RestoreMode::ConversationOnly,
        RestoreMode::WorkspaceOnly,
        RestoreMode::Fork,
    ];

    for mode in modes {
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::RestoreRequested {
                checkpoint_id,
                mode,
            },
        );
        assert_eq!(result.next_phase, Phase::Restoring);
    }
}

/// Verify all conflict resolutions are handled
#[test]
fn test_conflict_resolutions_complete() {
    let runtime = make_runtime();

    let resolutions = vec![
        ConflictResolution::KeepCurrent,
        ConflictResolution::RestoreCheckpoint,
        ConflictResolution::Merge,
        ConflictResolution::SaveCurrentAndRestore,
        ConflictResolution::Cancel,
    ];

    // First enter conflict resolution state
    let result = transition(
        Phase::Restoring,
        &runtime,
        &InputEvent::WorkspaceConflictDetected {
            files: vec!["file.rs".to_string()],
        },
    );
    assert_eq!(result.next_phase, Phase::ConflictResolution);

    for resolution in resolutions {
        let result = transition(
            Phase::ConflictResolution,
            &runtime,
            &InputEvent::ConflictResolved { resolution },
        );
        // All resolutions should produce a valid next phase
        assert!(
            matches!(result.next_phase, Phase::Idle | Phase::Restoring),
            "Invalid resolution {:?} -> {:?}",
            resolution,
            result.next_phase
        );
    }
}

/// Verify checkpoint reasons are handled
#[test]
fn test_checkpoint_reasons_complete() {
    let runtime = make_runtime();

    let reasons = vec![
        CheckpointReason::BeforeUserPrompt,
        CheckpointReason::BeforeFileWrite,
        CheckpointReason::AfterAssistantTurn,
        CheckpointReason::Safety,
        CheckpointReason::Named,
        CheckpointReason::PreRestore,
    ];

    for reason in reasons {
        let result = transition(
            Phase::Idle,
            &runtime,
            &InputEvent::CheckpointRequested { name: None, reason },
        );
        assert_eq!(result.next_phase, Phase::Checkpointing);
    }
}
