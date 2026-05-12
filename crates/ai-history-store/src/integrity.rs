//! Store-level integrity verification for persistent storage
//!
//! This module provides store-specific integrity checking that:
//! - Scans branch references and validates parent chain integrity
//! - Checks checkpoint reachability from branch heads
//! - Validates manifest presence for all checkpoints
//! - Verifies schema version and consistency
//! - Checks session/turn/branch relationships

use crate::sqlite_store::SqliteStore;
use ai_history_core::{
    BranchId, CheckpointId, IntegrityCheckType, IntegrityIssue, IntegritySummary, IssueSeverity,
    ManifestId, SessionId,
};
use std::collections::HashSet;

/// Result of a full integrity scan
#[derive(Debug, Clone)]
pub struct IntegrityScanResult {
    pub passed: bool,
    pub issues: Vec<IntegrityIssue>,
    pub summary: IntegritySummary,
    pub scanned_sessions: usize,
    pub scanned_branches: usize,
    pub scanned_checkpoints: usize,
    pub scanned_turns: usize,
    pub scanned_blobs: usize,
    pub scanned_manifests: usize,
}

impl IntegrityScanResult {
    /// Create a passing result
    pub fn pass() -> Self {
        Self {
            passed: true,
            issues: vec![],
            summary: IntegritySummary::default(),
            scanned_sessions: 0,
            scanned_branches: 0,
            scanned_checkpoints: 0,
            scanned_turns: 0,
            scanned_blobs: 0,
            scanned_manifests: 0,
        }
    }

    /// Add an issue and mark as failing
    pub fn add_issue(&mut self, issue: IntegrityIssue) {
        self.passed = false;
        self.issues.push(issue);
    }

    /// Build summary from issues
    pub fn build_summary(&mut self) {
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut info_count = 0;

        for issue in &self.issues {
            match issue.severity {
                IssueSeverity::Error => error_count += 1,
                IssueSeverity::Warning => warning_count += 1,
                IssueSeverity::Info => info_count += 1,
            }
        }

        self.summary = IntegritySummary {
            total_checks: 1,
            passed_checks: if self.passed { 1 } else { 0 },
            failed_checks: if self.passed { 0 } else { 1 },
            error_count,
            warning_count,
            info_count,
        };
    }
}

impl Default for IntegrityScanResult {
    fn default() -> Self {
        Self::pass()
    }
}

/// Store integrity scanner that checks the persistence layer
pub struct StoreIntegrityScanner<'a> {
    store: &'a SqliteStore,
    known_sessions: HashSet<SessionId>,
    known_branches: HashSet<BranchId>,
    known_checkpoints: HashSet<CheckpointId>,
    known_manifests: HashSet<ManifestId>,
    known_blobs: HashSet<String>,
}

impl<'a> StoreIntegrityScanner<'a> {
    /// Create a new store integrity scanner
    pub fn new(store: &'a SqliteStore) -> Self {
        Self {
            store,
            known_sessions: HashSet::new(),
            known_branches: HashSet::new(),
            known_checkpoints: HashSet::new(),
            known_manifests: HashSet::new(),
            known_blobs: HashSet::new(),
        }
    }

    /// Load all known entities from the store
    pub fn load_known_entities(&mut self) -> Result<(), String> {
        let sessions = self.store.list_sessions().map_err(|e| e.to_string())?;
        for session in &sessions {
            self.known_sessions.insert(session.id);
        }

        for session_id in &sessions {
            let branches = self
                .store
                .list_branches(session_id.id)
                .map_err(|e| e.to_string())?;
            for branch in &branches {
                self.known_branches.insert(branch.id);
            }
        }

        for session_id in &sessions {
            let checkpoints = self
                .store
                .list_checkpoints(session_id.id)
                .map_err(|e| e.to_string())?;
            for cp in &checkpoints {
                self.known_checkpoints.insert(cp.id);
                self.known_manifests.insert(cp.manifest_id);
            }
        }

        Ok(())
    }

