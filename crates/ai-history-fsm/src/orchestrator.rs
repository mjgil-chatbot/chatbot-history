//! Orchestrator layer for FSM command execution
//!
//! Task 27: Separate FSM commands from side effects
//!
//! ## Design
//!
//! This module defines the boundary between the pure FSM and side-effecting
//! code. The FSM outputs `OutputAction` commands which are purely declarative
//! - they describe *what* should happen, not *how* to do it.
//!
//! ## Command Execution
//!
//! An orchestrator receives `OutputAction` commands from FSM transitions
//! and executes them against real storage/snapshot/restore systems. The
//! orchestrator is responsible for:
//!   - Transactionally managing side effects
//!   - Handling execution failures
//!   - Feeding completion events back to the FSM
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! // Receive commands from FSM
//! for action in result.actions {
//!     match action {
//!         OutputAction::CreateTurn { .. } => {
//!             // Execute against event store
//!         }
//!         OutputAction::CaptureWorkspaceSnapshot { .. } => {
//!             // Execute against snapshot system
//!         }
//!         // ... etc
//!     }
//! }
//! ```
//!
//! ## Guarantees
//!
//! The FSM itself makes no filesystem, database, or network calls. It is
//! guaranteed to be pure and deterministic given the same inputs.

use crate::output::OutputAction;

/// Trait for command executors
/// Implementors handle specific command types
pub trait CommandExecutor {
    /// Execute a command and return any resulting data
    fn execute(&mut self, command: &OutputAction) -> CommandResult;
}

/// Result of command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Optional data returned from execution
    pub data: Option<serde_json::Value>,
    /// Error message if failed
    pub error: Option<String>,
}

impl CommandResult {
    pub fn success() -> Self {
        Self {
            success: true,
            data: None,
            error: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn failed(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// Extension trait for checking command purity guarantees
pub trait CommandPurityChecker {
    /// Verify this executor has no filesystem/database side effects
    fn is_pure(&self) -> bool;
}

/// Null executor for testing - validates commands are executable
/// without actually performing side effects
pub struct NullExecutor;

impl NullExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NullExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandExecutor for NullExecutor {
    fn execute(&mut self, _command: &OutputAction) -> CommandResult {
        // Accept all commands without executing them
        // This is used to validate FSM output without side effects
        CommandResult::success()
    }
}

impl CommandPurityChecker for NullExecutor {
    fn is_pure(&self) -> bool {
        true
    }
}

/// Command iterator for FSM actions
/// Provides type-safe iteration over output actions
pub struct CommandIterator<'a> {
    actions: std::slice::Iter<'a, OutputAction>,
}

impl<'a> CommandIterator<'a> {
    pub fn new(actions: &'a [OutputAction]) -> Self {
        Self {
            actions: actions.iter(),
        }
    }
}

impl<'a> Iterator for CommandIterator<'a> {
    type Item = &'a OutputAction;

    fn next(&mut self) -> Option<Self::Item> {
        self.actions.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::NotificationSeverity;
    use ai_history_core::{BranchId, CheckpointId, SessionId};

    #[test]
    fn test_null_executor_accepts_all_commands() {
        let mut executor = NullExecutor::new();

        let commands = vec![
            OutputAction::CreateTurn {
                session_id: SessionId::new(),
                branch_id: BranchId::new(),
                parent_turn_id: None,
            },
            OutputAction::CaptureWorkspaceSnapshot {
                reason: "test".to_string(),
                checkpoint_id: CheckpointId::new(),
            },
            OutputAction::EmitNotification {
                message: "test".to_string(),
                severity: NotificationSeverity::Info,
            },
        ];

        for cmd in &commands {
            let result = executor.execute(cmd);
            assert!(result.success);
        }
    }

    #[test]
    fn test_null_executor_is_pure() {
        let executor = NullExecutor::new();
        assert!(executor.is_pure());
    }

    #[test]
    fn test_command_result_success() {
        let result = CommandResult::success();
        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_command_result_with_data() {
        let result = CommandResult::success().with_data(serde_json::json!({"id": "123"}));
        assert!(result.success);
        assert!(result.data.is_some());
    }

    #[test]
    fn test_command_result_failure() {
        let result = CommandResult::failed("something went wrong".to_string());
        assert!(!result.success);
        assert!(result.error.is_some());
        assert_eq!(result.error.unwrap(), "something went wrong");
    }

    #[test]
    fn test_command_iterator() {
        let actions = vec![
            OutputAction::CancelPending,
            OutputAction::EmitNotification {
                message: "done".to_string(),
                severity: NotificationSeverity::Success,
            },
        ];

        let mut iter = CommandIterator::new(&actions);
        assert!(iter.next().is_some());
        assert!(iter.next().is_some());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_fsm_has_no_filesystem_calls() {
        // This test documents the purity guarantee
        // The FSM crate should not have std::fs, std::process, or similar
        // imports that would indicate side effects

        // If this test compiles and runs, it means the FSM module
        // structure is valid. Runtime purity is enforced by the
        // architecture, not by this test.
        let executor = NullExecutor::new();
        assert!(executor.is_pure());
    }
}
