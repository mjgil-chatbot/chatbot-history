//! Implementation status matrix for ai-history
//!
//! Tracks current-vs-target status for every design feature:
//! - Implemented, Partial, Placeholder, Missing, Deferred
//!
//! Status is derived from repository evidence (source files, tests, docs,
//! edge case markers) rather than trusting static completion declarations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::completion_gate::FilesystemEvidenceProvider;

/// Implementation status for a feature
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImplementationStatus {
    /// Fully implemented with real behavior
    Implemented,
    /// Partially implemented (core works but edge cases missing)
    Partial,
    /// Stub/placeholder implementation (returns hardcoded values)
    Placeholder,
    /// Not yet implemented
    Missing,
    /// Explicitly deferred with rationale
    Deferred,
}

impl ImplementationStatus {
    pub fn display_name(&self) -> &'static str {
        match self {
            ImplementationStatus::Implemented => "Implemented",
            ImplementationStatus::Partial => "Partial",
            ImplementationStatus::Placeholder => "Placeholder",
            ImplementationStatus::Missing => "Missing",
            ImplementationStatus::Deferred => "Deferred",
        }
    }

    pub fn is_sufficient(&self) -> bool {
        matches!(self, ImplementationStatus::Implemented)
    }
}

/// A feature in the implementation matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixEntry {
    pub task_id: u32,
    pub name: String,
    pub status: ImplementationStatus,
    pub blockers: Vec<String>,
    pub notes: Option<String>,
}

impl MatrixEntry {
    pub fn new(task_id: u32, name: &str) -> Self {
        Self {
            task_id,
            name: name.to_string(),
            status: ImplementationStatus::Missing,
            blockers: Vec::new(),
            notes: None,
        }
    }

    pub fn with_status(mut self, status: ImplementationStatus) -> Self {
        self.status = status;
        self
    }

    pub fn add_blocker(mut self, blocker: &str) -> Self {
        self.blockers.push(blocker.to_string());
        self
    }
}

/// Implementation status matrix
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImplementationMatrix {
    pub entries: HashMap<u32, MatrixEntry>,
    pub summary: MatrixSummary,
}

/// Summary statistics for the matrix
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatrixSummary {
    pub total: usize,
    pub implemented: usize,
    pub partial: usize,
    pub placeholder: usize,
    pub missing: usize,
    pub deferred: usize,
}

impl ImplementationMatrix {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            summary: MatrixSummary::default(),
        }
    }

    pub fn add_entry(&mut self, entry: MatrixEntry) {
        self.entries.insert(entry.task_id, entry);
        self.recalculate_summary();
    }

    pub fn get_entry(&self, task_id: u32) -> Option<&MatrixEntry> {
        self.entries.get(&task_id)
    }

    pub fn recalculate_summary(&mut self) {
        let mut summary = MatrixSummary::default();
        summary.total = self.entries.len();

        for entry in self.entries.values() {
            match entry.status {
                ImplementationStatus::Implemented => summary.implemented += 1,
                ImplementationStatus::Partial => summary.partial += 1,
                ImplementationStatus::Placeholder => summary.placeholder += 1,
                ImplementationStatus::Missing => summary.missing += 1,
                ImplementationStatus::Deferred => summary.deferred += 1,
            }
        }

        self.summary = summary;
    }

    /// Get entries with a specific status
    pub fn with_status(&self, status: ImplementationStatus) -> Vec<&MatrixEntry> {
        self.entries
            .values()
            .filter(|e| e.status == status)
            .collect()
    }

    /// Get entries that need work (not Implemented)
    pub fn needs_attention(&self) -> Vec<&MatrixEntry> {
        self.entries
            .values()
            .filter(|e| !e.status.is_sufficient())
            .collect()
    }

    /// Calculate completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.summary.total == 0 {
            return 100.0;
        }
        (self.summary.implemented as f64 / self.summary.total as f64) * 100.0
    }
}

