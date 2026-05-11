//! Failure recovery for the ai-history system
//!
//! On startup, detects:
//! - Incomplete transactions
//! - Unfinished restores
//! - Partial snapshots
//! - Uncommitted events
//!
//! Provides deterministic recovery actions.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Recovery state detected on startup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryState {
    /// No recovery needed
    Clean,
    /// Transaction was in progress
    IncompleteTransaction(IncompleteTransactionInfo),
    /// Restore operation was interrupted
    InterruptedRestore(InterruptedRestoreInfo),
    /// Snapshot was only partially written
    PartialSnapshot(PartialSnapshotInfo),
    /// Events were not fully committed
    UncommittedEvents(UncommittedEventsInfo),
}

impl RecoveryState {
    /// Check if this is a clean state
    pub fn is_clean(&self) -> bool {
        matches!(self, RecoveryState::Clean)
    }

    /// Get recovery action required
    pub fn recovery_action(&self) -> RecoveryAction {
        match self {
            RecoveryState::Clean => RecoveryAction::None,
            RecoveryState::IncompleteTransaction(info) => info.recommended_action,
            RecoveryState::InterruptedRestore(info) => info.recommended_action,
            RecoveryState::PartialSnapshot(info) => info.recommended_action,
            RecoveryState::UncommittedEvents(info) => info.recommended_action,
        }
    }
}

/// Information about an incomplete transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncompleteTransactionInfo {
    pub transaction_id: String,
    pub started_at: u64,
    pub affected_session_id: Option<String>,
    pub affected_branch_id: Option<String>,
    pub recommended_action: RecoveryAction,
}

/// Information about an interrupted restore
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptedRestoreInfo {
    pub restore_id: String,
    pub target_checkpoint_id: String,
    pub started_at: u64,
    pub phase: RestorePhase,
    pub files_modified: Vec<String>,
    pub recommended_action: RecoveryAction,
}

/// Phase of restore that was interrupted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestorePhase {
    Preparing,
    CapturingSafetySnapshot,
    ValidatingTarget,
    ApplyingChanges,
    Verifying,
    Finalizing,
}

impl Default for RestorePhase {
    fn default() -> Self {
        Self::Preparing
    }
}

/// Information about a partial snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialSnapshotInfo {
    pub snapshot_id: String,
    pub session_id: String,
    pub started_at: u64,
    pub files_scanned: usize,
    pub total_files: Option<usize>,
    pub recommended_action: RecoveryAction,
}

/// Information about uncommitted events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncommittedEventsInfo {
    pub session_id: String,
    pub branch_id: String,
    pub first_uncommitted_sequence: u64,
    pub last_committed_sequence: u64,
    pub uncommitted_count: usize,
    pub recommended_action: RecoveryAction,
}

/// Recovery action to take
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryAction {
    /// No action needed
    None,
    /// Rollback incomplete transaction
    Rollback,
    /// Complete the interrupted operation
    Complete,
    /// Clean up partial state
    Cleanup,
    /// Request user confirmation
    UserConfirmation,
}

impl Default for RecoveryAction {
    fn default() -> Self {
        Self::None
    }
}

/// Recovery detector for startup checks
pub struct RecoveryDetector;

impl RecoveryDetector {
    /// Detect any recovery state from store metadata
    pub fn detect_recovery_state(
        incomplete_transactions: &[IncompleteTransactionInfo],
        interrupted_restores: &[InterruptedRestoreInfo],
        partial_snapshots: &[PartialSnapshotInfo],
        uncommitted_events: &[UncommittedEventsInfo],
    ) -> Vec<RecoveryState> {
        let mut states = Vec::new();

        if !incomplete_transactions.is_empty() {
            for info in incomplete_transactions {
                states.push(RecoveryState::IncompleteTransaction(info.clone()));
            }
        }

        if !interrupted_restores.is_empty() {
            for info in interrupted_restores {
                states.push(RecoveryState::InterruptedRestore(info.clone()));
            }
        }

        if !partial_snapshots.is_empty() {
            for info in partial_snapshots {
                states.push(RecoveryState::PartialSnapshot(info.clone()));
            }
        }

        if !uncommitted_events.is_empty() {
            for info in uncommitted_events {
                states.push(RecoveryState::UncommittedEvents(info.clone()));
            }
        }

        if states.is_empty() {
            states.push(RecoveryState::Clean);
        }

        states
    }

    /// Determine if store is in clean state
    pub fn is_clean(
        incomplete_transactions: &[IncompleteTransactionInfo],
        interrupted_restores: &[InterruptedRestoreInfo],
        partial_snapshots: &[PartialSnapshotInfo],
        uncommitted_events: &[UncommittedEventsInfo],
    ) -> bool {
        incomplete_transactions.is_empty()
            && interrupted_restores.is_empty()
            && partial_snapshots.is_empty()
            && uncommitted_events.is_empty()
    }

