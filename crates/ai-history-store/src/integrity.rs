//! Store-level integrity verification for persistent storage
//!
//! This module provides store-specific integrity checking that:
//! - Scans branch references and validates parent chain integrity
//! - Checks checkpoint reachability from branch heads
//! - Validates manifest presence for all checkpoints
//! - Verifies schema version and consistency
//! - Checks session/turn/branch relationships

use ai_history_core::IntegrityIssue;
use ai_history_core::IntegritySummary;
use ai_history_core::IssueSeverity;
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
    }}
impl IntegrityScanResult {
    /// Add an issue and mark as failing
    pub fn add_issue(&mut self, issue: IntegrityIssue) {
        self.passed = false;
        self.issues.push(issue);
    }}
impl IntegrityScanResult {
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
    }}
impl Default for IntegrityScanResult {
    fn default() -> Self {
        Self::pass()
    }
}
