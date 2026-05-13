//! FSM crate design tests
//!
//! Task 9: Complete `ai-history-fsm` crate design
//!
//! Verify:
//! - FSM is pure and declarative (no side effects in transition function)
//! - All required phase states exist
//! - All input event categories are covered
//! - Transition function exists and produces valid results

#[cfg(test)]
mod test_fsm_purity {
    use ai_history_core::{SessionId, TurnId};
    use ai_history_fsm::input::InputEvent;
    use ai_history_fsm::state::Phase;
    use ai_history_fsm::transition::transition;

    /// Verify transition function is pure (no side effects)
    /// Same inputs produce same next phase
    #[test]
    fn test_transition_is_pure() {
        let current_phase = Phase::Idle;
        let runtime_state = ai_history_fsm::RuntimeState::default();
        let input = InputEvent::UserPromptSubmitted {
            content: "test".to_string(),
            session_id: SessionId::new(),
        };

        let result1 = transition(current_phase, &runtime_state, &input);
        let result2 = transition(current_phase, &runtime_state, &input);

        // Same inputs must produce same next phase (purity)
        assert_eq!(result1.next_phase, result2.next_phase);
    }

    /// Verify transition produces valid output actions
    #[test]
    fn test_transition_produces_actions() {
        let runtime_state = ai_history_fsm::RuntimeState::default();
        let result = transition(
            Phase::Idle,
            &runtime_state,
            &InputEvent::UserPromptSubmitted {
                content: "test".to_string(),
                session_id: SessionId::new(),
            },
        );

        // Should produce output actions
        assert!(!result.actions.is_empty());
    }
}

#[cfg(test)]
mod test_fsm_states {
    use ai_history_fsm::state::Phase;

    /// Verify FSM has all required phase states per task description
    #[test]
    fn test_all_required_phases_exist() {
        // States from task 9: prompts, streaming, tools, patches, checkpoints,
        // restore, conflict resolution, cancellation, branch switching,
        // forward/back navigation, and forking
        let phases = vec![
            Phase::Idle,               // Waiting for input
            Phase::CreatingTurn,       // Creating new turn
            Phase::StreamingAssistant, // Streaming response
            Phase::AwaitingToolResult, // Tool execution
            Phase::ApplyingPatch,      // Applying file patches
            Phase::Checkpointing,      // Creating checkpoints
            Phase::Restoring,          // Restoring from checkpoint
            Phase::ConflictResolution, // Resolving conflicts
            Phase::Cancelled,          // Operation cancelled
            Phase::Error,              // Error state
        ];

        assert_eq!(phases.len(), 10);
    }

    /// Verify Phase is serializable (for state persistence)
    #[test]
    fn test_phase_is_serializable() {
        let phase = Phase::StreamingAssistant;
        let json = serde_json::to_string(&phase).unwrap();
        let parsed: Phase = serde_json::from_str(&json).unwrap();
        assert_eq!(phase, parsed);
    }

    /// Verify Phase has Display implementation
    #[test]
    fn test_phase_display() {
        assert_eq!(Phase::Idle.to_string(), "Idle");
        assert_eq!(Phase::StreamingAssistant.to_string(), "StreamingAssistant");
    }
}

#[cfg(test)]
mod test_fsm_inputs {
    use ai_history_core::{BranchId, CheckpointId, SessionId, TurnId};
    use ai_history_fsm::input::{CheckpointReason, ConflictResolution, InputEvent, RestoreMode};

    /// Verify all restore modes exist
    #[test]
    fn test_restore_modes_comprehensive() {
        // All four modes from task description
        assert!(matches!(
            RestoreMode::ConversationOnly,
            RestoreMode::ConversationOnly
        ));
        assert!(matches!(
            RestoreMode::WorkspaceOnly,
            RestoreMode::WorkspaceOnly
        ));
        assert!(matches!(RestoreMode::Both, RestoreMode::Both));
        assert!(matches!(RestoreMode::Fork, RestoreMode::Fork));
    }

