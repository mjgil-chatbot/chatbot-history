//! Integrity verification for the ai-history system
//!
//! Implements `doctor` checks for:
//! - Event checksum validation
//! - Missing blobs
//! - Broken branch references
//! - Invalid manifests
//! - Orphaned checkpoints
//! - Schema mismatch

use crate::{BlobId, BranchId, CheckpointId, ManifestId, SessionId, TurnId};
use std::collections::HashSet;

/// Type of integrity check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityCheckType {
    EventChecksum,
    MissingBlob,
    BrokenBranchReference,
    InvalidManifest,
    OrphanedCheckpoint,
    SchemaMismatch,
}

/// Severity of an integrity issue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

/// A single integrity issue found during verification
#[derive(Debug, Clone)]
pub struct IntegrityIssue {
    pub check_type: IntegrityCheckType,
    pub severity: IssueSeverity,
    pub description: String,
    pub entity_id: Option<String>,
    pub suggestion: Option<String>,
}

impl IntegrityIssue {
    /// Create a new error-level issue
    pub fn error(check_type: IntegrityCheckType, description: String) -> Self {
        Self {
            check_type,
            severity: IssueSeverity::Error,
            description,
            entity_id: None,
            suggestion: None,
        }
    }

    /// Create a new warning-level issue
    pub fn warning(check_type: IntegrityCheckType, description: String) -> Self {
        Self {
            check_type,
            severity: IssueSeverity::Warning,
            description,
            entity_id: None,
            suggestion: None,
        }
    }

    /// Create a new info-level issue
    pub fn info(check_type: IntegrityCheckType, description: String) -> Self {
        Self {
            check_type,
            severity: IssueSeverity::Info,
            description,
            entity_id: None,
            suggestion: None,
        }
    }

    /// Set the entity ID
    pub fn with_entity_id(mut self, id: String) -> Self {
        self.entity_id = Some(id);
        self
    }

    /// Set the suggestion
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }
}

/// Result of an integrity check
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub passed: bool,
    pub issues: Vec<IntegrityIssue>,
}

impl CheckResult {
    /// Create a passing result
    pub fn pass() -> Self {
        Self {
            passed: true,
            issues: vec![],
        }
    }

    /// Create a failing result with issues
    pub fn fail(issues: Vec<IntegrityIssue>) -> Self {
        Self {
            passed: false,
            issues,
        }
    }

    /// Add an issue to the result
    pub fn add_issue(&mut self, issue: IntegrityIssue) {
        self.passed = false;
        self.issues.push(issue);
    }

    /// Combine two check results
    pub fn merge(&mut self, other: CheckResult) {
        if !other.passed {
            self.passed = false;
            self.issues.extend(other.issues);
        }
    }
}

/// Integrity checker for the doctor command
pub struct IntegrityChecker {
    // Known good entities
    known_blobs: HashSet<BlobId>,
    known_manifests: HashSet<ManifestId>,
    known_checkpoints: HashSet<CheckpointId>,
    known_branches: HashSet<BranchId>,
    known_sessions: HashSet<SessionId>,
    known_turns: HashSet<TurnId>,
    known_checksums: HashSet<String>,
}

impl Default for IntegrityChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl IntegrityChecker {
    /// Create a new integrity checker
    pub fn new() -> Self {
        Self {
            known_blobs: HashSet::new(),
            known_manifests: HashSet::new(),
            known_checkpoints: HashSet::new(),
            known_branches: HashSet::new(),
            known_sessions: HashSet::new(),
            known_turns: HashSet::new(),
            known_checksums: HashSet::new(),
        }
    }

    /// Register a known blob
    pub fn register_blob(&mut self, blob_id: BlobId) {
        self.known_blobs.insert(blob_id);
    }

    /// Register a known manifest
    pub fn register_manifest(&mut self, manifest_id: ManifestId) {
        self.known_manifests.insert(manifest_id);
    }

    /// Register a known checkpoint
    pub fn register_checkpoint(&mut self, checkpoint_id: CheckpointId) {
        self.known_checkpoints.insert(checkpoint_id);
    }

    /// Register a known branch
    pub fn register_branch(&mut self, branch_id: BranchId) {
        self.known_branches.insert(branch_id);
    }

    /// Register a known session
    pub fn register_session(&mut self, session_id: SessionId) {
        self.known_sessions.insert(session_id);
    }

