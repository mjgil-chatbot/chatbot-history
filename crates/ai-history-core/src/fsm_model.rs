//! Finite-state interaction controller
//!
//! Extended finite-state machine with phases:
//! - Idle, CreatingTurn, StreamingAssistant, AwaitingToolResult
//! - ApplyingPatch, Checkpointing, Restoring, ConflictResolution
//! - Cancelled, Error

use serde::{Deserialize, Serialize};

/// Phase in the interaction state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionPhase {
    /// Initial idle state, ready for user input
    Idle,
    /// Creating a new turn
    CreatingTurn,
    /// Assistant is streaming response
    StreamingAssistant,
    /// Waiting for tool result
    AwaitingToolResult,
    /// Applying a patch to workspace
    ApplyingPatch,
    /// Creating or capturing checkpoint
    Checkpointing,
    /// Restore operation in progress
    Restoring,
    /// Conflict resolution in progress
    ConflictResolution,
    /// Operation was cancelled
    Cancelled,
    /// Error occurred
    Error,
}

impl Default for InteractionPhase {
    fn default() -> Self {
        Self::Idle
    }
}

impl InteractionPhase {
    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Cancelled | Self::Error)
    }

    /// Check if this is an active processing state
    pub fn is_active(&self) -> bool {
        !matches!(
            self,
            Self::Idle | Self::Cancelled | Self::Error | Self::ConflictResolution
        )
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::CreatingTurn => "Creating Turn",
            Self::StreamingAssistant => "Streaming",
            Self::AwaitingToolResult => "Awaiting Tool",
            Self::ApplyingPatch => "Applying Patch",
            Self::Checkpointing => "Checkpointing",
            Self::Restoring => "Restoring",
            Self::ConflictResolution => "Conflict Resolution",
            Self::Cancelled => "Cancelled",
            Self::Error => "Error",
        }
    }
}

/// Input event to the state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FsmInput {
    /// User sent a prompt
    UserPrompt,
    /// Assistant sent a delta
    AssistantDelta,
    /// Assistant completed message
    AssistantDone,
    /// Tool call was requested
    ToolCallRequested,
    /// Tool result received
    ToolResultReceived,
    /// Patch was proposed
    PatchProposed,
    /// Patch was applied
    PatchApplied,
    /// Checkpoint requested
    CheckpointRequested,
    /// Restore requested
    RestoreRequested,
    /// Conflict detected
    ConflictDetected,
    /// Cancellation requested
    CancellationRequested,
    /// Error occurred
    ErrorOccurred,
}

impl FsmInput {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::UserPrompt => "User Prompt",
            Self::AssistantDelta => "Assistant Delta",
            Self::AssistantDone => "Assistant Done",
            Self::ToolCallRequested => "Tool Call Requested",
            Self::ToolResultReceived => "Tool Result Received",
            Self::PatchProposed => "Patch Proposed",
            Self::PatchApplied => "Patch Applied",
            Self::CheckpointRequested => "Checkpoint Requested",
            Self::RestoreRequested => "Restore Requested",
            Self::ConflictDetected => "Conflict Detected",
            Self::CancellationRequested => "Cancellation Requested",
            Self::ErrorOccurred => "Error Occurred",
        }
    }
}

/// Output command from the state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FsmOutput {
    /// Create a new turn
    CreateTurn,
    /// Append user message
    AppendUserMessage,
    /// Append assistant delta
    AppendAssistantDelta,
    /// Finalize assistant message
    FinalizeAssistantMessage,
    /// Capture pre-write checkpoint
    CapturePreWriteCheckpoint,
    /// Validate patch
    ValidatePatch,
    /// Apply patch to workspace
    ApplyPatch,
    /// Capture post-write snapshot
    CapturePostWriteSnapshot,
    /// Create checkpoint
    CreateCheckpoint,
    /// Capture safety snapshot
    CaptureSafetySnapshot,
    /// Restore workspace snapshot
    RestoreWorkspaceSnapshot,
    /// Show conflict UI
    ShowConflictUI,
    /// Emit UI notification
    EmitNotification,
    /// Log error
    LogError,
}

