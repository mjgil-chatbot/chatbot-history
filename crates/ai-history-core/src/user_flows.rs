//! Core user flows for the AI history system
//!
//! Defines step-by-step flows for all user interactions:
//! - send prompt, stream answer, tool call, apply patch
//! - restore checkpoint, restore files only, restore chat only, restore both
//! - fork from checkpoint, edit previous prompt
//! - redo abandoned branch, resolve conflicts

use serde::{Deserialize, Serialize};

/// Phase in a user flow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowPhase {
    Init,
    Waiting,
    Processing,
    AwaitingInput,
    Completed,
    Failed,
    Cancelled,
}

impl Default for FlowPhase {
    fn default() -> Self {
        Self::Init
    }
}

/// A step in a user flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStep {
    pub step_number: usize,
    pub name: &'static str,
    pub description: &'static str,
    pub next_phase: FlowPhase,
    pub side_effects: Vec<&'static str>,
    pub checkpoints_created: Vec<&'static str>,
}

/// A complete user flow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFlow {
    pub name: &'static str,
    pub description: &'static str,
    pub steps: Vec<FlowStep>,
    pub expected_outcome: &'static str,
    pub rollback_available: bool,
}

impl UserFlow {
    /// Get step by number
    pub fn get_step(&self, step_number: usize) -> Option<&FlowStep> {
        self.steps.iter().find(|s| s.step_number == step_number)
    }

