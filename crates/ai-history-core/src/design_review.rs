//! Final design review checklist
//!
//! Reviews:
//! - Correctness
//! - Safety
//! - Rollback guarantees
//! - Branch preservation
//! - Conflict behavior
//! - Rust crate boundaries
//! - Test coverage
//! - Docs coverage
//! - Security
//! - Performance

use serde::{Deserialize, Serialize};

/// Review category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReviewCategory {
    Correctness,
    Safety,
    RollbackGuarantees,
    BranchPreservation,
    ConflictBehavior,
    CrateBoundaries,
    TestCoverage,
    DocsCoverage,
    Security,
    Performance,
}

impl ReviewCategory {
    /// Get all review categories
    pub fn all() -> &'static [Self] {
        &[
            Self::Correctness,
            Self::Safety,
            Self::RollbackGuarantees,
            Self::BranchPreservation,
            Self::ConflictBehavior,
            Self::CrateBoundaries,
            Self::TestCoverage,
            Self::DocsCoverage,
            Self::Security,
            Self::Performance,
        ]
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Correctness => "Correctness",
            Self::Safety => "Safety",
            Self::RollbackGuarantees => "Rollback Guarantees",
            Self::BranchPreservation => "Branch Preservation",
            Self::ConflictBehavior => "Conflict Behavior",
            Self::CrateBoundaries => "Crate Boundaries",
            Self::TestCoverage => "Test Coverage",
            Self::DocsCoverage => "Documentation Coverage",
            Self::Security => "Security",
            Self::Performance => "Performance",
        }
    }

    /// Get review questions for this category
    pub fn questions(&self) -> &'static [&'static str] {
        match self {
            Self::Correctness => &[
                "Are all state transitions well-defined?",
                "Is event sourcing replay deterministic?",
                "Are typed IDs used correctly throughout?",
                "Are error cases handled properly?",
                "Is serialization/deserialization symmetric?",
            ],
            Self::Safety => &[
                "Do destructive actions require confirmation?",
                "Are safety checkpoints created before restore?",
                "Is workspace integrity verified?",
                "Are protected entities never deleted?",
                "Is there protection against data loss?",
            ],
            Self::RollbackGuarantees => &[
                "Can any state be restored to any checkpoint?",
                "Is rollback atomic (all-or-nothing)?",
                "Is undo/redo history preserved?",
                "Are partial restores prevented?",
                "Is rollback conflict-free when no conflicts exist?",
            ],
            Self::BranchPreservation => &[
                "Are all branches preserved until explicitly deleted?",
                "Does branch fork preserve full history?",
                "Can branches be merged?",
                "Is branch switching safe?",
                "Are branch names unique per session?",
            ],
            Self::ConflictBehavior => &[
                "Are workspace conflicts detected before restore?",
                "Is conflict resolution UI clear?",
                "Can users understand their options?",
                "Are conflicts resolved consistently?",
                "Is conflict state properly tracked?",
            ],
            Self::CrateBoundaries => &[
                "Are public APIs minimal and focused?",
                "Is there no cyclic dependencies?",
                "Are internal types private?",
                "Do crates have single responsibility?",
                "Are feature flags used appropriately?",
            ],
            Self::TestCoverage => &[
                "Are all public APIs tested?",
                "Is event sourcing tested end-to-end?",
                "Are error paths tested?",
                "Is concurrency tested?",
                "Is the golden path tested?",
            ],
            Self::DocsCoverage => &[
                "Is the design doc complete?",
                "Are all public APIs documented?",
                "Is error handling documented?",
                "Are examples provided?",
                "Is the README up-to-date?",
            ],
            Self::Security => &[
                "Are checksums verified on read?",
                "Is sensitive data excluded from logs?",
                "Are imports verified before acceptance?",
                "Is SQLite injection prevented?",
                "Are file paths validated?",
            ],
            Self::Performance => &[
                "Is blob content-addressed for deduplication?",
                "Is manifest loading lazy?",
                "Are queries indexed appropriately?",
                "Is blob streaming used for large files?",
                "Is GC incremental?",
            ],
        }
    }
}

/// Status of a review item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewStatus {
    Pass,
    Fail,
    NeedsWork,
    NotApplicable,
}

impl Default for ReviewStatus {
    fn default() -> Self {
        Self::NeedsWork
    }
}

/// A single review item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewItem {
    pub category: ReviewCategory,
    pub question: String,
    pub status: ReviewStatus,
    pub notes: Option<String>,
}

impl ReviewItem {
    /// Create a new review item
    pub fn new(category: ReviewCategory, question: &str) -> Self {
        Self {
            category,
            question: question.to_string(),
            status: ReviewStatus::default(),
            notes: None,
        }
    }

    /// Mark as passing
    pub fn mark_pass(mut self, notes: &str) -> Self {
        self.status = ReviewStatus::Pass;
        self.notes = Some(notes.to_string());
        self
    }

