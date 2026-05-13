#[cfg(test)]
mod test_ui_accessibility_contract {
    use ai_history_ui_model::{
        AccessibilityContract, AccessibilityLabels, AnnouncementPriority, ConflictExplanation,
        DestructiveActionType, DestructiveConfirmState, KeyboardNavigation,
        ScreenReaderAnnouncement, UxCopy,
    };

    #[test]
    fn test_accessibility_labels_default() {
        let labels = AccessibilityLabels::default();
        assert_eq!(labels.confirm_restore, "Confirm restore");
        assert_eq!(labels.cancel, "Cancel");
    }

    #[test]
    fn test_keyboard_navigation_default() {
        let nav = KeyboardNavigation::default();
        assert!(nav.enabled);
        assert!(nav.focused_element.is_none());
    }

    #[test]
    fn test_destructive_confirm_restore() {
        let state = DestructiveConfirmState::for_restore(5);
        assert!(state.confirmation_required);
        assert!(!state.user_confirmed);
        assert!(state.warning_message.contains("5 files"));
    }

    #[test]
    fn test_conflict_explanation_modified() {
        let exp = ConflictExplanation::modified("src/main.rs");
        assert_eq!(exp.conflict_type, "Modified");
        assert!(exp.resolutions.len() >= 2);
    }

    #[test]
    fn test_ux_copy_default() {
        let ux = UxCopy::default();
        assert_eq!(ux.confirm_button, "Confirm");
        assert_eq!(ux.warning_title, "Warning");
    }

    #[test]
    fn test_accessibility_contract_composition() {
        let contract = AccessibilityContract::default();
        assert_eq!(contract.labels.confirm_restore, "Confirm restore");
        assert!(contract.keyboard.enabled);
    }

    #[test]
    fn test_screen_reader_announcement() {
        let msg = ScreenReaderAnnouncement {
            message: "Restore complete".to_string(),
            priority: AnnouncementPriority::Assertive,
        };
        assert_eq!(msg.priority, AnnouncementPriority::Assertive);
    }
}

#[cfg(test)]
mod test_acceptance_scenarios {
    use crate::acceptance::{
        run_all_scenarios, run_scenario_one, run_scenario_three, run_scenario_two,
    };

    #[test]
    fn test_scenario_one_format() {
        let result = run_scenario_one().expect("Scenario 1 should succeed");
        assert!(result.contains("Session:"));
        assert!(result.contains("Branch:"));
        assert!(result.contains("Checkpoint:"));
    }

    #[test]
    fn test_scenario_two_format() {
        let result = run_scenario_two().expect("Scenario 2 should succeed");
        assert!(result.contains("Dirty:"));
        assert!(result.contains("Conflicts:"));
    }

    #[test]
    fn test_scenario_three_format() {
        let result = run_scenario_three().expect("Scenario 3 should succeed");
        assert!(result.contains("Main:"));
        assert!(result.contains("Feature:"));
    }

    #[test]
    fn test_all_scenarios_run() {
        let scenarios = run_all_scenarios();
        assert_eq!(scenarios.len(), 4, "Should have 4 acceptance scenarios");
        for (name, result) in scenarios {
            assert!(
                result.is_ok(),
                "Scenario '{}' should succeed: {:?}",
                name,
                result
            );
            assert!(
                !result.unwrap().is_empty(),
                "Scenario '{}' returned empty",
                name
            );
        }
    }
}

#[cfg(test)]
mod test_implementation_audit {
    use ai_history_core::ImplementationStatus;

    #[test]
    fn test_implementation_status_variants() {
        assert!(matches!(
            ImplementationStatus::Implemented,
            ImplementationStatus::Implemented
        ));
        assert!(matches!(
            ImplementationStatus::Partial,
            ImplementationStatus::Partial
        ));
        assert!(matches!(
            ImplementationStatus::Missing,
            ImplementationStatus::Missing
        ));
        assert!(matches!(
            ImplementationStatus::Placeholder,
            ImplementationStatus::Placeholder
        ));
        assert!(matches!(
            ImplementationStatus::Deferred,
            ImplementationStatus::Deferred
        ));
    }

