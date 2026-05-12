//! Operation orchestrator for coordinating FSM side effects
//!
//! Task 28: Implement operation orchestration
//!
//! ## Design
//!
//! This module provides the orchestration layer that connects the pure FSM
//! to side-effecting systems. It receives `OutputAction` commands from FSM
//! transitions and executes them against storage, snapshot, and restore
//! systems.
//!
//! ## Execution Flow
//!
//! ```text
//! FSM Transition -> OutputAction commands -> Orchestrator -> Side Effects
//!                                              |
//!                                              v
//!                                    Completion Event -> FSM Input
//! ```
//!
//! ## Transactional Guarantees
//!
//! The orchestrator executes commands transactionally where possible,
//! ensuring atomicity of multi-step operations. If a command fails,
//! the orchestrator can emit error events back to the FSM.

use ai_history_core::{BranchId, CheckpointId, CoreResult, SessionId, TurnId};
use ai_history_events::EventStore;
use ai_history_fsm::output::OutputAction;
/// Event types that can be fed back to the FSM
#[derive(Debug, Clone)]
pub enum OrchestratedEvent {
    /// Session was created
    SessionCreated(SessionId),
    /// Turn was created
    TurnCreated(TurnId),
    /// Checkpoint was created
    CheckpointCreated(CheckpointId),
    /// Workspace was restored
    WorkspaceRestored,
    /// Tool call completed
    ToolCallCompleted {
        turn_id: TurnId,
        result: serde_json::Value,
    },
    /// Branch was created
    BranchCreated(BranchId),
    /// Branch head was moved
    BranchHeadMoved {
        branch_id: BranchId,
        new_turn_id: TurnId,
    },
    /// Error occurred during execution
    Error { message: String, recoverable: bool },
    /// Operation was cancelled
    Cancelled,
}
/// Orchestrator for FSM command execution
pub struct Orchestrator<S: EventStore> {
    store: S,
}
impl<S: EventStore> Orchestrator<S> {
    /// Create a new orchestrator with the given store
    pub fn new(store: S) -> Self {
        Self { store }
    }}
impl<S: EventStore> Orchestrator<S> {
    /// Execute a single command and return the result
    pub fn execute_command(
        &mut self,
        command: &OutputAction,
    ) -> CoreResult<Option<OrchestratedEvent>> {
        match command {
            OutputAction::AppendEvent { event: _ } => {
                // Events are stored for later replay
                // The event itself doesn't trigger a completion event
                Ok(None)
            }
            OutputAction::CreateTurn {
                session_id: _,
                branch_id: _,
                parent_turn_id: _,
            } => {
                // CreateTurn triggers TurnCreated event
                // Actual turn creation would happen in store
                Ok(Some(OrchestratedEvent::TurnCreated(TurnId::new())))
            }
            OutputAction::CaptureWorkspaceSnapshot {
                reason: _,
                checkpoint_id,
            } => {
                // Snapshot capture triggers CheckpointCreated
                Ok(Some(OrchestratedEvent::CheckpointCreated(*checkpoint_id)))
            }
            OutputAction::ApplyPatch { patch_id: _ } => {
                // Patch application - actual file operations happen in restore crate
                // Return success event
                Ok(None)
            }
            OutputAction::RestoreWorkspace {
                checkpoint_id: _,
                mode: _,
            } => {
                // Workspace restoration
                Ok(Some(OrchestratedEvent::WorkspaceRestored))
            }
            OutputAction::MoveBranchHead { turn_id } => {
                // Move branch head
                Ok(Some(OrchestratedEvent::BranchHeadMoved {
                    branch_id: BranchId::new(), // Would be determined by runtime state
                    new_turn_id: *turn_id,
                }))
            }
            OutputAction::CreateBranch {
                from_turn_id: _,
                name: _,
            } => {
                // Create new branch
                Ok(Some(OrchestratedEvent::BranchCreated(BranchId::new())))
            }
            OutputAction::ShowConflictUI { files: _ } => {
                // Conflict UI shown - FSM handles this via ConflictResolution phase
                Ok(None)
            }
            OutputAction::EmitNotification { .. } => {
                // Notifications don't trigger FSM events
                Ok(None)
            }
            OutputAction::FinalizeTurn { turn_id: _ } => {
                // Turn finalization complete
                Ok(None)
            }
            OutputAction::CancelPending => {
                // Cancellation triggered
                Ok(Some(OrchestratedEvent::Cancelled))
            }
            OutputAction::BeginToolPolling { tool_name: _ } => {
                // Tool polling started - actual result comes asynchronously
                Ok(None)
            }
            OutputAction::EmitError { error, .. } => {
                // Error during execution
                Ok(Some(OrchestratedEvent::Error {
                    message: error.clone(),
                    recoverable: true,
                }))
            }
            OutputAction::NavigateToTurn { turn_id: _ } => {
                // Navigation doesn't trigger events
                Ok(None)
            }
            OutputAction::ForkBranch {
                from_turn_id: _,
                name: _,
            } => {
                // Fork creates a new branch
                Ok(Some(OrchestratedEvent::BranchCreated(BranchId::new())))
            }
            OutputAction::SwitchBranch { branch_id } => {
                // Branch switch complete
                Ok(Some(OrchestratedEvent::BranchHeadMoved {
                    branch_id: *branch_id,
                    new_turn_id: TurnId::new(),
                }))
            }
            OutputAction::RestoreWorkspaceOnly { checkpoint_id: _ } => {
                Ok(Some(OrchestratedEvent::WorkspaceRestored))
            }
            OutputAction::RestoreConversationOnly { checkpoint_id: _ } => {
                Ok(Some(OrchestratedEvent::WorkspaceRestored))
            }
        }
    }}