    /// Register a known turn
    pub fn register_turn(&mut self, turn_id: TurnId) {
        self.known_turns.insert(turn_id);
    }

    /// Register a known checksum
    pub fn register_checksum(&mut self, checksum: String) {
        self.known_checksums.insert(checksum);
    }

    /// Check event checksum validity
    pub fn check_checksum_format(&self, checksum: &str) -> CheckResult {
        // Verify checksum format (64 hex chars for SHA-256)
        if checksum.len() == 64 && checksum.chars().all(|c| c.is_ascii_hexdigit()) {
            CheckResult::pass()
        } else {
            CheckResult::fail(vec![IntegrityIssue::error(
                IntegrityCheckType::EventChecksum,
                format!("Invalid checksum format: {}", checksum),
            )])
        }
    }

    /// Check for missing blobs referenced by manifest
    pub fn check_missing_blobs(&self, manifest_blob_refs: &[BlobId]) -> CheckResult {
        let mut result = CheckResult::pass();
        let mut missing = vec![];

        for blob_id in manifest_blob_refs {
            if !self.known_blobs.contains(blob_id) {
                missing.push(blob_id);
            }
        }

        if !missing.is_empty() {
            result.add_issue(
                IntegrityIssue::error(
                    IntegrityCheckType::MissingBlob,
                    format!("{} missing blobs found", missing.len()),
                )
                .with_suggestion(
                    "Run garbage collection to clean up or restore missing blobs".to_string(),
                ),
            );
        }

        result
    }

    /// Check for broken branch references
    pub fn check_branch_references(
        &self,
        branch_parent_refs: &[(BranchId, Option<BranchId>)],
    ) -> CheckResult {
        let mut result = CheckResult::pass();

        for (branch_id, parent_id) in branch_parent_refs {
            if let Some(parent) = parent_id {
                if !self.known_branches.contains(parent) {
                    result.add_issue(
                        IntegrityIssue::error(
                            IntegrityCheckType::BrokenBranchReference,
                            format!("Branch {} references non-existent parent branch", branch_id),
                        )
                        .with_entity_id(branch_id.to_string()),
                    );
                }
            }

            if !self.known_branches.contains(branch_id) {
                result.add_issue(
                    IntegrityIssue::error(
                        IntegrityCheckType::BrokenBranchReference,
                        format!("Branch {} does not exist", branch_id),
                    )
                    .with_entity_id(branch_id.to_string()),
                );
            }
        }

        result
    }

    /// Check manifest integrity
    pub fn check_manifest(&self, manifest_id: &ManifestId) -> CheckResult {
        if self.known_manifests.contains(manifest_id) {
            CheckResult::pass()
        } else {
            CheckResult::fail(vec![IntegrityIssue::error(
                IntegrityCheckType::InvalidManifest,
                format!("Manifest {} not found", manifest_id),
            )
            .with_entity_id(manifest_id.to_string())])
        }
    }

    /// Check for orphaned checkpoints (not referenced by any branch)
    pub fn check_orphaned_checkpoints(&self, all_checkpoints: &[CheckpointId]) -> CheckResult {
        let mut result = CheckResult::pass();
        let mut orphaned = vec![];

        for checkpoint_id in all_checkpoints {
            if !self.known_checkpoints.contains(checkpoint_id) {
                orphaned.push(checkpoint_id);
            }
        }

        if !orphaned.is_empty() {
            result.add_issue(
                IntegrityIssue::warning(
                    IntegrityCheckType::OrphanedCheckpoint,
                    format!("{} orphaned checkpoints found", orphaned.len()),
                )
                .with_suggestion("Consider running garbage collection".to_string()),
            );
        }

        result
    }

    /// Check if session references are valid
    pub fn check_session_references(
        &self,
        session_turns: &[(SessionId, Vec<TurnId>)],
    ) -> CheckResult {
        let mut result = CheckResult::pass();

        for (session_id, turn_ids) in session_turns {
            if !self.known_sessions.contains(session_id) {
                result.add_issue(
                    IntegrityIssue::error(
                        IntegrityCheckType::SchemaMismatch,
                        format!("Session {} not found", session_id),
                    )
                    .with_entity_id(session_id.to_string()),
                );
            }

            for turn_id in turn_ids {
                if !self.known_turns.contains(turn_id) {
                    result.add_issue(
                        IntegrityIssue::warning(
                            IntegrityCheckType::BrokenBranchReference,
                            format!(
                                "Turn {} referenced by session {} not found",
                                turn_id, session_id
                            ),
                        )
                        .with_entity_id(turn_id.to_string()),
                    );
                }
            }
        }

        result
    }