impl FsmOutput {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CreateTurn => "Create Turn",
            Self::AppendUserMessage => "Append User Message",
            Self::AppendAssistantDelta => "Append Assistant Delta",
            Self::FinalizeAssistantMessage => "Finalize Assistant Message",
            Self::CapturePreWriteCheckpoint => "Capture Pre-Write Checkpoint",
            Self::ValidatePatch => "Validate Patch",
            Self::ApplyPatch => "Apply Patch",
            Self::CapturePostWriteSnapshot => "Capture Post-Write Snapshot",
            Self::CreateCheckpoint => "Create Checkpoint",
            Self::CaptureSafetySnapshot => "Capture Safety Snapshot",
            Self::RestoreWorkspaceSnapshot => "Restore Workspace Snapshot",
            Self::ShowConflictUI => "Show Conflict UI",
            Self::EmitNotification => "Emit Notification",
            Self::LogError => "Log Error",
        }
    }
}

/// State transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from_phase: InteractionPhase,
    pub input: FsmInput,
    pub to_phase: InteractionPhase,
    pub outputs: Vec<FsmOutput>,
}

/// Transition table for the FSM
pub struct TransitionTable {
    transitions: Vec<Transition>,
}

impl Default for TransitionTable {
    fn default() -> Self {
        Self::new()
    }
}

impl TransitionTable {
    pub fn new() -> Self {
        Self {
            transitions: Self::build_transitions(),
        }
    }

    fn build_transitions() -> Vec<Transition> {
        vec![
            // Idle state transitions
            Transition {
                from_phase: InteractionPhase::Idle,
                input: FsmInput::UserPrompt,
                to_phase: InteractionPhase::CreatingTurn,
                outputs: vec![
                    FsmOutput::CreateTurn,
                    FsmOutput::CapturePreWriteCheckpoint,
                    FsmOutput::AppendUserMessage,
                ],
            },
            Transition {
                from_phase: InteractionPhase::Idle,
                input: FsmInput::RestoreRequested,
                to_phase: InteractionPhase::Restoring,
                outputs: vec![FsmOutput::CaptureSafetySnapshot],
            },
            // CreatingTurn transitions
            Transition {
                from_phase: InteractionPhase::CreatingTurn,
                input: FsmInput::AssistantDelta,
                to_phase: InteractionPhase::StreamingAssistant,
                outputs: vec![FsmOutput::AppendAssistantDelta],
            },
            Transition {
                from_phase: InteractionPhase::CreatingTurn,
                input: FsmInput::ToolCallRequested,
                to_phase: InteractionPhase::AwaitingToolResult,
                outputs: vec![FsmOutput::CapturePreWriteCheckpoint],
            },
            // StreamingAssistant transitions
            Transition {
                from_phase: InteractionPhase::StreamingAssistant,
                input: FsmInput::AssistantDelta,
                to_phase: InteractionPhase::StreamingAssistant,
                outputs: vec![FsmOutput::AppendAssistantDelta],
            },
            Transition {
                from_phase: InteractionPhase::StreamingAssistant,
                input: FsmInput::AssistantDone,
                to_phase: InteractionPhase::Idle,
                outputs: vec![
                    FsmOutput::FinalizeAssistantMessage,
                    FsmOutput::CreateCheckpoint,
                ],
            },
            Transition {
                from_phase: InteractionPhase::StreamingAssistant,
                input: FsmInput::ToolCallRequested,
                to_phase: InteractionPhase::AwaitingToolResult,
                outputs: vec![FsmOutput::CapturePreWriteCheckpoint],
            },
            Transition {
                from_phase: InteractionPhase::StreamingAssistant,
                input: FsmInput::CancellationRequested,
                to_phase: InteractionPhase::Cancelled,
                outputs: vec![],
            },
            // AwaitingToolResult transitions
            Transition {
                from_phase: InteractionPhase::AwaitingToolResult,
                input: FsmInput::ToolResultReceived,
                to_phase: InteractionPhase::StreamingAssistant,
                outputs: vec![FsmOutput::AppendAssistantDelta],
            },
            Transition {
                from_phase: InteractionPhase::AwaitingToolResult,
                input: FsmInput::CancellationRequested,
                to_phase: InteractionPhase::Cancelled,
                outputs: vec![],
            },
            // ApplyingPatch transitions
            Transition {
                from_phase: InteractionPhase::Idle,
                input: FsmInput::PatchProposed,
                to_phase: InteractionPhase::ApplyingPatch,
                outputs: vec![
                    FsmOutput::CapturePreWriteCheckpoint,
                    FsmOutput::ValidatePatch,
                ],
            },
            Transition {
                from_phase: InteractionPhase::ApplyingPatch,
                input: FsmInput::PatchApplied,
                to_phase: InteractionPhase::Idle,
                outputs: vec![FsmOutput::ApplyPatch, FsmOutput::CapturePostWriteSnapshot],
            },
            Transition {
                from_phase: InteractionPhase::ApplyingPatch,
                input: FsmInput::CancellationRequested,
                to_phase: InteractionPhase::Cancelled,
                outputs: vec![],
            },
            // Restoring transitions
            Transition {
                from_phase: InteractionPhase::Restoring,
                input: FsmInput::ConflictDetected,
                to_phase: InteractionPhase::ConflictResolution,
                outputs: vec![FsmOutput::ShowConflictUI],
            },
            Transition {
                from_phase: InteractionPhase::Restoring,
                input: FsmInput::CheckpointRequested,
                to_phase: InteractionPhase::Checkpointing,
                outputs: vec![FsmOutput::CreateCheckpoint],
            },
            Transition {
                from_phase: InteractionPhase::Restoring,
                input: FsmInput::AssistantDone,
                to_phase: InteractionPhase::Idle,
                outputs: vec![FsmOutput::RestoreWorkspaceSnapshot],
            },
            // ConflictResolution transitions
            Transition {
                from_phase: InteractionPhase::ConflictResolution,
                input: FsmInput::PatchApplied,
                to_phase: InteractionPhase::Restoring,
                outputs: vec![FsmOutput::ApplyPatch],
            },
            // Checkpointing transitions
            Transition {
                from_phase: InteractionPhase::Checkpointing,
                input: FsmInput::CheckpointRequested,
                to_phase: InteractionPhase::Idle,
                outputs: vec![FsmOutput::CreateCheckpoint],
            },
            // Error handling
            Transition {
                from_phase: InteractionPhase::StreamingAssistant,
                input: FsmInput::ErrorOccurred,
                to_phase: InteractionPhase::Error,
                outputs: vec![FsmOutput::LogError],
            },
            Transition {
                from_phase: InteractionPhase::AwaitingToolResult,
                input: FsmInput::ErrorOccurred,
                to_phase: InteractionPhase::Error,
                outputs: vec![FsmOutput::LogError],
            },
            // Cancelled can return to Idle
            Transition {
                from_phase: InteractionPhase::Cancelled,
                input: FsmInput::UserPrompt,
                to_phase: InteractionPhase::CreatingTurn,
                outputs: vec![FsmOutput::CreateTurn, FsmOutput::AppendUserMessage],
            },
        ]
    }