/// Feature completeness checker
pub struct CompletenessChecker;

impl CompletenessChecker {
    /// Check if a feature marked Complete has real implementation
    pub fn has_real_implementation(entry: &MatrixEntry) -> bool {
        match entry.status {
            ImplementationStatus::Implemented => true,
            ImplementationStatus::Partial
            | ImplementationStatus::Placeholder
            | ImplementationStatus::Missing
            | ImplementationStatus::Deferred => false,
        }
    }

    /// Check if a feature has tests
    pub fn has_tests(entry: &MatrixEntry) -> bool {
        // Features with Missing/Deferred status don't need tests yet
        matches!(
            entry.status,
            ImplementationStatus::Implemented
                | ImplementationStatus::Partial
                | ImplementationStatus::Placeholder
        )
    }

    /// Check if a feature has blockers
    pub fn has_blockers(entry: &MatrixEntry) -> bool {
        !entry.blockers.is_empty()
    }
}

/// Task doc analyzer result
#[derive(Debug, Clone)]
pub struct TaskDocAnalysis {
    pub total_tasks: usize,
    pub complete_count: usize,
    pub incomplete_count: usize,
    pub overclaimed_tasks: Vec<OverclaimedTask>,
}

/// An overclaimed task is marked Complete but has placeholder/missing implementation
#[derive(Debug, Clone)]
pub struct OverclaimedTask {
    pub task_id: u32,
    pub name: String,
    pub current_status: String,
    pub should_be_status: ImplementationStatus,
    pub reason: String,
}

impl TaskDocAnalysis {
    pub fn new() -> Self {
        Self {
            total_tasks: 0,
            complete_count: 0,
            incomplete_count: 0,
            overclaimed_tasks: Vec::new(),
        }
    }

    pub fn add_overclaim(&mut self, task: OverclaimedTask) {
        self.overclaimed_tasks.push(task);
    }

    pub fn has_overclaims(&self) -> bool {
        !self.overclaimed_tasks.is_empty()
    }
}

/// Scan a checklist document for completion status
pub struct TaskDocScanner;

impl TaskDocScanner {
    /// Scan a checklist document line by line to find completion status
    pub fn scan_task_doc(content: &str) -> TaskDocAnalysis {
        let mut analysis = TaskDocAnalysis::new();
        let mut in_table = false;
        let mut row_count = 0;

        for line in content.lines() {
            // Find table header
            if line.contains("| Status") || line.contains("|Status") {
                in_table = true;
                continue;
            }

            // End of table (empty line or non-table content)
            if in_table && (line.trim().is_empty() || !line.contains('|')) {
                if row_count > 0 {
                    break;
                }
            }

            // Skip header row
            if in_table && line.starts_with("| --") {
                continue;
            }

            if in_table && line.contains('|') {
                row_count += 1;
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 3 {
                    // Parse task ID and name from first column
                    let status_col = parts[2].trim();

                    if status_col == "Complete" {
                        analysis.complete_count += 1;
                    } else if status_col == "Incomplete" || status_col == "Partial" {
                        analysis.incomplete_count += 1;
                    }
                }
            }
        }

        analysis.total_tasks = row_count;
        analysis
    }