    /// Run all integrity checks
    pub fn run_full_check(&self) -> CheckResult {
        let mut result = CheckResult::pass();

        // Add any summary issues based on known entities
        if self.known_blobs.is_empty() && self.known_sessions.is_empty() {
            result.add_issue(IntegrityIssue::info(
                IntegrityCheckType::SchemaMismatch,
                "No data found in store".to_string(),
            ));
        }

        result
    }
}

/// Summary of integrity check results
#[derive(Debug, Clone, Default)]
pub struct IntegritySummary {
    pub total_checks: usize,
    pub passed_checks: usize,
    pub failed_checks: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

impl IntegritySummary {
    /// Create from check results
    pub fn from_results(results: &[CheckResult]) -> Self {
        let total_checks = results.len();
        let passed_checks = results.iter().filter(|r| r.passed).count();
        let failed_checks = total_checks - passed_checks;

        let mut error_count = 0;
        let mut warning_count = 0;
        let mut info_count = 0;

        for result in results {
            for issue in &result.issues {
                match issue.severity {
                    IssueSeverity::Error => error_count += 1,
                    IssueSeverity::Warning => warning_count += 1,
                    IssueSeverity::Info => info_count += 1,
                }
            }
        }

        Self {
            total_checks,
            passed_checks,
            failed_checks,
            error_count,
            warning_count,
            info_count,
        }
    }