    /// Get next state for a given state and input
    pub fn next_state(
        &self,
        current_phase: InteractionPhase,
        input: FsmInput,
    ) -> Option<Transition> {
        self.transitions
            .iter()
            .find(|t| t.from_phase == current_phase && t.input == input)
            .cloned()
    }

    /// Get all valid inputs for a phase
    pub fn valid_inputs(&self, phase: InteractionPhase) -> Vec<FsmInput> {
        self.transitions
            .iter()
            .filter(|t| t.from_phase == phase)
            .map(|t| t.input)
            .collect()
    }

    /// Get all transitions from a phase
    pub fn transitions_from(&self, phase: InteractionPhase) -> Vec<&Transition> {
        self.transitions
            .iter()
            .filter(|t| t.from_phase == phase)
            .collect()
    }
}

/// FSM state machine
#[derive(Debug, Clone)]
pub struct InteractionFsm {
    pub current_phase: InteractionPhase,
    pub previous_phase: Option<InteractionPhase>,
    pub transition_count: usize,
}

impl Default for InteractionFsm {
    fn default() -> Self {
        Self::new()
    }
}

impl InteractionFsm {
    pub fn new() -> Self {
        Self {
            current_phase: InteractionPhase::Idle,
            previous_phase: None,
            transition_count: 0,
        }
    }

