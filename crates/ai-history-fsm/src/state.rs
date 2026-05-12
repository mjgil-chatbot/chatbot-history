//! Finite control states for the FSM

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
/// Phase represents the finite control state of the interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    /// Initial state, waiting for user input
    Idle,
    /// Creating a new turn
    CreatingTurn,
    /// Streaming assistant response
    StreamingAssistant,
    /// Waiting for tool result
    AwaitingToolResult,
    /// Applying a file patch
    ApplyingPatch,
    /// Creating a checkpoint
    Checkpointing,
    /// Restoring from checkpoint
    Restoring,
    /// Resolving workspace conflicts
    ConflictResolution,
    /// Interaction was cancelled
    Cancelled,
    /// Error state
    Error,
}