    #[test]
    fn test_implemented_is_sufficient() {
        assert!(ImplementationStatus::Implemented.is_sufficient());
    }

    #[test]
    fn test_non_implemented_not_sufficient() {
        assert!(!ImplementationStatus::Partial.is_sufficient());
        assert!(!ImplementationStatus::Missing.is_sufficient());
        assert!(!ImplementationStatus::Placeholder.is_sufficient());
        assert!(!ImplementationStatus::Deferred.is_sufficient());
    }
}

#[cfg(test)]
mod test_design_review {
    use ai_history_core::design_review::{
        DesignReview, FinalReviewResult, ReviewCategory, ReviewItem, ReviewStatus,
    };

    #[test]
    fn test_design_review_new() {
        let review = DesignReview::new();
        assert!(review.items.is_empty());
    }

    #[test]
    fn test_design_review_with_categories() {
        let review = DesignReview::with_all_categories();
        assert!(!review.items.is_empty());
        for &category in ReviewCategory::all() {
            assert!(!review.items_by_category(category).is_empty());
        }
    }

    #[test]
    fn test_review_item_creation() {
        let item = ReviewItem::new(ReviewCategory::Correctness, "Test question?");
        assert_eq!(item.category, ReviewCategory::Correctness);
        assert_eq!(item.status, ReviewStatus::NeedsWork);
    }

    #[test]
    fn test_review_item_status_marking() {
        let item = ReviewItem::new(ReviewCategory::Safety, "Safe?").mark_pass("Verified");
        assert_eq!(item.status, ReviewStatus::Pass);

        let item = ReviewItem::new(ReviewCategory::Safety, "Safe?").mark_fail("Issue found");
        assert_eq!(item.status, ReviewStatus::Fail);
    }

    #[test]
    fn test_final_review_result() {
        let mut review = DesignReview::new();
        review.add_item(ReviewItem::new(ReviewCategory::Correctness, "Test?").mark_pass("OK"));
        let result = FinalReviewResult::from_review(review);
        assert!(result.all_checks_passed);
        assert!(result.all_resolved());
    }

    #[test]
    fn test_review_categories_count() {
        assert_eq!(ReviewCategory::all().len(), 10);
    }

    #[test]
    fn test_review_category_display_names() {
        assert_eq!(ReviewCategory::Correctness.display_name(), "Correctness");
        assert_eq!(ReviewCategory::Safety.display_name(), "Safety");
        assert_eq!(ReviewCategory::Performance.display_name(), "Performance");
    }
}

#[cfg(test)]
mod test_docs_gate {
    use ai_history_core::docs_gate::{DocsGateResult, EvidenceType};

    #[test]
    fn test_docs_gate_result_exists() {
        let result = DocsGateResult::new();
        assert_eq!(result.total_rows_scanned, 0);
        assert!(result.passed);
    }

    #[test]
    fn test_evidence_type_variants() {
        assert!(matches!(
            EvidenceType::Implementation,
            EvidenceType::Implementation
        ));
        assert!(matches!(EvidenceType::Tests, EvidenceType::Tests));
        assert!(matches!(
            EvidenceType::Documentation,
            EvidenceType::Documentation
        ));
    }
}

#[cfg(test)]
mod test_completion_gate {
    use ai_history_core::completion_gate::{
        CompletionGateCheck, CompletionRequirements, CompletionStatus, DocumentationChecker,
        EvidenceReport, FilesystemEvidenceProvider, TestCoverageChecker,
    };
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
    fn test_completion_requirements_satisfied() {
        let req = CompletionRequirements {
            has_implementation: true,
            has_tests: true,
            has_documentation: true,
            has_edge_case_coverage: true,
        };
        assert!(req.all_satisfied());
    }

