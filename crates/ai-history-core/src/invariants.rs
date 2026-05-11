//! System invariants for the AI history system
//!
//! Defines the invariants that must be maintained:
//! - No file write without a pre-write checkpoint
//! - No branch head mutation without an event
//! - No destructive restore without a safety snapshot
//! - No assistant delta after completion
//! - No checkpoint without a turn reference

use serde::{Deserialize, Serialize};

/// Category of invariant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvariantCategory {
    /// Workspace-related invariant
    Workspace,
    /// Branch-related invariant
    Branch,
    /// Turn-related invariant
    Turn,
    /// Event-related invariant
    Event,
    /// Checkpoint-related invariant
    Checkpoint,
}

/// Severity when invariant is violated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvariantSeverity {
    /// Warning - may cause issues
    Warning,
    /// Error - will cause incorrect behavior
    Error,
    /// Critical - data loss possible
    Critical,
}

/// A system invariant definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invariant {
    pub name: &'static str,
    pub description: &'static str,
    pub category: InvariantCategory,
    pub severity: InvariantSeverity,
    pub must_hold_during: Vec<&'static str>,
    pub reason: &'static str,
}

/// Check result for a single invariant
#[derive(Debug, Clone)]
pub struct InvariantCheck {
    pub invariant_name: &'static str,
    pub passed: bool,
    pub context: Option<String>,
}

impl InvariantCheck {
    pub fn pass(invariant_name: &'static str) -> Self {
        Self {
            invariant_name,
            passed: true,
            context: None,
        }
    }

    pub fn fail(invariant_name: &'static str, context: &str) -> Self {
        Self {
            invariant_name,
            passed: false,
            context: Some(context.to_string()),
        }
    }
}

/// Collection of all system invariants
pub struct InvariantRegistry {
    invariants: Vec<Invariant>,
}

impl Default for InvariantRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl InvariantRegistry {
    pub fn new() -> Self {
        Self {
            invariants: Self::all_invariants(),
        }
    }