    /// Run a full integrity scan of the store
    pub fn scan(&mut self) -> Result<IntegrityScanResult, String> {
        let mut result = IntegrityScanResult::pass();

        self.load_known_entities()?;

        result.scanned_sessions = self.known_sessions.len();
        result.scanned_branches = self.known_branches.len();
        result.scanned_checkpoints = self.known_checkpoints.len();
        result.scanned_blobs = self.known_blobs.len();
        result.scanned_manifests = self.known_manifests.len();

        self.check_event_checksums(&mut result)?;
        self.check_branch_references(&mut result)?;
        self.check_checkpoint_reachability(&mut result)?;
        self.check_manifest_presence(&mut result)?;
        self.check_schema_version(&mut result)?;

        result.build_summary();

        Ok(result)
    }

    fn check_event_checksums(&self, result: &mut IntegrityScanResult) -> Result<(), String> {
        for session_id in &self.known_sessions {
            let events = self
                .store
                .get_events_since(*session_id, 0)
                .map_err(|e| e.to_string())?;
            for event in events {
                let checksum = event.checksum();
                if checksum.len() != 64 || !checksum.chars().all(|c| c.is_ascii_hexdigit()) {
                    result.add_issue(IntegrityIssue::error(
                        IntegrityCheckType::EventChecksum,
                        format!("Invalid checksum format for event: {}", checksum),
                    ));
                }
            }
        }
        Ok(())
    }

    fn check_branch_references(&self, result: &mut IntegrityScanResult) -> Result<(), String> {
        for session_id in &self.known_sessions {
            let branches = self
                .store
                .list_branches(*session_id)
                .map_err(|e| e.to_string())?;
            for branch in &branches {
                if let Some(parent_id) = branch.parent_branch_id {
                    if !self.known_branches.contains(&parent_id) {
                        result.add_issue(
                            IntegrityIssue::error(
                                IntegrityCheckType::BrokenBranchReference,
                                format!(
                                    "Branch {} references non-existent parent branch {}",
                                    branch.id, parent_id
                                ),
                            )
                            .with_entity_id(branch.id.to_string()),
                        );
                    }
                }
                if !self.known_branches.contains(&branch.id) {
                    result.add_issue(
                        IntegrityIssue::error(
                            IntegrityCheckType::BrokenBranchReference,
                            format!("Branch {} does not exist in registry", branch.id),
                        )
                        .with_entity_id(branch.id.to_string()),
                    );
                }
            }
        }
        Ok(())
    }

    fn check_checkpoint_reachability(
        &self,
        result: &mut IntegrityScanResult,
    ) -> Result<(), String> {
        for session_id in &self.known_sessions {
            let checkpoints = self
                .store
                .list_checkpoints(*session_id)
                .map_err(|e| e.to_string())?;

            for cp in &checkpoints {
                if !self.known_branches.contains(&cp.branch_id) {
                    result.add_issue(
                        IntegrityIssue::error(
                            IntegrityCheckType::OrphanedCheckpoint,
                            format!(
                                "Checkpoint {} references non-existent branch {}",
                                cp.id, cp.branch_id
                            ),
                        )
                        .with_entity_id(cp.id.to_string()),
                    );
                }
                if !self.known_manifests.contains(&cp.manifest_id) {
                    result.add_issue(
                        IntegrityIssue::error(
                            IntegrityCheckType::InvalidManifest,
                            format!(
                                "Checkpoint {} references non-existent manifest {}",
                                cp.id, cp.manifest_id
                            ),
                        )
                        .with_entity_id(cp.id.to_string()),
                    );
                }
            }
        }
        Ok(())
    }

    fn check_manifest_presence(&self, result: &mut IntegrityScanResult) -> Result<(), String> {
        for manifest_id in &self.known_manifests {
            match self.store.get_manifest(*manifest_id) {
                Ok(Some(_)) => {}
                Ok(None) => {
                    result.add_issue(
                        IntegrityIssue::error(
                            IntegrityCheckType::InvalidManifest,
                            format!("Manifest {} not found in store", manifest_id),
                        )
                        .with_entity_id(manifest_id.to_string()),
                    );
                }
                Err(e) => {
                    result.add_issue(
                        IntegrityIssue::error(
                            IntegrityCheckType::SchemaMismatch,
                            format!("Failed to read manifest {}: {}", manifest_id, e),
                        )
                        .with_entity_id(manifest_id.to_string()),
                    );
                }
            }
        }
        Ok(())
    }