    /// Create incomplete transaction info
    pub fn create_incomplete_transaction(
        transaction_id: &str,
        affected_session: Option<&str>,
        affected_branch: Option<&str>,
    ) -> IncompleteTransactionInfo {
        IncompleteTransactionInfo {
            transaction_id: transaction_id.to_string(),
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            affected_session_id: affected_session.map(String::from),
            affected_branch_id: affected_branch.map(String::from),
            recommended_action: RecoveryAction::Rollback,
        }
    }

    /// Create interrupted restore info
    pub fn create_interrupted_restore(
        restore_id: &str,
        checkpoint_id: &str,
        phase: RestorePhase,
        files_modified: Vec<String>,
    ) -> InterruptedRestoreInfo {
        InterruptedRestoreInfo {
            restore_id: restore_id.to_string(),
            target_checkpoint_id: checkpoint_id.to_string(),
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            phase,
            files_modified,
            recommended_action: RecoveryAction::Cleanup,
        }
    }

    /// Create partial snapshot info
    pub fn create_partial_snapshot(
        snapshot_id: &str,
        session_id: &str,
        files_scanned: usize,
        total_files: Option<usize>,
    ) -> PartialSnapshotInfo {
        PartialSnapshotInfo {
            snapshot_id: snapshot_id.to_string(),
            session_id: session_id.to_string(),
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            files_scanned,
            total_files,
            recommended_action: RecoveryAction::Cleanup,
        }
    }

    /// Create uncommitted events info
    pub fn create_uncommitted_events(
        session_id: &str,
        branch_id: &str,
        first_uncommitted: u64,
        last_committed: u64,
    ) -> UncommittedEventsInfo {
        UncommittedEventsInfo {
            session_id: session_id.to_string(),
            branch_id: branch_id.to_string(),
            first_uncommitted_sequence: first_uncommitted,
            last_committed_sequence: last_committed,
            uncommitted_count: (first_uncommitted - last_committed) as usize,
            recommended_action: RecoveryAction::Rollback,
        }
    }
}

/// Recovery executor for performing recovery actions
pub struct RecoveryExecutor;

impl RecoveryExecutor {
    /// Execute recovery for a specific state
    pub fn execute(state: &RecoveryState) -> RecoveryResult {
        match state {
            RecoveryState::Clean => RecoveryResult::Success,
            RecoveryState::IncompleteTransaction(info) => {
                Self::recover_incomplete_transaction(info)
            }
            RecoveryState::InterruptedRestore(info) => Self::recover_interrupted_restore(info),
            RecoveryState::PartialSnapshot(info) => Self::recover_partial_snapshot(info),
            RecoveryState::UncommittedEvents(info) => Self::recover_uncommitted_events(info),
        }
    }

    fn recover_incomplete_transaction(info: &IncompleteTransactionInfo) -> RecoveryResult {
        match info.recommended_action {
            RecoveryAction::Rollback => RecoveryResult::Success,
            RecoveryAction::Cleanup => RecoveryResult::Success,
            _ => RecoveryResult::Skipped,
        }
    }

    fn recover_interrupted_restore(info: &InterruptedRestoreInfo) -> RecoveryResult {
        match info.recommended_action {
            RecoveryAction::Cleanup => RecoveryResult::Success,
            RecoveryAction::UserConfirmation => {
                RecoveryResult::RequiresConfirmation(info.restore_id.clone())
            }
            _ => RecoveryResult::Skipped,
        }
    }

    fn recover_partial_snapshot(info: &PartialSnapshotInfo) -> RecoveryResult {
        match info.recommended_action {
            RecoveryAction::Cleanup => RecoveryResult::Success,
            _ => RecoveryResult::Skipped,
        }
    }

    fn recover_uncommitted_events(info: &UncommittedEventsInfo) -> RecoveryResult {
        match info.recommended_action {
            RecoveryAction::Rollback => RecoveryResult::Success,
            _ => RecoveryResult::Skipped,
        }
    }
}

/// Result of a recovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryResult {
    /// Recovery succeeded
    Success,
    /// Recovery was skipped (not needed or not applicable)
    Skipped,
    /// Recovery requires user confirmation
    RequiresConfirmation(String),
    /// Recovery failed
    Failed(String),
}

impl RecoveryResult {
    /// Check if recovery was successful
    pub fn is_success(&self) -> bool {
        matches!(self, RecoveryResult::Success)
    }

    /// Check if confirmation is required
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, RecoveryResult::RequiresConfirmation(_))
    }
}