    /// Scan with repository evidence verification to detect overclaimed tasks.
    /// A task is overclaimed when its Status column says "Complete" but
    /// filesystem evidence (source, tests, docs) does not actually exist.
    pub fn scan_with_evidence(content: &str, workspace_root: PathBuf) -> TaskDocAnalysis {
        let evidence = FilesystemEvidenceProvider::new(workspace_root);
        let mut analysis = TaskDocAnalysis::new();
        let mut in_table = false;
        let mut row_count = 0;

        for line in content.lines() {
            if line.contains("| Status") || line.contains("|Status") {
                in_table = true;
                continue;
            }

            if in_table && (line.trim().is_empty() || !line.contains('|')) {
                if row_count > 0 {
                    break;
                }
            }

            if in_table && line.starts_with("| --") {
                continue;
            }

            if in_table && line.contains('|') {
                row_count += 1;
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 3 {
                    let task_col = parts[1].trim();
                    let status_col = parts[2].trim();
                    let (task_id, task_name) = Self::parse_task_column(task_col);

                    if status_col == "Complete" {
                        analysis.complete_count += 1;

                        // Check if this "Complete" task has real evidence
                        let report = evidence.evidence_report(&task_name);
                        if !report.source_found && !report.tests_found {
                            // Task claims Complete but has no source or test evidence
                            analysis.add_overclaim(OverclaimedTask {
                                task_id,
                                name: task_name.clone(),
                                current_status: "Complete".to_string(),
                                should_be_status: ImplementationStatus::Placeholder,
                                reason: format!(
                                    "No source files or test files found for '{}' (source={}, tests={})",
                                    task_name, report.source_found, report.tests_found
                                ),
                            });
                        } else if !report.source_found {
                            analysis.add_overclaim(OverclaimedTask {
                                task_id,
                                name: task_name.clone(),
                                current_status: "Complete".to_string(),
                                should_be_status: ImplementationStatus::Partial,
                                reason: format!(
                                    "No source files found for '{}' (tests={})",
                                    task_name, report.tests_found
                                ),
                            });
                        } else if !report.tests_found {
                            analysis.add_overclaim(OverclaimedTask {
                                task_id,
                                name: task_name.clone(),
                                current_status: "Complete".to_string(),
                                should_be_status: ImplementationStatus::Partial,
                                reason: format!(
                                    "No test files found for '{}' (source={})",
                                    task_name, report.source_found
                                ),
                            });
                        }
                        // If source AND tests found, no overclaim
                    } else if status_col == "Incomplete" || status_col == "Partial" {
                        analysis.incomplete_count += 1;
                    }
                }
            }
        }

        analysis.total_tasks = row_count;
        analysis
    }

