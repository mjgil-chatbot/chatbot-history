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

impl Default for Phase {
    fn default() -> Self {
        Self::Idle
    }
}

impl Display for Phase {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::Idle => write!(f, "Idle"),
            Phase::CreatingTurn => write!(f, "CreatingTurn"),
            Phase::StreamingAssistant => write!(f, "StreamingAssistant"),
            Phase::AwaitingToolResult => write!(f, "AwaitingToolResult"),
            Phase::ApplyingPatch => write!(f, "ApplyingPatch"),
            Phase::Checkpointing => write!(f, "Checkpointing"),
            Phase::Restoring => write!(f, "Restoring"),
            Phase::ConflictResolution => write!(f, "ConflictResolution"),
            Phase::Cancelled => write!(f, "Cancelled"),
            Phase::Error => write!(f, "Error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_default() {
        assert_eq!(Phase::default(), Phase::Idle);
    }

    #[test]
    fn test_phase_display() {
        assert_eq!(Phase::Idle.to_string(), "Idle");
        assert_eq!(Phase::StreamingAssistant.to_string(), "StreamingAssistant");
    }

    #[test]
    fn test_phase_serialization() {
        let phase = Phase::StreamingAssistant;
        let json = serde_json::to_string(&phase).unwrap();
        let parsed: Phase = serde_json::from_str(&json).unwrap();
        assert_eq!(phase, parsed);
    }
}