    fn all_invariants() -> Vec<Invariant> {
        vec![
            // Workspace invariants
            Invariant {
                name: "no_file_write_without_checkpoint",
                description: "No file write without a pre-write checkpoint",
                category: InvariantCategory::Workspace,
                severity: InvariantSeverity::Critical,
                must_hold_during: vec!["ApplyingPatch", "ToolCallExecution"],
                reason: "Without a pre-write checkpoint, rollback would lose changes",
            },
            Invariant {
                name: "no_destructive_restore_without_safety",
                description: "No destructive restore without a safety snapshot",
                category: InvariantCategory::Workspace,
                severity: InvariantSeverity::Critical,
                must_hold_during: vec!["Restoring", "RestoreExecution"],
                reason: "Without safety snapshot, failed restore cannot be undone",
            },
            Invariant {
                name: "workspace_integrity_before_restore",
                description: "Workspace integrity must be verified before restore",
                category: InvariantCategory::Workspace,
                severity: InvariantSeverity::Error,
                must_hold_during: vec!["RestoreValidation"],
                reason: "Restoring to corrupted workspace causes data loss",
            },
            // Branch invariants
            Invariant {
                name: "no_branch_head_mutation_without_event",
                description: "No branch head mutation without an event",
                category: InvariantCategory::Branch,
                severity: InvariantSeverity::Critical,
                must_hold_during: vec!["BranchSwitch", "BranchCreation", "BranchFork"],
                reason: "Without events, branch history would be inconsistent",
            },
            Invariant {
                name: "branch_preserves_abandoned_futures",
                description: "Branches must preserve abandoned futures after rollback",
                category: InvariantCategory::Branch,
                severity: InvariantSeverity::Error,
                must_hold_during: vec!["Rollback", "RestoreExecution"],
                reason: "Users must be able to return to abandoned branches",
            },
            Invariant {
                name: "no_branch_delete_with_active_head",
                description: "Cannot delete branch that has active head turn",
                category: InvariantCategory::Branch,
                severity: InvariantSeverity::Error,
                must_hold_during: vec!["BranchDeletion"],
                reason: "Active branches must be preserved",
            },
            // Turn invariants
            Invariant {
                name: "no_assistant_delta_after_completion",
                description: "No assistant delta after completion",
                category: InvariantCategory::Turn,
                severity: InvariantSeverity::Critical,
                must_hold_during: vec!["TurnFinalization", "AssistantStreaming"],
                reason: "Deltas after completion cause inconsistent state",
            },
            Invariant {
                name: "turns_immutable_after_commit",
                description: "Turns are immutable after commit except allowed metadata",
                category: InvariantCategory::Turn,
                severity: InvariantSeverity::Error,
                must_hold_during: vec!["TurnCommit"],
                reason: "Immutable turns ensure reproducible history",
            },
            Invariant {
                name: "turn_has_parent_reference",
                description: "Turn must reference parent turn or null if first",
                category: InvariantCategory::Turn,
                severity: InvariantSeverity::Error,
                must_hold_during: vec!["TurnCreation"],
                reason: "Parent reference required for branch ancestry",
            },
            // Event invariants
            Invariant {
                name: "event_sequence_monotonic",
                description: "Event sequence numbers must be monotonically increasing",
                category: InvariantCategory::Event,
                severity: InvariantSeverity::Critical,
                must_hold_during: vec!["EventAppend", "EventReplay"],
                reason: "Out-of-order events cause incorrect replay",
            },
            Invariant {
                name: "event_checksum_valid",
                description: "Every event must have valid checksum",
                category: InvariantCategory::Event,
                severity: InvariantSeverity::Critical,
                must_hold_during: vec!["EventValidation", "EventReplay"],
                reason: "Invalid checksums indicate data corruption",
            },
            Invariant {
                name: "no_duplicate_event_id",
                description: "No duplicate event IDs in sequence",
                category: InvariantCategory::Event,
                severity: InvariantSeverity::Error,
                must_hold_during: vec!["EventAppend"],
                reason: "Duplicates cause replay ambiguity",
            },
            // Checkpoint invariants
            Invariant {
                name: "no_checkpoint_without_turn_reference",
                description: "No checkpoint without a turn reference",
                category: InvariantCategory::Checkpoint,
                severity: InvariantSeverity::Critical,
                must_hold_during: vec!["CheckpointCreation"],
                reason: "Checkpoint must be associated with a turn for restore",
            },
            Invariant {
                name: "checkpoint_has_valid_manifest",
                description: "Checkpoint must have valid manifest reference",
                category: InvariantCategory::Checkpoint,
                severity: InvariantSeverity::Critical,
                must_hold_during: vec!["CheckpointCreation"],
                reason: "Without manifest, checkpoint is unusable",
            },
            Invariant {
                name: "safety_checkpoint_before_destructive",
                description: "Safety checkpoint must be created before destructive operations",
                category: InvariantCategory::Checkpoint,
                severity: InvariantSeverity::Critical,
                must_hold_during: vec!["RestoreExecution", "PatchApplication"],
                reason: "Safety checkpoint enables rollback of failed operations",
            },
            Invariant {
                name: "named_checkpoint_protected",
                description: "Named checkpoints must be protected from garbage collection",
                category: InvariantCategory::Checkpoint,
                severity: InvariantSeverity::Error,
                must_hold_during: vec!["GarbageCollection"],
                reason: "User-named checkpoints are explicitly protected",
            },
        ]
    }

    /// Get all invariants
    pub fn all(&self) -> &[Invariant] {
        &self.invariants
    }

    /// Get invariants by category
    pub fn by_category(&self, category: InvariantCategory) -> Vec<&Invariant> {
        self.invariants
            .iter()
            .filter(|i| i.category == category)
            .collect()
    }

    /// Get critical invariants
    pub fn critical(&self) -> Vec<&Invariant> {
        self.invariants
            .iter()
            .filter(|i| i.severity == InvariantSeverity::Critical)
            .collect()
    }

    /// Get invariant by name
    pub fn get(&self, name: &str) -> Option<&Invariant> {
        self.invariants.iter().find(|i| i.name == name)
    }
}

/// Invariant checker for runtime verification
pub struct InvariantChecker;