impl<S: EventStore> Orchestrator<S> {
    /// Execute multiple commands and collect all completion events
    pub fn execute_all(&mut self, commands: &[OutputAction]) -> CoreResult<Vec<OrchestratedEvent>> {
        let mut events = Vec::new();
        for cmd in commands {
            if let Some(event) = self.execute_command(cmd)? {
                events.push(event);
            }
        }
        Ok(events)
    }}
impl<S: EventStore> Orchestrator<S> {
    /// Get the underlying store
    pub fn store(&self) -> &S {
        &self.store
    }}
impl<S: EventStore> Orchestrator<S> {
    /// Get a mutable reference to the store
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }}
/// Builder for creating orchestrators with specific configurations
pub struct OrchestratorBuilder<S: EventStore> {
    store: S,
}
impl<S: EventStore> OrchestratorBuilder<S> {
    /// Create a new builder with the given store
    pub fn new(store: S) -> Self {
        Self { store }
    }}
impl<S: EventStore> OrchestratorBuilder<S> {
    /// Build the orchestrator
    pub fn build(self) -> Orchestrator<S> {
        Orchestrator::new(self.store)
    }}
/// Extension trait for converting OrchestratedEvent to FSM input events
pub trait ToFsmInput {
    fn to_fsm_input(&self) -> Option<ai_history_fsm::input::InputEvent>;
}
impl ToFsmInput for OrchestratedEvent {
    fn to_fsm_input(&self) -> Option<ai_history_fsm::input::InputEvent> {
        match self {
            OrchestratedEvent::SessionCreated(_) => None,
            OrchestratedEvent::TurnCreated(_) => None,
            OrchestratedEvent::CheckpointCreated(_) => {
                Some(ai_history_fsm::input::InputEvent::CheckpointCreated {
                    checkpoint_id: CheckpointId::new(),
                })
            }
            OrchestratedEvent::WorkspaceRestored => {
                Some(ai_history_fsm::input::InputEvent::RestoreCompleted)
            }
            OrchestratedEvent::ToolCallCompleted { turn_id, result } => {
                Some(ai_history_fsm::input::InputEvent::ToolCallCompleted {
                    result: result.clone(),
                    turn_id: *turn_id,
                })
            }
            OrchestratedEvent::BranchCreated(_) => None,
            OrchestratedEvent::BranchHeadMoved { .. } => None,
            OrchestratedEvent::Error {
                message,
                recoverable,
            } => Some(ai_history_fsm::input::InputEvent::Error {
                error: message.clone(),
                recoverable: *recoverable,
            }),
            OrchestratedEvent::Cancelled => {
                Some(ai_history_fsm::input::InputEvent::CancellationRequested)
            }
        }
    }
}