    /// Get total steps
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

/// Send prompt flow
pub fn send_prompt_flow() -> UserFlow {
    UserFlow {
        name: "send_prompt",
        description: "User sends a prompt and receives streaming response",
        steps: vec![
            FlowStep {
                step_number: 1,
                name: "capture_pre_state",
                description: "Capture pre-turn checkpoint for safety",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Create safety checkpoint"],
                checkpoints_created: vec!["pre_turn_checkpoint"],
            },
            FlowStep {
                step_number: 2,
                name: "create_turn",
                description: "Create new turn record",
                next_phase: FlowPhase::Waiting,
                side_effects: vec!["Create turn record", "Append TurnStarted event"],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 3,
                name: "stream_response",
                description: "Stream assistant response deltas",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Append AssistantMessageDelta events"],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 4,
                name: "finalize_response",
                description: "Finalize assistant message",
                next_phase: FlowPhase::Completed,
                side_effects: vec!["Append AssistantMessageFinalized event"],
                checkpoints_created: vec!["post_turn_checkpoint"],
            },
        ],
        expected_outcome: "New turn created with full conversation history",
        rollback_available: true,
    }
}

/// Tool call flow
pub fn tool_call_flow() -> UserFlow {
    UserFlow {
        name: "tool_call",
        description: "Assistant requests tool execution and receives result",
        steps: vec![
            FlowStep {
                step_number: 1,
                name: "capture_pre_tool_state",
                description: "Capture pre-tool checkpoint for safety",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Create pre-tool checkpoint"],
                checkpoints_created: vec!["pre_tool_checkpoint"],
            },
            FlowStep {
                step_number: 2,
                name: "request_tool",
                description: "Emit ToolCallRequested event",
                next_phase: FlowPhase::AwaitingInput,
                side_effects: vec!["Append ToolCallRequested event"],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 3,
                name: "execute_tool",
                description: "Tool executes in workspace",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Apply file changes"],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 4,
                name: "record_result",
                description: "Record tool result",
                next_phase: FlowPhase::Completed,
                side_effects: vec!["Append ToolCallCompleted event"],
                checkpoints_created: vec!["post_tool_checkpoint"],
            },
        ],
        expected_outcome: "Tool executed and result recorded in history",
        rollback_available: true,
    }
}

/// Apply patch flow
pub fn apply_patch_flow() -> UserFlow {
    UserFlow {
        name: "apply_patch",
        description: "User approves and applies a diff patch to workspace",
        steps: vec![
            FlowStep {
                step_number: 1,
                name: "capture_pre_patch_state",
                description: "Capture workspace state before patch",
                next_phase: FlowPhase::Waiting,
                side_effects: vec!["Create pre-patch checkpoint"],
                checkpoints_created: vec!["pre_patch_checkpoint"],
            },
            FlowStep {
                step_number: 2,
                name: "validate_patch",
                description: "Validate patch can be applied",
                next_phase: FlowPhase::AwaitingInput,
                side_effects: vec![],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 3,
                name: "await_user_approval",
                description: "Wait for user to approve patch",
                next_phase: FlowPhase::AwaitingInput,
                side_effects: vec![],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 4,
                name: "apply_changes",
                description: "Apply patch to workspace files",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Apply file diffs", "Append FilePatchApplied event"],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 5,
                name: "verify_and_finalize",
                description: "Verify patch application and create post-patch snapshot",
                next_phase: FlowPhase::Completed,
                side_effects: vec![
                    "Create post-patch snapshot",
                    "Append CheckpointCreated event",
                ],
                checkpoints_created: vec!["post_patch_checkpoint"],
            },
        ],
        expected_outcome: "Patch applied, workspace updated, history recorded",
        rollback_available: true,
    }
}

/// Restore checkpoint flow (full restore)
pub fn restore_checkpoint_flow() -> UserFlow {
    UserFlow {
        name: "restore_checkpoint",
        description: "Restore both workspace and conversation to checkpoint",
        steps: vec![
            FlowStep {
                step_number: 1,
                name: "capture_safety",
                description: "Capture safety snapshot of current state",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Create safety snapshot"],
                checkpoints_created: vec!["safety_snapshot"],
            },
            FlowStep {
                step_number: 2,
                name: "detect_conflicts",
                description: "Compare current workspace with checkpoint",
                next_phase: FlowPhase::Processing,
                side_effects: vec![],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 3,
                name: "await_user_confirmation",
                description: "Require user confirmation for destructive restore",
                next_phase: FlowPhase::AwaitingInput,
                side_effects: vec![],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 4,
                name: "restore_workspace",
                description: "Apply checkpoint snapshot to workspace",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Restore files from blob store"],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 5,
                name: "restore_conversation",
                description: "Set branch head to checkpoint turn",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Update branch head"],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 6,
                name: "finalize",
                description: "Emit restore completed event",
                next_phase: FlowPhase::Completed,
                side_effects: vec!["Append WorkspaceRestored event"],
                checkpoints_created: vec![],
            },
        ],
        expected_outcome: "Workspace and conversation restored to checkpoint state",
        rollback_available: true,
    }
}

/// Restore files only flow
pub fn restore_files_only_flow() -> UserFlow {
    UserFlow {
        name: "restore_files_only",
        description: "Restore only workspace files, not conversation",
        steps: vec![
            FlowStep {
                step_number: 1,
                name: "capture_safety",
                description: "Capture safety snapshot",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Create safety snapshot"],
                checkpoints_created: vec!["safety_snapshot"],
            },
            FlowStep {
                step_number: 2,
                name: "detect_conflicts",
                description: "Compare workspace with checkpoint",
                next_phase: FlowPhase::Processing,
                side_effects: vec![],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 3,
                name: "await_user_confirmation",
                description: "Require confirmation for file changes",
                next_phase: FlowPhase::AwaitingInput,
                side_effects: vec![],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 4,
                name: "restore_workspace",
                description: "Apply checkpoint files to workspace",
                next_phase: FlowPhase::Completed,
                side_effects: vec!["Restore files", "Append WorkspaceFilesRestored event"],
                checkpoints_created: vec![],
            },
        ],
        expected_outcome: "Workspace restored, conversation history unchanged",
        rollback_available: true,
    }
}

/// Restore chat only flow
pub fn restore_chat_only_flow() -> UserFlow {
    UserFlow {
        name: "restore_chat_only",
        description: "Restore only conversation, not workspace files",
        steps: vec![
            FlowStep {
                step_number: 1,
                name: "create_restore_branch",
                description: "Create new branch from checkpoint turn",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Create branch from checkpoint"],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 2,
                name: "switch_to_restore_branch",
                description: "Switch active branch to restore branch",
                next_phase: FlowPhase::Completed,
                side_effects: vec!["Update active branch"],
                checkpoints_created: vec![],
            },
        ],
        expected_outcome: "New branch created, conversation restored, files unchanged",
        rollback_available: true,
    }
}

/// Fork from checkpoint flow
pub fn fork_from_checkpoint_flow() -> UserFlow {
    UserFlow {
        name: "fork_from_checkpoint",
        description: "Create new branch from checkpoint without restoring",
        steps: vec![
            FlowStep {
                step_number: 1,
                name: "create_fork_branch",
                description: "Create branch from checkpoint",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Create new branch", "Append BranchCreated event"],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 2,
                name: "switch_to_fork",
                description: "Switch to new fork branch",
                next_phase: FlowPhase::Completed,
                side_effects: vec!["Update active branch"],
                checkpoints_created: vec![],
            },
        ],
        expected_outcome: "New branch created from checkpoint, current state preserved",
        rollback_available: true,
    }
}

/// Edit previous prompt flow
pub fn edit_previous_prompt_flow() -> UserFlow {
    UserFlow {
        name: "edit_previous_prompt",
        description: "Edit a previous prompt and create new branch",
        steps: vec![
            FlowStep {
                step_number: 1,
                name: "capture_branch_point",
                description: "Capture branch point at parent turn",
                next_phase: FlowPhase::Processing,
                side_effects: vec![],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 2,
                name: "create_edit_branch",
                description: "Create new branch from parent turn",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Create branch from parent", "Append BranchCreated event"],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 3,
                name: "add_edited_message",
                description: "Add edited message as new turn",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Append UserMessageReceived event with edit flag"],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 4,
                name: "stream_new_response",
                description: "Stream new assistant response",
                next_phase: FlowPhase::Completed,
                side_effects: vec!["Stream response deltas"],
                checkpoints_created: vec![],
            },
        ],
        expected_outcome: "New branch created with edited prompt, original preserved",
        rollback_available: true,
    }
}

/// Redo abandoned branch flow
pub fn redo_abandoned_branch_flow() -> UserFlow {
    UserFlow {
        name: "redo_abandoned_branch",
        description: "Continue from an abandoned branch after rollback",
        steps: vec![
            FlowStep {
                step_number: 1,
                name: "identify_abandoned_branch",
                description: "Find abandoned branch with future turns",
                next_phase: FlowPhase::Processing,
                side_effects: vec![],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 2,
                name: "switch_to_abandoned",
                description: "Switch to abandoned branch",
                next_phase: FlowPhase::Processing,
                side_effects: vec!["Update active branch"],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 3,
                name: "continue_streaming",
                description: "Continue or restart assistant response",
                next_phase: FlowPhase::Completed,
                side_effects: vec!["Stream response or resume from checkpoint"],
                checkpoints_created: vec![],
            },
        ],
        expected_outcome: "Returned to abandoned branch, can continue from that point",
        rollback_available: true,
    }
}

/// Resolve conflicts flow
pub fn resolve_conflicts_flow() -> UserFlow {
    UserFlow {
        name: "resolve_conflicts",
        description: "User resolves workspace conflicts before restore",
        steps: vec![
            FlowStep {
                step_number: 1,
                name: "detect_conflicts",
                description: "Classify each conflict by type",
                next_phase: FlowPhase::Processing,
                side_effects: vec![],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 2,
                name: "present_ui",
                description: "Show conflict resolution UI",
                next_phase: FlowPhase::AwaitingInput,
                side_effects: vec!["Display conflict screen"],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 3,
                name: "collect_resolutions",
                description: "User selects resolution for each conflict",
                next_phase: FlowPhase::AwaitingInput,
                side_effects: vec![],
                checkpoints_created: vec![],
            },
            FlowStep {
                step_number: 4,
                name: "apply_resolutions",
                description: "Apply selected resolutions to workspace",
                next_phase: FlowPhase::Completed,
                side_effects: vec!["Apply file changes", "Append ConflictResolved event"],
                checkpoints_created: vec![],
            },
        ],
        expected_outcome: "All conflicts resolved, workspace ready for restore",
        rollback_available: true,
    }
}

/// All defined user flows
pub fn all_user_flows() -> Vec<UserFlow> {
    vec![
        send_prompt_flow(),
        tool_call_flow(),
        apply_patch_flow(),
        restore_checkpoint_flow(),
        restore_files_only_flow(),
        restore_chat_only_flow(),
        fork_from_checkpoint_flow(),
        edit_previous_prompt_flow(),
        redo_abandoned_branch_flow(),
        resolve_conflicts_flow(),
    ]
}

/// Find flow by name
pub fn get_flow_by_name(name: &str) -> Option<UserFlow> {
    all_user_flows().into_iter().find(|f| f.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_prompt_flow() {
        let flow = send_prompt_flow();
        assert_eq!(flow.name, "send_prompt");
        assert!(flow.rollback_available);
        assert_eq!(flow.steps.len(), 4);
    }

    #[test]
    fn test_tool_call_flow() {
        let flow = tool_call_flow();
        assert_eq!(flow.name, "tool_call");
        assert!(flow.rollback_available);
    }

    #[test]
    fn test_apply_patch_flow() {
        let flow = apply_patch_flow();
        assert_eq!(flow.name, "apply_patch");
        assert!(flow.steps.iter().any(|s| s.name == "await_user_approval"));
    }

    #[test]
    fn test_restore_checkpoint_flow() {
        let flow = restore_checkpoint_flow();
        assert!(flow.steps.iter().any(|s| s.name == "capture_safety"));
        assert!(flow.steps.iter().any(|s| s.name == "detect_conflicts"));
        assert!(flow
            .steps
            .iter()
            .any(|s| s.name == "await_user_confirmation"));
    }

    #[test]
    fn test_restore_files_only_flow() {
        let flow = restore_files_only_flow();
        assert!(flow.steps.iter().any(|s| s.name == "restore_workspace"));
        assert!(!flow.steps.iter().any(|s| s.name == "restore_conversation"));
    }

    #[test]
    fn test_restore_chat_only_flow() {
        let flow = restore_chat_only_flow();
        assert!(flow.steps.iter().any(|s| s.name == "create_restore_branch"));
        assert!(flow
            .steps
            .iter()
            .any(|s| s.name == "switch_to_restore_branch"));
    }

    #[test]
    fn test_fork_from_checkpoint_flow() {
        let flow = fork_from_checkpoint_flow();
        assert_eq!(flow.steps.len(), 2);
    }

    #[test]
    fn test_edit_previous_prompt_flow() {
        let flow = edit_previous_prompt_flow();
        assert!(flow.steps.iter().any(|s| s.name == "create_edit_branch"));
        assert!(flow.steps.iter().any(|s| s.name == "add_edited_message"));
    }

    #[test]
    fn test_redo_abandoned_branch_flow() {
        let flow = redo_abandoned_branch_flow();
        assert!(flow
            .steps
            .iter()
            .any(|s| s.name == "identify_abandoned_branch"));
    }

    #[test]
    fn test_resolve_conflicts_flow() {
        let flow = resolve_conflicts_flow();
        assert!(flow.steps.iter().any(|s| s.name == "present_ui"));
        assert!(flow.steps.iter().any(|s| s.name == "collect_resolutions"));
    }

    #[test]
    fn test_flow_get_step() {
        let flow = send_prompt_flow();
        assert!(flow.get_step(1).is_some());
        assert!(flow.get_step(99).is_none());
    }

    #[test]
    fn test_flow_step_count() {
        let flow = send_prompt_flow();
        assert_eq!(flow.step_count(), 4);
    }

    #[test]
    fn test_all_user_flows_count() {
        let flows = all_user_flows();
        assert_eq!(flows.len(), 10);
    }

    #[test]
    fn test_get_flow_by_name() {
        assert!(get_flow_by_name("send_prompt").is_some());
        assert!(get_flow_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_flow_phase_default() {
        assert_eq!(FlowPhase::default(), FlowPhase::Init);
    }

    #[test]
    fn test_flow_step_serialization() {
        let step = FlowStep {
            step_number: 1,
            name: "test",
            description: "test step",
            next_phase: FlowPhase::Completed,
            side_effects: vec!["effect1"],
            checkpoints_created: vec!["cp1"],
        };
        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("\"test\""));
    }

    #[test]
    fn test_user_flow_serialization() {
        let flow = send_prompt_flow();
        let json = serde_json::to_string(&flow).unwrap();
        assert!(json.contains("send_prompt"));
    }

    #[test]
    fn test_all_flows_have_expected_outcome() {
        for flow in all_user_flows() {
            assert!(!flow.expected_outcome.is_empty());
        }
    }

    #[test]
    fn test_all_flows_rollback_available() {
        for flow in all_user_flows() {
            assert!(
                flow.rollback_available,
                "Flow {} should be rollback available",
                flow.name
            );
        }
    }

    #[test]
    fn test_restore_creates_safety_checkpoint() {
        let flow = restore_checkpoint_flow();
        let has_safety = flow
            .steps
            .iter()
            .any(|s| s.checkpoints_created.contains(&"safety_snapshot"));
        assert!(has_safety);
    }
}