impl InvariantChecker {
    /// Check file write invariant
    pub fn check_file_write_invariant(
        has_checkpoint: bool,
        is_destructive: bool,
    ) -> InvariantCheck {
        if is_destructive && !has_checkpoint {
            InvariantCheck::fail(
                "no_file_write_without_checkpoint",
                "File write attempted without pre-write checkpoint",
            )
        } else {
            InvariantCheck::pass("no_file_write_without_checkpoint")
        }
    }

    /// Check branch head mutation invariant
    pub fn check_branch_head_mvariant(has_event: bool) -> InvariantCheck {
        if !has_event {
            InvariantCheck::fail(
                "no_branch_head_mutation_without_event",
                "Branch head mutation attempted without corresponding event",
            )
        } else {
            InvariantCheck::pass("no_branch_head_mutation_without_event")
        }
    }

    /// Check restore safety snapshot invariant
    pub fn check_restore_safety_invariant(has_safety_snapshot: bool) -> InvariantCheck {
        if !has_safety_snapshot {
            InvariantCheck::fail(
                "no_destructive_restore_without_safety",
                "Restore attempted without safety snapshot",
            )
        } else {
            InvariantCheck::pass("no_destructive_restore_without_safety")
        }
    }

    /// Check assistant delta completion invariant
    pub fn check_delta_after_completion(
        is_completed: bool,
        has_pending_deltas: bool,
    ) -> InvariantCheck {
        if is_completed && has_pending_deltas {
            InvariantCheck::fail(
                "no_assistant_delta_after_completion",
                "Delta received after message was finalized",
            )
        } else {
            InvariantCheck::pass("no_assistant_delta_after_completion")
        }
    }

    /// Check checkpoint turn reference invariant
    pub fn check_checkpoint_turn_reference(has_turn_ref: bool) -> InvariantCheck {
        if !has_turn_ref {
            InvariantCheck::fail(
                "no_checkpoint_without_turn_reference",
                "Checkpoint created without turn reference",
            )
        } else {
            InvariantCheck::pass("no_checkpoint_without_turn_reference")
        }
    }

    /// Check event sequence monotonicity
    pub fn check_event_sequence_monotonic(previous_seq: u64, current_seq: u64) -> InvariantCheck {
        if current_seq <= previous_seq {
            InvariantCheck::fail(
                "event_sequence_monotonic",
                &format!(
                    "Event sequence not monotonic: {} -> {}",
                    previous_seq, current_seq
                ),
            )
        } else {
            InvariantCheck::pass("event_sequence_monotonic")
        }
    }