    /// Process an input and get the resulting outputs
    pub fn process_input(
        &mut self,
        input: FsmInput,
        transition_table: &TransitionTable,
    ) -> ProcessResult {
        if let Some(transition) = transition_table.next_state(self.current_phase, input) {
            self.previous_phase = Some(self.current_phase);
            self.current_phase = transition.to_phase;
            self.transition_count += 1;

            ProcessResult {
                accepted: true,
                transition: Some(transition),
                error_message: None,
            }
        } else {
            ProcessResult {
                accepted: false,
                transition: None,
                error_message: Some(format!(
                    "Invalid transition: {:?} + {:?}",
                    self.current_phase, input
                )),
            }
        }
    }

    /// Reset to idle state
    pub fn reset(&mut self) {
        self.previous_phase = Some(self.current_phase);
        self.current_phase = InteractionPhase::Idle;
        self.transition_count += 1;
    }

    /// Check if in a valid state to accept user input
    pub fn can_accept_user_input(&self) -> bool {
        matches!(self.current_phase, InteractionPhase::Idle)
    }
}

/// Result of processing an input
#[derive(Debug, Clone)]
pub struct ProcessResult {
    pub accepted: bool,
    pub transition: Option<Transition>,
    pub error_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_phase_default() {
        assert_eq!(InteractionPhase::default(), InteractionPhase::Idle);
    }

    #[test]
    fn test_interaction_phase_is_terminal() {
        assert!(InteractionPhase::Cancelled.is_terminal());
        assert!(InteractionPhase::Error.is_terminal());
        assert!(!InteractionPhase::Idle.is_terminal());
        assert!(!InteractionPhase::StreamingAssistant.is_terminal());
    }

    #[test]
    fn test_interaction_phase_is_active() {
        assert!(InteractionPhase::StreamingAssistant.is_active());
        assert!(InteractionPhase::CreatingTurn.is_active());
        assert!(!InteractionPhase::Idle.is_active());
        assert!(!InteractionPhase::ConflictResolution.is_active());
    }

    #[test]
    fn test_transition_table_has_transitions() {
        let table = TransitionTable::new();
        assert!(!table.transitions.is_empty());
    }

    #[test]
    fn test_transition_table_next_state() {
        let table = TransitionTable::new();
        let result = table.next_state(InteractionPhase::Idle, FsmInput::UserPrompt);
        assert!(result.is_some());
        let t = result.unwrap();
        assert_eq!(t.to_phase, InteractionPhase::CreatingTurn);
    }

    #[test]
    fn test_transition_table_invalid_transition() {
        let table = TransitionTable::new();
        let result = table.next_state(InteractionPhase::Idle, FsmInput::AssistantDone);
        assert!(result.is_none());
    }

    #[test]
    fn test_transition_table_valid_inputs() {
        let table = TransitionTable::new();
        let inputs = table.valid_inputs(InteractionPhase::Idle);
        assert!(inputs.contains(&FsmInput::UserPrompt));
        assert!(inputs.contains(&FsmInput::RestoreRequested));
    }

    #[test]
    fn test_transition_table_transitions_from() {
        let table = TransitionTable::new();
        let transitions = table.transitions_from(InteractionPhase::StreamingAssistant);
        assert!(!transitions.is_empty());
    }

    #[test]
    fn test_interaction_fsm_new() {
        let fsm = InteractionFsm::new();
        assert_eq!(fsm.current_phase, InteractionPhase::Idle);
        assert!(fsm.previous_phase.is_none());
        assert_eq!(fsm.transition_count, 0);
    }

    #[test]
    fn test_interaction_fsm_process_valid_input() {
        let mut fsm = InteractionFsm::new();
        let table = TransitionTable::new();
        let result = fsm.process_input(FsmInput::UserPrompt, &table);
        assert!(result.accepted);
        assert!(result.transition.is_some());
        assert_eq!(fsm.current_phase, InteractionPhase::CreatingTurn);
    }

    #[test]
    fn test_interaction_fsm_process_invalid_input() {
        let mut fsm = InteractionFsm::new();
        let table = TransitionTable::new();
        let result = fsm.process_input(FsmInput::AssistantDone, &table);
        assert!(!result.accepted);
        assert!(result.error_message.is_some());
    }