    /// Mark as failing
    pub fn mark_fail(mut self, notes: &str) -> Self {
        self.status = ReviewStatus::Fail;
        self.notes = Some(notes.to_string());
        self
    }

    /// Mark as needing work
    pub fn mark_needs_work(mut self, notes: &str) -> Self {
        self.status = ReviewStatus::NeedsWork;
        self.notes = Some(notes.to_string());
        self
    }

    /// Mark as not applicable
    pub fn mark_na(mut self) -> Self {
        self.status = ReviewStatus::NotApplicable;
        self.notes = None;
        self
    }
}

/// A complete design review
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DesignReview {
    pub items: Vec<ReviewItem>,
    pub reviewer: Option<String>,
    pub review_date: Option<String>,
}

impl DesignReview {
    /// Create a new empty design review
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new review with all categories populated
    pub fn with_all_categories() -> Self {
        let mut review = Self::new();
        for &category in ReviewCategory::all() {
            for &question in category.questions() {
                review.items.push(ReviewItem::new(category, question));
            }
        }
        review
    }

    /// Add a review item
    pub fn add_item(&mut self, item: ReviewItem) {
        self.items.push(item);
    }

    /// Set reviewer info
    pub fn set_reviewer(mut self, name: &str, date: &str) -> Self {
        self.reviewer = Some(name.to_string());
        self.review_date = Some(date.to_string());
        self
    }

    /// Get items by category
    pub fn items_by_category(&self, category: ReviewCategory) -> Vec<&ReviewItem> {
        self.items
            .iter()
            .filter(|i| i.category == category)
            .collect()
    }

    /// Get pass count
    pub fn pass_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == ReviewStatus::Pass)
            .count()
    }

    /// Get fail count
    pub fn fail_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == ReviewStatus::Fail)
            .count()
    }

    /// Get needs work count
    pub fn needs_work_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == ReviewStatus::NeedsWork)
            .count()
    }

    /// Get not applicable count
    pub fn na_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == ReviewStatus::NotApplicable)
            .count()
    }

    /// Check if review passed (no fails)
    pub fn passed(&self) -> bool {
        self.fail_count() == 0
    }

    /// Check if review is complete
    pub fn is_complete(&self) -> bool {
        self.items
            .iter()
            .all(|i| i.status != ReviewStatus::NeedsWork)
    }

    /// Get summary by category
    pub fn category_summary(&self) -> HashMap<ReviewCategory, (usize, usize)> {
        use std::collections::HashMap;
        let mut summary: HashMap<ReviewCategory, (usize, usize)> = HashMap::new();

        for &category in ReviewCategory::all() {
            let items = self.items_by_category(category);
            let total = items.len();
            let passing = items
                .iter()
                .filter(|i| i.status == ReviewStatus::Pass)
                .count();
            summary.insert(category, (passing, total));
        }

        summary
    }
}

/// Final review result
#[derive(Debug, Clone)]
pub struct FinalReviewResult {
    pub review: DesignReview,
    pub all_checks_passed: bool,
    pub known_risks: Vec<String>,
    pub unresolved_decisions: Vec<String>,
}

impl FinalReviewResult {
    /// Create from a completed review
    pub fn from_review(review: DesignReview) -> Self {
        let known_risks = review
            .items
            .iter()
            .filter(|i| i.status == ReviewStatus::Fail)
            .map(|i| format!("{}: {}", i.category.display_name(), i.question))
            .collect();

        let unresolved_decisions = review
            .items
            .iter()
            .filter(|i| i.status == ReviewStatus::NeedsWork)
            .map(|i| format!("{}: {}", i.category.display_name(), i.question))
            .collect();

        Self {
            review: review.clone(),
            all_checks_passed: review.passed(),
            known_risks,
            unresolved_decisions,
        }
    }