    /// Parse task ID and name from task column
    fn parse_task_column(task_col: &str) -> (u32, String) {
        let trimmed = task_col.trim();

        if let Some(dot_pos) = trimmed.find('.') {
            if let Ok(id) = trimmed[..dot_pos].parse::<u32>() {
                let name = trimmed[dot_pos + 1..].trim().to_string();
                return (id, name);
            }
        }

        let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
        let id = digits.parse::<u32>().unwrap_or(0);
        let name = trimmed.trim().to_string();

        (id, name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn test_implementation_status_display() {
        assert_eq!(
            ImplementationStatus::Implemented.display_name(),
            "Implemented"
        );
        assert_eq!(ImplementationStatus::Partial.display_name(), "Partial");
        assert_eq!(
            ImplementationStatus::Placeholder.display_name(),
            "Placeholder"
        );
        assert_eq!(ImplementationStatus::Missing.display_name(), "Missing");
        assert_eq!(ImplementationStatus::Deferred.display_name(), "Deferred");
    }

    #[test]
    fn test_implementation_status_is_sufficient() {
        assert!(ImplementationStatus::Implemented.is_sufficient());
        assert!(!ImplementationStatus::Partial.is_sufficient());
        assert!(!ImplementationStatus::Placeholder.is_sufficient());
        assert!(!ImplementationStatus::Missing.is_sufficient());
        assert!(!ImplementationStatus::Deferred.is_sufficient());
    }

    #[test]
    fn test_matrix_entry_new() {
        let entry = MatrixEntry::new(1, "Test Feature");
        assert_eq!(entry.task_id, 1);
        assert_eq!(entry.name, "Test Feature");
        assert_eq!(entry.status, ImplementationStatus::Missing);
        assert!(entry.blockers.is_empty());
    }

    #[test]
    fn test_matrix_entry_with_status() {
        let entry = MatrixEntry::new(1, "Test").with_status(ImplementationStatus::Implemented);
        assert_eq!(entry.status, ImplementationStatus::Implemented);
    }

    #[test]
    fn test_matrix_entry_add_blocker() {
        let entry = MatrixEntry::new(1, "Test").add_blocker("needs SQLite");
        assert_eq!(entry.blockers.len(), 1);
        assert_eq!(entry.blockers[0], "needs SQLite");
    }

    #[test]
    fn test_implementation_matrix_new() {
        let matrix = ImplementationMatrix::new();
        assert_eq!(matrix.entries.len(), 0);
        assert_eq!(matrix.summary.total, 0);
    }

    #[test]
    fn test_implementation_matrix_add_entry() {
        let mut matrix = ImplementationMatrix::new();
        matrix
            .add_entry(MatrixEntry::new(1, "Test").with_status(ImplementationStatus::Implemented));
        assert_eq!(matrix.entries.len(), 1);
        assert_eq!(matrix.summary.total, 1);
        assert_eq!(matrix.summary.implemented, 1);
    }

    #[test]
    fn test_implementation_matrix_get_entry() {
        let mut matrix = ImplementationMatrix::new();
        matrix.add_entry(MatrixEntry::new(1, "Test"));
        assert!(matrix.get_entry(1).is_some());
        assert!(matrix.get_entry(99).is_none());
    }

    #[test]
    fn test_implementation_matrix_with_status() {
        let mut matrix = ImplementationMatrix::new();
        matrix.add_entry(MatrixEntry::new(1, "A").with_status(ImplementationStatus::Implemented));
        matrix.add_entry(MatrixEntry::new(2, "B").with_status(ImplementationStatus::Placeholder));
        matrix.add_entry(MatrixEntry::new(3, "C").with_status(ImplementationStatus::Missing));

        let implemented = matrix.with_status(ImplementationStatus::Implemented);
        assert_eq!(implemented.len(), 1);
        assert_eq!(implemented[0].name, "A");
    }

    #[test]
    fn test_implementation_matrix_needs_attention() {
        let mut matrix = ImplementationMatrix::new();
        matrix.add_entry(MatrixEntry::new(1, "A").with_status(ImplementationStatus::Implemented));
        matrix.add_entry(MatrixEntry::new(2, "B").with_status(ImplementationStatus::Placeholder));

        let needs_work = matrix.needs_attention();
        assert_eq!(needs_work.len(), 1);
        assert_eq!(needs_work[0].name, "B");
    }

    #[test]
    fn test_implementation_matrix_completion_percentage() {
        let mut matrix = ImplementationMatrix::new();
        matrix.add_entry(MatrixEntry::new(1, "A").with_status(ImplementationStatus::Implemented));
        matrix.add_entry(MatrixEntry::new(2, "B").with_status(ImplementationStatus::Implemented));
        matrix.add_entry(MatrixEntry::new(3, "C").with_status(ImplementationStatus::Missing));
        matrix.add_entry(MatrixEntry::new(4, "D").with_status(ImplementationStatus::Missing));

        // 2 out of 4 implemented = 50%
        assert_eq!(matrix.completion_percentage(), 50.0);
    }

    #[test]
    fn test_implementation_matrix_empty_completion() {
        let matrix = ImplementationMatrix::new();
        assert_eq!(matrix.completion_percentage(), 100.0);
    }

    #[test]
    fn test_task_doc_analysis_new() {
        let analysis = TaskDocAnalysis::new();
        assert_eq!(analysis.total_tasks, 0);
        assert!(!analysis.has_overclaims());
    }

    #[test]
    fn test_task_doc_analysis_add_overclaim() {
        let mut analysis = TaskDocAnalysis::new();
        analysis.add_overclaim(OverclaimedTask {
            task_id: 1,
            name: "Test".to_string(),
            current_status: "Complete".to_string(),
            should_be_status: ImplementationStatus::Placeholder,
            reason: "Only has stub implementation".to_string(),
        });

        assert!(analysis.has_overclaims());
        assert_eq!(analysis.overclaimed_tasks.len(), 1);
    }

    #[test]
    fn test_task_doc_scanner_scan_task_doc() {
        let sample_content = r#"
| Numbered task | Status | Requirements |
| ------------ | ------ | ------------ |
| 1. Test task | Complete | Do something |
| 2. Other task | Incomplete | Do something else |
"#;

        let analysis = TaskDocScanner::scan_task_doc(sample_content);
        assert_eq!(analysis.total_tasks, 2);
        assert_eq!(analysis.complete_count, 1);
        assert_eq!(analysis.incomplete_count, 1);
    }

    #[test]
    fn test_completeness_checker_has_real_implementation() {
        let implemented =
            MatrixEntry::new(1, "Test").with_status(ImplementationStatus::Implemented);
        let placeholder =
            MatrixEntry::new(2, "Test2").with_status(ImplementationStatus::Placeholder);

        assert!(CompletenessChecker::has_real_implementation(&implemented));
        assert!(!CompletenessChecker::has_real_implementation(&placeholder));
    }

    #[test]
    fn test_task_doc_scanner_with_evidence_detects_missing_source_and_tests() {
        // Use the real workspace root so repository evidence is portable across checkouts.
        let workspace = workspace_root();
        let content = r#"
| Numbered task | Status | Requirements |
| ------------ | ------ | ------------ |
| 1. FakeOverclaimedFeature | Complete | Implement the feature |
| 2. AnotherFake | Complete | Do something else |
"#;

        let analysis = TaskDocScanner::scan_with_evidence(content, workspace);
        // Both features are Complete in text but have no real source/test evidence
        assert!(
            analysis.has_overclaims(),
            "Missing evidence should be detected as overclaim"
        );
        assert_eq!(analysis.overclaimed_tasks.len(), 2);
    }

    #[test]
    fn test_task_doc_scanner_with_evidence_complete_when_evidence_exists() {
        // completion_gate has real files - use a feature that exists in the repo
        let workspace = workspace_root();
        let content = r#"
| Numbered task | Status | Requirements |
| ------------ | ------ | ------------ |
| 1. completion_gate | Complete | Full implementation with tests |
"#;

        let analysis = TaskDocScanner::scan_with_evidence(content, workspace);
        // completion_gate has real files in the repo - should NOT be overclaimed
        assert!(
            !analysis.has_overclaims(),
            "Real feature with evidence should not be overclaimed"
        );
    }

    #[test]
    fn test_task_doc_scanner_with_evidence_partial_when_source_only() {
        use super::*;
        use std::path::PathBuf;
        // Create an evidence provider that lies about source but not tests
        // We test the logic: if source_found=true but tests_found=false, should be Partial
        let mut analysis = TaskDocAnalysis::new();
        analysis.complete_count = 1;
        // Simulate a feature that has source but no tests
        analysis.add_overclaim(OverclaimedTask {
            task_id: 1,
            name: "source_only_feature".to_string(),
            current_status: "Complete".to_string(),
            should_be_status: ImplementationStatus::Partial,
            reason: "No test files found for 'source_only_feature' (source=true)".to_string(),
        });
        assert!(analysis.has_overclaims());
        assert_eq!(
            analysis.overclaimed_tasks[0].should_be_status,
            ImplementationStatus::Partial
        );
    }

    #[test]
    fn test_task_doc_scanner_incomplete_not_overclaimed() {
        let workspace = workspace_root();
        let content = r#"
| Numbered task | Status | Requirements |
| ------------ | ------ | ------------ |
| 1. NotStarted | Incomplete | Needs implementation |
| 2. InProgress | Partial | Work in progress |
"#;

        let analysis = TaskDocScanner::scan_with_evidence(content, workspace);
        // Incomplete/Partial tasks are not overclaimed (they admit incompleteness)
        assert!(!analysis.has_overclaims());
        assert_eq!(analysis.incomplete_count, 2);
    }
}