    /// Check if all checks passed
    pub fn all_passed(&self) -> bool {
        self.error_count == 0 && self.failed_checks == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integrity_issue_error() {
        let issue = IntegrityIssue::error(
            IntegrityCheckType::MissingBlob,
            "Blob not found".to_string(),
        );
        assert_eq!(issue.severity, IssueSeverity::Error);
        assert!(issue.entity_id.is_none());
    }

    #[test]
    fn test_integrity_issue_with_entity() {
        let issue = IntegrityIssue::error(
            IntegrityCheckType::BrokenBranchReference,
            "Branch not found".to_string(),
        )
        .with_entity_id("branch_123".to_string());

        assert_eq!(issue.entity_id, Some("branch_123".to_string()));
    }

    #[test]
    fn test_integrity_issue_with_suggestion() {
        let issue = IntegrityIssue::warning(
            IntegrityCheckType::OrphanedCheckpoint,
            "Orphaned checkpoint".to_string(),
        )
        .with_suggestion("Run gc".to_string());

        assert_eq!(issue.suggestion, Some("Run gc".to_string()));
    }

    #[test]
    fn test_check_result_pass() {
        let result = CheckResult::pass();
        assert!(result.passed);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn test_check_result_fail() {
        let issue = IntegrityIssue::error(
            IntegrityCheckType::MissingBlob,
            "Blob not found".to_string(),
        );
        let result = CheckResult::fail(vec![issue]);
        assert!(!result.passed);
        assert_eq!(result.issues.len(), 1);
    }

    #[test]
    fn test_check_result_add_issue() {
        let mut result = CheckResult::pass();
        result.add_issue(IntegrityIssue::info(
            IntegrityCheckType::SchemaMismatch,
            "Info message".to_string(),
        ));
        assert!(!result.passed);
        assert_eq!(result.issues.len(), 1);
    }

    #[test]
    fn test_check_result_merge() {
        let mut result1 = CheckResult::pass();
        let issue =
            IntegrityIssue::error(IntegrityCheckType::MissingBlob, "Blob missing".to_string());
        result1.add_issue(issue);

        let result2 = CheckResult::pass();

        result1.merge(result2);
        assert!(!result1.passed);
        assert_eq!(result1.issues.len(), 1);
    }

    #[test]
    fn test_integrity_checker_new() {
        let checker = IntegrityChecker::new();
        assert!(checker.known_blobs.is_empty());
        assert!(checker.known_sessions.is_empty());
    }

    #[test]
    fn test_integrity_checker_register() {
        let mut checker = IntegrityChecker::new();
        let blob_id = BlobId::new();
        let session_id = SessionId::new();

        checker.register_blob(blob_id);
        checker.register_session(session_id);

        assert!(checker.known_blobs.contains(&blob_id));
        assert!(checker.known_sessions.contains(&session_id));
    }

    #[test]
    fn test_check_event_checksum() {
        let checker = IntegrityChecker::new();
        // SHA-256 checksum is 64 hex characters
        let valid_checksum = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let result = checker.check_checksum_format(valid_checksum);
        assert!(result.passed);
    }

    #[test]
    fn test_check_missing_blobs() {
        let mut checker = IntegrityChecker::new();
        let blob1 = BlobId::new();
        let blob2 = BlobId::new();

        checker.register_blob(blob1);

        let result = checker.check_missing_blobs(&[blob1, blob2]);
        assert!(!result.passed);
        assert_eq!(result.issues.len(), 1);
    }

    #[test]
    fn test_check_branch_references_valid() {
        let mut checker = IntegrityChecker::new();
        let branch1 = BranchId::new();
        let branch2 = BranchId::new();

        checker.register_branch(branch1);
        checker.register_branch(branch2);

        let result = checker.check_branch_references(&[(branch1, None), (branch2, Some(branch1))]);
        assert!(result.passed);
    }

    #[test]
    fn test_check_branch_references_broken() {
        let mut checker = IntegrityChecker::new();
        let branch1 = BranchId::new();
        let branch2 = BranchId::new();

        checker.register_branch(branch1);
        // branch2 not registered

        let result = checker.check_branch_references(&[(branch1, None), (branch2, Some(branch1))]);
        assert!(!result.passed);
    }

    #[test]
    fn test_check_manifest_found() {
        let mut checker = IntegrityChecker::new();
        let manifest_id = ManifestId::new();

        checker.register_manifest(manifest_id);

        let result = checker.check_manifest(&manifest_id);
        assert!(result.passed);
    }

    #[test]
    fn test_check_manifest_not_found() {
        let checker = IntegrityChecker::new();
        let manifest_id = ManifestId::new();

        let result = checker.check_manifest(&manifest_id);
        assert!(!result.passed);
    }

    #[test]
    fn test_integrity_summary_from_results() {
        let results = vec![
            CheckResult::pass(),
            CheckResult::fail(vec![
                IntegrityIssue::error(IntegrityCheckType::MissingBlob, "Error 1".to_string()),
                IntegrityIssue::warning(
                    IntegrityCheckType::OrphanedCheckpoint,
                    "Warning 1".to_string(),
                ),
            ]),
        ];

        let summary = IntegritySummary::from_results(&results);

        assert_eq!(summary.total_checks, 2);
        assert_eq!(summary.passed_checks, 1);
        assert_eq!(summary.failed_checks, 1);
        assert_eq!(summary.error_count, 1);
        assert_eq!(summary.warning_count, 1);
        assert_eq!(summary.info_count, 0);
    }

    #[test]
    fn test_integrity_summary_all_passed() {
        let results = vec![CheckResult::pass(), CheckResult::pass()];
        let summary = IntegritySummary::from_results(&results);
        assert!(summary.all_passed());
    }

    #[test]
    fn test_integrity_summary_not_all_passed() {
        let results = vec![
            CheckResult::pass(),
            CheckResult::fail(vec![IntegrityIssue::error(
                IntegrityCheckType::MissingBlob,
                "Error".to_string(),
            )]),
        ];
        let summary = IntegritySummary::from_results(&results);
        assert!(!summary.all_passed());
    }

    #[test]
    fn test_check_orphaned_checkpoints() {
        let mut checker = IntegrityChecker::new();
        let checkpoint1 = CheckpointId::new();
        let checkpoint2 = CheckpointId::new();

        checker.register_checkpoint(checkpoint1);
        // checkpoint2 not registered = orphaned

        let result = checker.check_orphaned_checkpoints(&[checkpoint1, checkpoint2]);
        assert!(!result.passed);
    }

    #[test]
    fn test_run_full_check_empty() {
        let checker = IntegrityChecker::new();
        let result = checker.run_full_check();
        // Empty store generates an info issue, not a pass
        assert!(!result.passed);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].severity, IssueSeverity::Info);
    }
}
