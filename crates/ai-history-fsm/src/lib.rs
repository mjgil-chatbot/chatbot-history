// AI History FSM - Finite-state transducer for interaction lifecycle
//
// This crate implements the extended finite-state machine that controls
// the lifecycle of AI interactions. It maps:
//   current state + input event -> next state + output actions
//
// ## Design Principles
//
// This FSM is **pure** - it contains no side effects, filesystem access,
// database calls, or network operations. All output is declarative:
//   - Output actions describe *what* should happen, not *how* to do it
//   - An external orchestrator layer is responsible for executing actions
//
// ## Required Phases (States)
//
// - Idle
// - CreatingTurn
// - StreamingAssistant
// - AwaitingToolResult
// - ApplyingPatch
// - Checkpointing
// - Restoring
// - ConflictResolution
// - Cancelled
// - Error
//
// ## Invariant Enforcement
//
// The FSM enforces operation invariants through its transition rules:
//
// 1. **Safety checkpoint before restore**: Any restore operation captures a
//    safety checkpoint before proceeding, ensuring rollback capability.
//
// 2. **Checkpoint capture for destructive operations**: File patches capture
//    a workspace snapshot before applying changes.
//
// 3. **Invalid transition rejection**: Events that are invalid in a given
//    state transition to the Error phase rather than being silently ignored.
//
// 4. **Turn finalization immutability**: Turns can be explicitly finalized,
//    after which no further deltas are accepted in that turn's context.
//
// ## Checkpoint Trigger Wiring
//
// Checkpoint creation and restore flows are wired through public APIs:
//
// - `InputEvent::CheckpointRequested` triggers `CaptureWorkspaceSnapshot` action
// - `InputEvent::RestoreRequested` triggers pre-restore safety checkpoint
// - `InputEvent::RestoreWorkspaceOnly` / `RestoreConversationOnly` handle
//   partial restore modes with appropriate safety measures
// - `InputEvent::CheckpointCreated` / `RestoreCompleted` complete the flows
//
// ## Restore Safety Requirements
//
// The FSM ensures restore safety by:
// 1. Capturing a safety checkpoint before any restore operation
// 2. Supporting `RestoreMode::WorkspaceOnly`, `ConversationOnly`, and `Both`
// 3. Transitioning to `ConflictResolution` when workspace conflicts are detected
// 4. Providing cancellation paths for restores that cannot complete safely
//
// ## Usage
//
// ```rust
// use ai_history_fsm::{Phase, RuntimeState, InputEvent, transition};
//
// let phase = Phase::Idle;
// let runtime = RuntimeState::default();
// let input = InputEvent::UserPromptSubmitted { /* ... */ };
// let result = transition(phase, &runtime, &input);
// // result.next_phase and result.actions are purely derived
// ```
//
// ## Command Execution Boundaries
//
// The FSM outputs `OutputAction` commands which are executed by an
// orchestrator layer. This separation ensures:
//   - FSM logic is testable and deterministic
//   - Side effects are centralized and transactionally managed
//   - The core crate (ai-history-core) remains storage-agnostic

#![cfg_attr(test, allow(unused_imports, unused_variables))]

pub mod input;
pub mod orchestrator;
pub mod output;
#[cfg(test)]
mod scope_tests;
pub mod state;
pub mod transition;

pub use input::*;
pub use output::*;
pub use state::*;
pub use transition::*;