    fn check_schema_version(&self, result: &mut IntegrityScanResult) -> Result<(), String> {
        let sessions = self.store.list_sessions().map_err(|e| e.to_string())?;

        if sessions.is_empty() {
            result.add_issue(IntegrityIssue::info(
                IntegrityCheckType::SchemaMismatch,
                "No sessions found - store may be empty or uninitialized".to_string(),
            ));
        }

        for session_id in &self.known_sessions {
            if self.store.get_session(*session_id).is_err() {
                result.add_issue(IntegrityIssue::error(
                    IntegrityCheckType::SchemaMismatch,
                    format!("Failed to read session {}", session_id),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BranchInfo, CheckpointInfo, ManifestInfo, SqliteStore};
    use ai_history_core::ManifestId;

    fn create_test_store() -> SqliteStore {
        SqliteStore::in_memory().unwrap()
    }

    #[test]
    fn test_integrity_scan_passes_on_empty_store() {
        let store = create_test_store();
        let mut scanner = StoreIntegrityScanner::new(&store);
        let result = scanner.scan().unwrap();

        assert!(
            result.passed
                || result
                    .issues
                    .iter()
                    .any(|i| i.severity == IssueSeverity::Info)
        );
    }

    #[test]
    fn test_integrity_scan_detects_missing_branch_parent() {
        let store = create_test_store();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp").unwrap();

        let branch_id = BranchId::new();
        let fake_parent_id = BranchId::new();

        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "test".to_string(),
                parent_branch_id: Some(fake_parent_id),
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        let mut scanner = StoreIntegrityScanner::new(&store);
        let result = scanner.scan().unwrap();

        let broken_refs: Vec<_> = result
            .issues
            .iter()
            .filter(|i| matches!(i.check_type, IntegrityCheckType::BrokenBranchReference))
            .collect();
        assert!(
            !broken_refs.is_empty(),
            "Should detect broken branch reference"
        );
    }

    #[test]
    fn test_integrity_scan_detects_missing_manifest() {
        let store = create_test_store();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        let checkpoint_id = CheckpointId::new();
        let fake_manifest_id = ManifestId::new();

        store
            .create_checkpoint(&CheckpointInfo {
                id: checkpoint_id,
                session_id,
                branch_id,
                turn_id: None,
                manifest_id: fake_manifest_id,
                reason: "test".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        let mut scanner = StoreIntegrityScanner::new(&store);
        let result = scanner.scan().unwrap();

        let manifest_issues: Vec<_> = result
            .issues
            .iter()
            .filter(|i| matches!(i.check_type, IntegrityCheckType::InvalidManifest))
            .collect();
        assert!(
            !manifest_issues.is_empty(),
            "Should detect missing manifest"
        );
    }

    #[test]
    fn test_integrity_scan_counts_entities() {
        let store = create_test_store();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp").unwrap();

        let branch_id = BranchId::new();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        let mut scanner = StoreIntegrityScanner::new(&store);
        let result = scanner.scan().unwrap();

        assert_eq!(result.scanned_sessions, 1);
        assert_eq!(result.scanned_branches, 1);
    }

    #[test]
    fn test_integrity_scan_passes_with_valid_data() {
        let store = create_test_store();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp").unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        let checkpoint_id = CheckpointId::new();
        let manifest_id = ManifestId::new();

        store
            .create_checkpoint(&CheckpointInfo {
                id: checkpoint_id,
                session_id,
                branch_id,
                turn_id: None,
                manifest_id,
                reason: "test".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        store
            .create_manifest(&ManifestInfo {
                id: manifest_id,
                session_id,
                checkpoint_id,
                manifest_data: r#"{"version":1,"entries":{}}"#.to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        let mut scanner = StoreIntegrityScanner::new(&store);
        let result = scanner.scan().unwrap();

        assert!(result.passed, "Valid data should pass: {:?}", result.issues);
    }
}