    #[test]
    fn test_interaction_fsm_reset() {
        let mut fsm = InteractionFsm::new();
        let table = TransitionTable::new();
        fsm.process_input(FsmInput::UserPrompt, &table);
        assert_eq!(fsm.current_phase, InteractionPhase::CreatingTurn);
        fsm.reset();
        assert_eq!(fsm.current_phase, InteractionPhase::Idle);
    }

    #[test]
    fn test_interaction_fsm_can_accept_user_input() {
        let fsm = InteractionFsm::new();
        assert!(fsm.can_accept_user_input());
    }

    #[test]
    fn test_interaction_fsm_cannot_accept_during_streaming() {
        let mut fsm = InteractionFsm::new();
        let table = TransitionTable::new();
        fsm.process_input(FsmInput::UserPrompt, &table);
        assert!(!fsm.can_accept_user_input());
    }

    #[test]
    fn test_fsm_input_display_name() {
        assert_eq!(FsmInput::UserPrompt.display_name(), "User Prompt");
        assert_eq!(FsmInput::AssistantDelta.display_name(), "Assistant Delta");
    }

    #[test]
    fn test_fsm_output_display_name() {
        assert_eq!(FsmOutput::CreateTurn.display_name(), "Create Turn");
        assert_eq!(
            FsmOutput::AppendAssistantDelta.display_name(),
            "Append Assistant Delta"
        );
    }

    #[test]
    fn test_interaction_phase_display_name() {
        assert_eq!(InteractionPhase::Idle.display_name(), "Idle");
        assert_eq!(
            InteractionPhase::StreamingAssistant.display_name(),
            "Streaming"
        );
    }

    #[test]
    fn test_transition_serialization() {
        let transition = Transition {
            from_phase: InteractionPhase::Idle,
            input: FsmInput::UserPrompt,
            to_phase: InteractionPhase::CreatingTurn,
            outputs: vec![FsmOutput::CreateTurn],
        };
        let json = serde_json::to_string(&transition).unwrap();
        assert!(json.contains("Idle"));
    }

    #[test]
    fn test_all_phases_have_transitions() {
        let table = TransitionTable::new();
        for phase in [
            InteractionPhase::Idle,
            InteractionPhase::CreatingTurn,
            InteractionPhase::StreamingAssistant,
            InteractionPhase::AwaitingToolResult,
            InteractionPhase::ApplyingPatch,
            InteractionPhase::Restoring,
            InteractionPhase::ConflictResolution,
            InteractionPhase::Cancelled,
            InteractionPhase::Error,
        ] {
            let transitions = table.transitions_from(phase);
            // Every phase should have at least one outgoing transition (except maybe Error)
            // or can be reached from another phase
            let has_incoming = table.transitions.iter().any(|t| t.to_phase == phase);
            assert!(
                !transitions.is_empty() || has_incoming || phase == InteractionPhase::Error,
                "Phase {:?} should have transitions or be reachable",
                phase
            );
        }
    }

    #[test]
    fn test_streaming_to_done() {
        let mut fsm = InteractionFsm::new();
        let table = TransitionTable::new();

        // Idle -> UserPrompt -> CreatingTurn
        fsm.process_input(FsmInput::UserPrompt, &table);
        assert_eq!(fsm.current_phase, InteractionPhase::CreatingTurn);

        // CreatingTurn -> StreamingAssistant
        fsm.process_input(FsmInput::AssistantDelta, &table);
        assert_eq!(fsm.current_phase, InteractionPhase::StreamingAssistant);

        // StreamingAssistant -> Idle (done)
        let result = fsm.process_input(FsmInput::AssistantDone, &table);
        assert!(result.accepted);
        assert_eq!(fsm.current_phase, InteractionPhase::Idle);
    }

    #[test]
    fn test_restore_with_conflict() {
        let mut fsm = InteractionFsm::new();
        let table = TransitionTable::new();

        // Idle -> RestoreRequested -> Restoring
        fsm.process_input(FsmInput::RestoreRequested, &table);
        assert_eq!(fsm.current_phase, InteractionPhase::Restoring);

        // Restoring -> ConflictDetected -> ConflictResolution
        let result = fsm.process_input(FsmInput::ConflictDetected, &table);
        assert!(result.accepted);
        assert_eq!(fsm.current_phase, InteractionPhase::ConflictResolution);
    }
}