    #[test]
    fn test_completion_requirements_missing() {
        let req = CompletionRequirements {
            has_implementation: true,
            has_tests: false,
            has_documentation: true,
            has_edge_case_coverage: false,
        };
        let missing = req.missing_requirements();
        assert!(missing.contains(&"tests"));
        assert!(missing.contains(&"edge case coverage"));
    }

    #[test]
    fn test_completion_gate_check_pass() {
        let check = CompletionGateCheck::pass("API completeness");
        assert!(check.passed);
    }

    #[test]
    fn test_completion_gate_check_fail() {
        let check = CompletionGateCheck::fail("API completeness", "Missing return type");
        assert!(!check.passed);
    }

    #[test]
    fn test_completion_status_variants() {
        assert!(matches!(
            CompletionStatus::NotStarted,
            CompletionStatus::NotStarted
        ));
        assert!(matches!(
            CompletionStatus::InProgress,
            CompletionStatus::InProgress
        ));
        assert!(matches!(
            CompletionStatus::Complete,
            CompletionStatus::Complete
        ));
    }

    #[test]
    fn test_documentation_checker_fake_feature_no_docs() {
        let checker = DocumentationChecker::with_filesystem_evidence(workspace_root());
        assert!(!checker.has_documentation("nonexistent_fake_feature_xyz123"));
    }

    #[test]
    fn test_test_coverage_checker_fake_feature_no_tests() {
        let checker = TestCoverageChecker::with_filesystem_evidence(workspace_root());
        assert!(!checker.has_tests("fake_nonexistent_feature_abc"));
        assert!(!checker.has_edge_case_coverage("fake_nonexistent_feature_abc"));
    }

    #[test]
    fn test_keyword_only_doc_does_not_pass() {
        let provider = FilesystemEvidenceProvider::new(workspace_root());
        let report = provider.evidence_report("test_test_test");
        assert!(
            !report.docs_found,
            "Keyword-only feature name should not pass as documented"
        );
    }

    #[test]
    fn test_fake_feature_evidence_is_empty() {
        let provider = FilesystemEvidenceProvider::new(workspace_root());
        let report = provider.evidence_report("completely_fake_feature_xyz789");
        assert!(
            !report.source_found,
            "Fake feature should not have source evidence"
        );
        assert!(
            !report.tests_found,
            "Fake feature should not have test evidence"
        );
        assert!(
            !report.docs_found,
            "Fake feature should not have doc evidence"
        );
        assert!(
            !report.edge_cases_found,
            "Fake feature should not have edge case evidence"
        );
    }

    #[test]
    fn test_evidence_report_has_all_fields() {
        let report = EvidenceReport::default();
        assert!(report.source_files.is_empty());
        assert!(report.test_files.is_empty());
        assert!(report.doc_files.is_empty());
        assert!(report.edge_case_markers.is_empty());
        assert!(!report.source_found);
        assert!(!report.tests_found);
        assert!(!report.docs_found);
        assert!(!report.edge_cases_found);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_documentation_checker_default_not_hardcoded_true() {
        let checker = DocumentationChecker::default();
        let result = checker.has_documentation("feature_that_does_not_exist_12345");
        assert!(!result, "DocumentationChecker should not hardcode true");
    }

    #[test]
    fn test_test_coverage_checker_default_not_hardcoded_true() {
        let checker = TestCoverageChecker::default();
        let has_tests = checker.has_tests("fake_feature_no_tests_xyz");
        let has_edge = checker.has_edge_case_coverage("fake_feature_no_edge_xyz");
        assert!(
            !has_tests,
            "TestCoverageChecker should not hardcode has_tests true"
        );
        assert!(
            !has_edge,
            "TestCoverageChecker should not hardcode has_edge_case_coverage true"
        );
    }
}