    /// Check if all is resolved
    pub fn all_resolved(&self) -> bool {
        self.known_risks.is_empty() && self.unresolved_decisions.is_empty()
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_category_all_count() {
        assert_eq!(ReviewCategory::all().len(), 10);
    }

    #[test]
    fn test_review_category_display_name() {
        assert_eq!(ReviewCategory::Correctness.display_name(), "Correctness");
        assert_eq!(ReviewCategory::Safety.display_name(), "Safety");
        assert_eq!(ReviewCategory::Performance.display_name(), "Performance");
    }

    #[test]
    fn test_review_category_questions() {
        let questions = ReviewCategory::Correctness.questions();
        assert!(!questions.is_empty());
        assert!(questions.iter().all(|q| !q.is_empty()));
    }

    #[test]
    fn test_review_item_new() {
        let item = ReviewItem::new(ReviewCategory::Correctness, "Is event sourcing correct?");
        assert_eq!(item.category, ReviewCategory::Correctness);
        assert_eq!(item.status, ReviewStatus::NeedsWork);
        assert!(item.notes.is_none());
    }

    #[test]
    fn test_review_item_mark_pass() {
        let item =
            ReviewItem::new(ReviewCategory::Correctness, "Test?").mark_pass("Verified with tests");
        assert_eq!(item.status, ReviewStatus::Pass);
        assert!(item.notes.is_some());
    }

    #[test]
    fn test_review_item_mark_fail() {
        let item =
            ReviewItem::new(ReviewCategory::Correctness, "Test?").mark_fail("Needs implementation");
        assert_eq!(item.status, ReviewStatus::Fail);
    }

    #[test]
    fn test_review_item_mark_na() {
        let item = ReviewItem::new(ReviewCategory::Correctness, "Test?").mark_na();
        assert_eq!(item.status, ReviewStatus::NotApplicable);
        assert!(item.notes.is_none());
    }

    #[test]
    fn test_design_review_new() {
        let review = DesignReview::new();
        assert!(review.items.is_empty());
        assert!(review.pass_count() == 0);
        assert!(review.fail_count() == 0);
    }

    #[test]
    fn test_design_review_with_all_categories() {
        let review = DesignReview::with_all_categories();
        assert!(!review.items.is_empty());

        // Should have items from all 10 categories
        for &category in ReviewCategory::all() {
            assert!(!review.items_by_category(category).is_empty());
        }
    }

    #[test]
    fn test_design_review_pass_count() {
        let review = DesignReview::with_all_categories();
        // Initially all are NeedsWork, so pass count should be 0
        assert_eq!(review.pass_count(), 0);
    }

    #[test]
    fn test_design_review_passed() {
        let mut review = DesignReview::new();
        review.add_item(ReviewItem::new(ReviewCategory::Correctness, "Test?").mark_pass("OK"));
        assert!(review.passed());
    }

    #[test]
    fn test_design_review_not_passed_with_fail() {
        let mut review = DesignReview::new();
        review.add_item(ReviewItem::new(ReviewCategory::Correctness, "Test?").mark_fail("fail"));
        assert!(!review.passed());
    }

    #[test]
    fn test_design_review_is_complete() {
        let mut review = DesignReview::new();
        review.add_item(ReviewItem::new(ReviewCategory::Correctness, "Test?").mark_pass("OK"));
        assert!(review.is_complete());
    }

    #[test]
    fn test_design_review_not_complete() {
        let mut review = DesignReview::new();
        review.add_item(
            ReviewItem::new(ReviewCategory::Correctness, "Test?").mark_needs_work("TODO"),
        );
        assert!(!review.is_complete());
    }

    #[test]
    fn test_design_review_category_summary() {
        let review = DesignReview::with_all_categories();
        let summary = review.category_summary();
        assert_eq!(summary.len(), 10);

        for &category in ReviewCategory::all() {
            let (passing, total) = summary[&category];
            assert_eq!(passing, 0); // No items marked as pass initially
            assert!(total > 0);
        }
    }

    #[test]
    fn test_design_review_set_reviewer() {
        let review = DesignReview::new().set_reviewer("reviewer-1", "2024-01-01");
        assert_eq!(review.reviewer, Some("reviewer-1".to_string()));
        assert_eq!(review.review_date, Some("2024-01-01".to_string()));
    }

    #[test]
    fn test_final_review_result_from_review() {
        let mut review = DesignReview::new();
        review.add_item(ReviewItem::new(ReviewCategory::Correctness, "Test?").mark_pass("OK"));

        let result = FinalReviewResult::from_review(review);
        assert!(result.all_checks_passed);
        assert!(result.known_risks.is_empty());
    }

    #[test]
    fn test_final_review_result_with_risks() {
        let mut review = DesignReview::new();
        review.add_item(ReviewItem::new(ReviewCategory::Correctness, "Test?").mark_fail("Risk"));

        let result = FinalReviewResult::from_review(review);
        assert!(!result.all_checks_passed);
        assert!(!result.known_risks.is_empty());
    }

    #[test]
    fn test_final_review_result_all_resolved() {
        let mut review = DesignReview::new();
        review.add_item(ReviewItem::new(ReviewCategory::Correctness, "Test?").mark_pass("OK"));

        let result = FinalReviewResult::from_review(review);
        assert!(result.all_resolved());
    }

    #[test]
    fn test_review_status_default() {
        assert_eq!(ReviewStatus::default(), ReviewStatus::NeedsWork);
    }

    #[test]
    fn test_review_status_serialization() {
        let status = ReviewStatus::Pass;
        let json = serde_json::to_string(&status).unwrap();
        let parsed: ReviewStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ReviewStatus::Pass);
    }

    #[test]
    fn test_design_review_serialization() {
        let review = DesignReview::new().set_reviewer("test", "2024-01-01");

        let json = serde_json::to_string(&review).unwrap();
        let parsed: DesignReview = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.items.len(), 0);
        assert_eq!(parsed.reviewer, Some("test".to_string()));
    }
}