    /// Run all checks for a workspace operation
    pub fn check_workspace_operation(
        has_checkpoint: bool,
        has_safety: bool,
        is_destructive: bool,
    ) -> Vec<InvariantCheck> {
        vec![
            if is_destructive {
                Self::check_file_write_invariant(has_checkpoint, is_destructive)
            } else {
                InvariantCheck::pass("no_file_write_without_checkpoint")
            },
            if is_destructive {
                Self::check_restore_safety_invariant(has_safety)
            } else {
                InvariantCheck::pass("no_destructive_restore_without_safety")
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invariant_registry_has_invariants() {
        let registry = InvariantRegistry::new();
        assert!(!registry.all().is_empty());
    }

    #[test]
    fn test_invariant_registry_by_category() {
        let registry = InvariantRegistry::new();
        let workspace = registry.by_category(InvariantCategory::Workspace);
        assert!(!workspace.is_empty());
    }

    #[test]
    fn test_invariant_registry_critical() {
        let registry = InvariantRegistry::new();
        let critical = registry.critical();
        assert!(!critical.is_empty());
        assert!(critical
            .iter()
            .all(|i| i.severity == InvariantSeverity::Critical));
    }

    #[test]
    fn test_invariant_registry_get() {
        let registry = InvariantRegistry::new();
        let inv = registry.get("no_file_write_without_checkpoint");
        assert!(inv.is_some());
        assert_eq!(inv.unwrap().name, "no_file_write_without_checkpoint");
    }

    #[test]
    fn test_invariant_check_pass() {
        let check = InvariantCheck::pass("test_invariant");
        assert!(check.passed);
        assert!(check.context.is_none());
    }

    #[test]
    fn test_invariant_check_fail() {
        let check = InvariantCheck::fail("test_invariant", "test context");
        assert!(!check.passed);
        assert!(check.context.is_some());
    }

    #[test]
    fn test_check_file_write_invariant_pass() {
        let check = InvariantChecker::check_file_write_invariant(true, true);
        assert!(check.passed);
    }

    #[test]
    fn test_check_file_write_invariant_fail() {
        let check = InvariantChecker::check_file_write_invariant(false, true);
        assert!(!check.passed);
    }

    #[test]
    fn test_check_file_write_non_destructive() {
        let check = InvariantChecker::check_file_write_invariant(false, false);
        assert!(check.passed);
    }

    #[test]
    fn test_check_branch_head_mutation_pass() {
        let check = InvariantChecker::check_branch_head_mvariant(true);
        assert!(check.passed);
    }

    #[test]
    fn test_check_branch_head_mutation_fail() {
        let check = InvariantChecker::check_branch_head_mvariant(false);
        assert!(!check.passed);
    }

    #[test]
    fn test_check_restore_safety_pass() {
        let check = InvariantChecker::check_restore_safety_invariant(true);
        assert!(check.passed);
    }

    #[test]
    fn test_check_restore_safety_fail() {
        let check = InvariantChecker::check_restore_safety_invariant(false);
        assert!(!check.passed);
    }

    #[test]
    fn test_check_delta_after_completion_pass() {
        let check = InvariantChecker::check_delta_after_completion(false, false);
        assert!(check.passed);
    }

    #[test]
    fn test_check_delta_after_completion_fail() {
        let check = InvariantChecker::check_delta_after_completion(true, true);
        assert!(!check.passed);
    }

    #[test]
    fn test_check_checkpoint_turn_reference_pass() {
        let check = InvariantChecker::check_checkpoint_turn_reference(true);
        assert!(check.passed);
    }

    #[test]
    fn test_check_checkpoint_turn_reference_fail() {
        let check = InvariantChecker::check_checkpoint_turn_reference(false);
        assert!(!check.passed);
    }

    #[test]
    fn test_check_event_sequence_monotonic_pass() {
        let check = InvariantChecker::check_event_sequence_monotonic(5, 10);
        assert!(check.passed);
    }

    #[test]
    fn test_check_event_sequence_monotonic_fail() {
        let check = InvariantChecker::check_event_sequence_monotonic(10, 5);
        assert!(!check.passed);
    }

    #[test]
    fn test_check_event_sequence_monotonic_equal_fail() {
        let check = InvariantChecker::check_event_sequence_monotonic(10, 10);
        assert!(!check.passed);
    }

    #[test]
    fn test_check_workspace_operation_multiple() {
        let checks = InvariantChecker::check_workspace_operation(true, true, true);
        assert_eq!(checks.len(), 2);
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_invariant_categories_all_represented() {
        let registry = InvariantRegistry::new();
        for category in [
            InvariantCategory::Workspace,
            InvariantCategory::Branch,
            InvariantCategory::Turn,
            InvariantCategory::Event,
            InvariantCategory::Checkpoint,
        ] {
            let by_cat = registry.by_category(category);
            assert!(
                !by_cat.is_empty(),
                "Category {:?} should have invariants",
                category
            );
        }
    }

    #[test]
    fn test_all_invariants_have_reasons() {
        let registry = InvariantRegistry::new();
        for inv in registry.all() {
            assert!(
                !inv.reason.is_empty(),
                "Invariant {} should have reason",
                inv.name
            );
        }
    }

    #[test]
    fn test_invariant_severity_serialization() {
        let severity = InvariantSeverity::Critical;
        let json = serde_json::to_string(&severity).unwrap();
        let parsed: InvariantSeverity = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, InvariantSeverity::Critical);
    }

    #[test]
    fn test_invariant_serialization() {
        let inv = Invariant {
            name: "test",
            description: "test invariant",
            category: InvariantCategory::Workspace,
            severity: InvariantSeverity::Error,
            must_hold_during: vec!["Test"],
            reason: "test reason",
        };
        let json = serde_json::to_string(&inv).unwrap();
        assert!(json.contains("test"));
    }
}