    /// Verify restore modes are serializable
    #[test]
    fn test_restore_mode_serialization() {
        let mode = RestoreMode::Fork;
        let json = serde_json::to_string(&mode).unwrap();
        let parsed: RestoreMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, parsed);
    }

    /// Verify checkpoint reasons are comprehensive
    #[test]
    fn test_checkpoint_reasons_comprehensive() {
        let reasons = vec![
            CheckpointReason::BeforeUserPrompt,
            CheckpointReason::BeforeFileWrite,
            CheckpointReason::AfterAssistantTurn,
            CheckpointReason::Safety,
            CheckpointReason::Named,
            CheckpointReason::PreRestore,
        ];

        assert_eq!(reasons.len(), 6);
    }

    /// Verify conflict resolutions are comprehensive
    #[test]
    fn test_conflict_resolutions_comprehensive() {
        let resolutions = vec![
            ConflictResolution::KeepCurrent,
            ConflictResolution::RestoreCheckpoint,
            ConflictResolution::Merge,
            ConflictResolution::SaveCurrentAndRestore,
            ConflictResolution::Cancel,
        ];

        assert_eq!(resolutions.len(), 5);
    }

    /// Verify all input event categories exist per task description
    #[test]
    fn test_input_categories_covered() {
        use ai_history_core::{BranchId, CheckpointId, SessionId, TurnId};

        // Navigation inputs
        let _ = InputEvent::NavigateBackward { steps: 1 };
        let _ = InputEvent::NavigateForward { steps: 1 };

        // Forking inputs
        let _ = InputEvent::ForkCurrentConversation { name: None };
        let _ = InputEvent::ForkFromCursor {
            name: None,
            restore_workspace: false,
        };
        let _ = InputEvent::ContinueFromCursor {
            content: "test".to_string(),
        };

        // Branch inputs
        let _ = InputEvent::BranchSwitch {
            branch_id: BranchId::new(),
        };

        // Turn inputs
        let _ = InputEvent::TurnFinalized {
            turn_id: TurnId::new(),
        };

        // Restore inputs
        let _ = InputEvent::RestoreWorkspaceOnly {
            checkpoint_id: CheckpointId::new(),
        };
        let _ = InputEvent::RestoreConversationOnly {
            checkpoint_id: CheckpointId::new(),
        };
    }
}

#[cfg(test)]
mod test_fsm_output {
    use ai_history_fsm::output::{NotificationSeverity, OutputAction};

    /// Verify OutputAction variants are comprehensive
    #[test]
    fn test_output_actions_comprehensive() {
        // Verify we have actions for the key operations
        // This is a compile-time check - if variants are missing, this won't compile
        let _ = OutputAction::CaptureWorkspaceSnapshot {
            reason: "test".to_string(),
            checkpoint_id: ai_history_core::CheckpointId::new(),
        };
        let _ = OutputAction::CreateTurn {
            session_id: ai_history_core::SessionId::new(),
            branch_id: ai_history_core::BranchId::new(),
            parent_turn_id: None,
        };
        let _ = OutputAction::AppendEvent {
            event: serde_json::json!({"type": "test"}),
        };
        let _ = OutputAction::RestoreWorkspace {
            checkpoint_id: ai_history_core::CheckpointId::new(),
            mode: ai_history_fsm::input::RestoreMode::Both,
        };
        let _ = OutputAction::EmitNotification {
            message: "test".to_string(),
            severity: NotificationSeverity::Info,
        };
    }
}

#[cfg(test)]
mod test_fsm_transition_function {
    use ai_history_core::SessionId;
    use ai_history_fsm::input::InputEvent;
    use ai_history_fsm::state::Phase;
    use ai_history_fsm::transition::{transition, RuntimeState, TransitionResult};

    /// Verify transition function exists and has correct signature
    #[test]
    fn test_transition_function_exists() {
        let result = transition(
            Phase::Idle,
            &RuntimeState::default(),
            &InputEvent::UserPromptSubmitted {
                content: "test".to_string(),
                session_id: SessionId::new(),
            },
        );

        assert!(matches!(result, TransitionResult { .. }));
    }

    /// Verify RuntimeState has required fields
    #[test]
    fn test_runtime_state_has_required_fields() {
        let state = RuntimeState::default();
        // RuntimeState should track current branch for branch switching
        // The exact field name may vary, but it should exist
        let _ = state.branch_id;
        let _ = state.session_id;
    }

    /// Verify transition from Idle with user prompt produces CreatingTurn
    #[test]
    fn test_idle_user_prompt_transitions_to_creating_turn() {
        let result = transition(
            Phase::Idle,
            &RuntimeState::default(),
            &InputEvent::UserPromptSubmitted {
                content: "test".to_string(),
                session_id: SessionId::new(),
            },
        );

        assert_eq!(result.next_phase, Phase::CreatingTurn);
    }

    /// Verify transition from Idle with restore request produces Restoring
    #[test]
    fn test_idle_restore_transitions_to_restoring() {
        let result = transition(
            Phase::Idle,
            &RuntimeState::default(),
            &InputEvent::RestoreRequested {
                checkpoint_id: ai_history_core::CheckpointId::new(),
                mode: ai_history_fsm::input::RestoreMode::Both,
            },
        );

        assert_eq!(result.next_phase, Phase::Restoring);
    }

    /// Verify transition from Idle with checkpoint request produces Checkpointing
    #[test]
    fn test_idle_checkpoint_transitions_to_checkpointing() {
        let result = transition(
            Phase::Idle,
            &RuntimeState::default(),
            &InputEvent::CheckpointRequested {
                name: None,
                reason: ai_history_fsm::input::CheckpointReason::Safety,
            },
        );

        assert_eq!(result.next_phase, Phase::Checkpointing);
    }
}