/// Complete recovery report
#[derive(Debug, Clone, Default)]
pub struct RecoveryReport {
    pub states: Vec<RecoveryState>,
    pub results: Vec<RecoveryResult>,
    pub all_successful: bool,
}

impl RecoveryReport {
    /// Create a new empty report
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a recovery result
    pub fn add_result(&mut self, state: RecoveryState, result: RecoveryResult) {
        self.states.push(state);
        self.results.push(result);
    }

    /// Finalize the report
    pub fn finalize(&mut self) {
        self.all_successful = self.results.iter().all(|r| r.is_success());
    }

    /// Get count of states requiring confirmation
    pub fn confirmation_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.requires_confirmation())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_state_clean() {
        let state = RecoveryState::Clean;
        assert!(state.is_clean());
        assert_eq!(state.recovery_action(), RecoveryAction::None);
    }

    #[test]
    fn test_recovery_state_incomplete_transaction() {
        let info = IncompleteTransactionInfo {
            transaction_id: "tx-123".to_string(),
            started_at: 1000,
            affected_session_id: Some("session-1".to_string()),
            affected_branch_id: None,
            recommended_action: RecoveryAction::Rollback,
        };
        let state = RecoveryState::IncompleteTransaction(info);
        assert!(!state.is_clean());
        assert_eq!(state.recovery_action(), RecoveryAction::Rollback);
    }

    #[test]
    fn test_recovery_state_interrupted_restore() {
        let info = InterruptedRestoreInfo {
            restore_id: "restore-1".to_string(),
            target_checkpoint_id: "cp-1".to_string(),
            started_at: 1000,
            phase: RestorePhase::ApplyingChanges,
            files_modified: vec!["file1.txt".to_string()],
            recommended_action: RecoveryAction::Cleanup,
        };
        let state = RecoveryState::InterruptedRestore(info);
        assert!(!state.is_clean());
        assert_eq!(state.recovery_action(), RecoveryAction::Cleanup);
    }

    #[test]
    fn test_recovery_state_partial_snapshot() {
        let info = PartialSnapshotInfo {
            snapshot_id: "snap-1".to_string(),
            session_id: "session-1".to_string(),
            started_at: 1000,
            files_scanned: 50,
            total_files: Some(100),
            recommended_action: RecoveryAction::Cleanup,
        };
        let state = RecoveryState::PartialSnapshot(info);
        assert!(!state.is_clean());
    }

    #[test]
    fn test_recovery_state_uncommitted_events() {
        let info = UncommittedEventsInfo {
            session_id: "session-1".to_string(),
            branch_id: "branch-1".to_string(),
            first_uncommitted_sequence: 100,
            last_committed_sequence: 95,
            uncommitted_count: 5,
            recommended_action: RecoveryAction::Rollback,
        };
        let state = RecoveryState::UncommittedEvents(info);
        assert!(!state.is_clean());
    }

    #[test]
    fn test_recovery_detector_is_clean() {
        assert!(RecoveryDetector::is_clean(&[], &[], &[], &[]));
        assert!(!RecoveryDetector::is_clean(
            &[IncompleteTransactionInfo {
                transaction_id: "tx-1".to_string(),
                started_at: 0,
                affected_session_id: None,
                affected_branch_id: None,
                recommended_action: RecoveryAction::Rollback,
            }],
            &[],
            &[],
            &[]
        ));
    }

    #[test]
    fn test_recovery_detector_detect_multiple_states() {
        let incomplete = vec![RecoveryDetector::create_incomplete_transaction(
            "tx-1", None, None,
        )];
        let uncommitted = vec![RecoveryDetector::create_uncommitted_events(
            "s1", "b1", 100, 95,
        )];

        let states = RecoveryDetector::detect_recovery_state(&incomplete, &[], &[], &uncommitted);
        assert_eq!(states.len(), 2);
        assert!(matches!(states[0], RecoveryState::IncompleteTransaction(_)));
        assert!(matches!(states[1], RecoveryState::UncommittedEvents(_)));
    }

    #[test]
    fn test_recovery_result_is_success() {
        assert!(RecoveryResult::Success.is_success());
        assert!(!RecoveryResult::Skipped.is_success());
        assert!(!RecoveryResult::RequiresConfirmation("x".to_string()).is_success());
        assert!(!RecoveryResult::Failed("err".to_string()).is_success());
    }

    #[test]
    fn test_recovery_result_requires_confirmation() {
        assert!(!RecoveryResult::Success.requires_confirmation());
        assert!(
            RecoveryResult::RequiresConfirmation("restore-1".to_string()).requires_confirmation()
        );
    }

    #[test]
    fn test_recovery_executor_clean() {
        let result = RecoveryExecutor::execute(&RecoveryState::Clean);
        assert!(result.is_success());
    }

    #[test]
    fn test_recovery_executor_incomplete_transaction() {
        let info = IncompleteTransactionInfo {
            transaction_id: "tx-1".to_string(),
            started_at: 0,
            affected_session_id: None,
            affected_branch_id: None,
            recommended_action: RecoveryAction::Rollback,
        };
        let result = RecoveryExecutor::execute(&RecoveryState::IncompleteTransaction(info));
        assert!(result.is_success());
    }

    #[test]
    fn test_recovery_executor_interrupted_restore_needs_confirmation() {
        let info = InterruptedRestoreInfo {
            restore_id: "restore-1".to_string(),
            target_checkpoint_id: "cp-1".to_string(),
            started_at: 0,
            phase: RestorePhase::ApplyingChanges,
            files_modified: vec![],
            recommended_action: RecoveryAction::UserConfirmation,
        };
        let result = RecoveryExecutor::execute(&RecoveryState::InterruptedRestore(info));
        assert!(result.requires_confirmation());
    }

    #[test]
    fn test_recovery_report_finalize() {
        let mut report = RecoveryReport::new();
        report.add_result(RecoveryState::Clean, RecoveryResult::Success);
        report.finalize();
        assert!(report.all_successful);
        assert_eq!(report.confirmation_count(), 0);
    }

    #[test]
    fn test_recovery_report_with_failures() {
        let mut report = RecoveryReport::new();
        report.add_result(RecoveryState::Clean, RecoveryResult::Success);
        report.add_result(
            RecoveryState::Clean,
            RecoveryResult::Failed("test failure".to_string()),
        );
        report.finalize();
        assert!(!report.all_successful);
    }

    #[test]
    fn test_recovery_report_confirmation_count() {
        let mut report = RecoveryReport::new();
        report.add_result(
            RecoveryState::InterruptedRestore(InterruptedRestoreInfo {
                restore_id: "r1".to_string(),
                target_checkpoint_id: "c1".to_string(),
                started_at: 0,
                phase: RestorePhase::ApplyingChanges,
                files_modified: vec![],
                recommended_action: RecoveryAction::UserConfirmation,
            }),
            RecoveryResult::RequiresConfirmation("r1".to_string()),
        );
        report.finalize();
        assert_eq!(report.confirmation_count(), 1);
    }

    #[test]
    fn test_restore_phase_default() {
        assert_eq!(RestorePhase::default(), RestorePhase::Preparing);
    }

    #[test]
    fn test_restore_phase_serialization() {
        let phase = RestorePhase::ApplyingChanges;
        let json = serde_json::to_string(&phase).unwrap();
        let parsed: RestorePhase = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, RestorePhase::ApplyingChanges);
    }

    #[test]
    fn test_recovery_action_default() {
        assert_eq!(RecoveryAction::default(), RecoveryAction::None);
    }

    #[test]
    fn test_create_incomplete_transaction() {
        let info = RecoveryDetector::create_incomplete_transaction(
            "tx-123",
            Some("session-1"),
            Some("branch-1"),
        );
        assert_eq!(info.transaction_id, "tx-123");
        assert_eq!(info.affected_session_id, Some("session-1".to_string()));
        assert_eq!(info.affected_branch_id, Some("branch-1".to_string()));
        assert_eq!(info.recommended_action, RecoveryAction::Rollback);
    }

    #[test]
    fn test_create_interrupted_restore() {
        let info = RecoveryDetector::create_interrupted_restore(
            "restore-1",
            "checkpoint-1",
            RestorePhase::Verifying,
            vec!["file1.rs".to_string(), "file2.rs".to_string()],
        );
        assert_eq!(info.restore_id, "restore-1");
        assert_eq!(info.target_checkpoint_id, "checkpoint-1");
        assert_eq!(info.phase, RestorePhase::Verifying);
        assert_eq!(info.files_modified.len(), 2);
    }

    #[test]
    fn test_create_partial_snapshot() {
        let info = RecoveryDetector::create_partial_snapshot("snap-1", "session-1", 75, Some(100));
        assert_eq!(info.snapshot_id, "snap-1");
        assert_eq!(info.files_scanned, 75);
        assert_eq!(info.total_files, Some(100));
    }

    #[test]
    fn test_create_uncommitted_events() {
        let info = RecoveryDetector::create_uncommitted_events("session-1", "branch-1", 100, 95);
        assert_eq!(info.first_uncommitted_sequence, 100);
        assert_eq!(info.last_committed_sequence, 95);
        assert_eq!(info.uncommitted_count, 5);
    }
}
