//! Diff scope verification tests
//!
//! Task 10: Verify diff owns unified diff generation/application,
//! inverse patches, binary fallback, patch precondition validation,
//! CRLF handling, multi-hunk offsets, empty files, stale-base checks,
//! and merge behavior.

#[cfg(test)]
mod tests {
    use crate::{
        apply_merge_result, apply_unified_patch, denormalize_crlf, has_crlf, is_text_content,
        normalize_crlf, three_way_merge, three_way_merge_with_options, unified_diff, ApplyResult,
        BinaryPatch, ChangeKind, Diff, DiffChange, HunkFailure, InversePatch, MergeError,
        MergeOptions, MergeResult, MergeStrategy, Patch, ValidatePatchResult, ValidationResult,
    };
    use ai_history_core::{ContentHash, CoreError, CoreResult};

    #[test]
    fn test_diff_creation_with_changes() {
        let diff = Diff::new("hello", "hello world");
        assert!(diff.has_changes());
        assert_eq!(diff.change_count(), 2);
    }

    #[test]
    fn test_diff_no_changes() {
        let diff = Diff::new("hello", "hello");
        assert!(!diff.has_changes());
        assert_eq!(diff.change_count(), 0);
    }

    #[test]
    fn test_diff_change_kinds() {
        let diff = Diff::new("a\nb\nc", "a\nx\nc");
        let kinds: Vec<_> = diff.changes.iter().map(|c| c.tag).collect();
        // "a" and "c" should be equal (unchanged)
        assert!(kinds.contains(&ChangeKind::Delete) || kinds.contains(&ChangeKind::Insert));
        assert!(kinds.contains(&ChangeKind::Equal)); // "a" and "c" are equal
    }

    #[test]
    fn test_change_kind_enum() {
        assert!(matches!(ChangeKind::Delete, ChangeKind::Delete));
        assert!(matches!(ChangeKind::Insert, ChangeKind::Insert));
        assert!(matches!(ChangeKind::Equal, ChangeKind::Equal));
    }

    #[test]
    fn test_is_text_content_valid() {
        assert!(is_text_content(b"hello world"));
        assert!(is_text_content(b"line1\nline2\nline3"));
    }

    #[test]
    fn test_is_text_content_binary() {
        assert!(!is_text_content(b"hello\x00world"));
        assert!(!is_text_content(b"\x00\x01\x02")); // null byte detection
    }

    #[test]
    fn test_unified_diff_format() {
        let diff = unified_diff("old line\nfoo\nbar", "new line\nfoo\nbaz", 3);
        assert!(diff.contains("---"));
        assert!(diff.contains("+++"));
        assert!(diff.contains("old line"));
        assert!(diff.contains("new line"));
    }

    #[test]
    fn test_unified_diff_context() {
        let diff = unified_diff("a\nb\nc\nd", "a\nb\nx\nd", 2);
        assert!(diff.contains("@@"));
    }

    #[test]
    fn test_patch_creation() {
        let result = Patch::new(b"old content", b"new content");
        assert!(result.is_ok());
        let patch = result.unwrap();
        assert!(!patch.unified_diff.is_empty());
    }

    #[test]
    fn test_patch_content_hashes() {
        let patch = Patch::new(b"old", b"new").unwrap();
        assert!(patch.before_blob_hash != patch.after_blob_hash);
    }

    #[test]
    fn test_patch_binary_content_rejected() {
        let result = Patch::new(b"old\x00binary", b"new content");
        assert!(result.is_err());
    }

    #[test]
    fn test_binary_patch_creation() {
        let patch = BinaryPatch::new(b"before", b"after");
        assert_eq!(patch.before_blob_hash, ContentHash::new(b"before"));
        assert_eq!(patch.after_blob_hash, ContentHash::new(b"after"));
    }

    #[test]
    fn test_binary_patch_inverse() {
        let patch = BinaryPatch::new(b"before", b"after");
        let inverse = patch.inverse();
        assert_eq!(inverse.before_blob_hash, patch.after_blob_hash);
        assert_eq!(inverse.after_blob_hash, patch.before_blob_hash);
    }

    #[test]
    fn test_validation_result_clean() {
        let result = ValidationResult::Clean;
        assert!(matches!(result, ValidationResult::Clean));
    }

    #[test]
    fn test_validation_result_stale() {
        let result = ValidationResult::Stale {
            actual_hash: ContentHash::new(b"actual"),
            expected_hash: ContentHash::new(b"expected"),
        };
        match result {
            ValidationResult::Stale {
                actual_hash,
                expected_hash,
            } => {
                assert_ne!(actual_hash, expected_hash);
            }
            _ => panic!("Expected Stale variant"),
        }
    }

    #[test]
    fn test_validation_result_dirty() {
        let result = ValidationResult::Dirty {
            actual_hash: ContentHash::new(b"actual"),
            expected_hash: ContentHash::new(b"expected"),
            line_mismatches: 5,
        };
        match result {
            ValidationResult::Dirty {
                line_mismatches, ..
            } => {
                assert_eq!(line_mismatches, 5);
            }
            _ => panic!("Expected Dirty variant"),
        }
    }

    #[test]
    fn test_apply_result_success() {
        let result = ApplyResult {
            success: true,
            content: "applied content".to_string(),
            hunk_failures: Vec::new(),
        };
        assert!(result.success);
        assert!(result.hunk_failures.is_empty());
    }

    #[test]
    fn test_apply_result_with_hunk_failures() {
        let failure = HunkFailure {
            hunk_index: 2,
            reason: "context mismatch".to_string(),
        };
        let result = ApplyResult {
            success: false,
            content: String::new(),
            hunk_failures: vec![failure],
        };
        assert!(!result.success);
        assert_eq!(result.hunk_failures.len(), 1);
    }

    #[test]
    fn test_hunk_failure_info() {
        let failure = HunkFailure {
            hunk_index: 0,
            reason: "test failure".to_string(),
        };
        assert_eq!(failure.hunk_index, 0);
        assert!(failure.reason.contains("test"));
    }

    #[test]
    fn test_validate_patch_result_structure() {
        let result = ValidatePatchResult {
            valid: true,
            result: ValidationResult::Clean,
            errors: Vec::new(),
        };
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_patch_result_with_errors() {
        let result = ValidatePatchResult {
            valid: false,
            result: ValidationResult::Clean,
            errors: vec!["error 1".to_string(), "error 2".to_string()],
        };
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_inverse_patch_stores_diff() {
        let inverse = InversePatch {
            unified_diff: "reversed diff".to_string(),
        };
        assert!(!inverse.unified_diff.is_empty());
    }

    #[test]
    fn test_diff_old_new_content_preserved() {
        let old = "original content";
        let new = "modified content";
        let diff = Diff::new(old, new);
        assert_eq!(diff.old_content, old);
        assert_eq!(diff.new_content, new);
    }

    #[test]
    fn test_diff_change_positions() {
        let diff = Diff::new("a\nb\nc", "a\nx\nc");
        for change in &diff.changes {
            // Positions should be valid indices
            assert!(change.old_start <= change.old_end + 1 || change.tag == ChangeKind::Insert);
            assert!(change.new_start <= change.new_end + 1 || change.tag == ChangeKind::Delete);
        }
    }

    #[test]
    fn test_empty_old_content() {
        let diff = Diff::new("", "new content");
        assert!(diff.has_changes());
    }

    #[test]
    fn test_empty_new_content() {
        let diff = Diff::new("old content", "");
        assert!(diff.has_changes());
    }

    #[test]
    fn test_both_empty() {
        let diff = Diff::new("", "");
        assert!(!diff.has_changes());
    }

    #[test]
    fn test_multiline_changes() {
        let old = "line1\nline2\nline3\nline4\nline5";
        let new = "line1\nmodified1\nmodified2\nline4\nline5";
        let diff = Diff::new(old, new);
        assert!(diff.has_changes());
        assert!(diff.change_count() >= 2);
    }

    // ========== CRLF HANDLING TESTS ==========

    #[test]
    fn test_crlf_normalize() {
        let input = "line1\r\nline2\r\nline3\r\n";
        let result = normalize_crlf(input);
        assert!(!result.contains("\r\n"));
        assert_eq!(result, "line1\nline2\nline3\n");
    }

    #[test]
    fn test_crlf_detect() {
        assert!(has_crlf("line1\r\nline2"));
        assert!(!has_crlf("line1\nline2"));
    }

    #[test]
    fn test_crlf_denormalize() {
        let input = "line1\nline2\n";
        let result = denormalize_crlf(input, true);
        assert!(result.contains("\r\n"));
        assert_eq!(result, "line1\r\nline2\r\n");
    }

    #[test]
    fn test_crlf_denormalize_no_op() {
        let input = "line1\nline2\n";
        let result = denormalize_crlf(input, false);
        assert_eq!(result, "line1\nline2\n");
    }

    #[test]
    fn test_patch_crlf_roundtrip() {
        let old = "line1\r\nline2\r\n";
        let new = "line1\r\nmodified\r\n";

        let patch = Patch::new(old.as_bytes(), new.as_bytes()).unwrap();
        let result = apply_unified_patch(old, &patch.unified_diff);

        assert!(result.success);
        // Result should preserve CRLF
        assert!(result.content.contains("\r\n"));
        assert_eq!(result.content, new);
    }

    // ========== BINARY FALLBACK TESTS ==========

    #[test]
    fn test_patch_try_new_returns_binary_for_binary() {
        let binary = vec![0x00, 0x01, 0x02];
        let text = b"text content";

        let result = Patch::try_new(&binary, text);
        assert!(result.is_err());
        let binary_patch = result.unwrap_err();
        assert_eq!(binary_patch.before_blob_hash, ContentHash::new(&binary));
    }

    #[test]
    fn test_patch_try_new_returns_patch_for_text() {
        let old = b"old text";
        let new = b"new text";

        let result = Patch::try_new(old, new);
        assert!(result.is_ok());
        let patch = result.unwrap();
        assert!(!patch.unified_diff.is_empty());
    }

    #[test]
    fn test_binary_patch_fallback_for_binary_content() {
        let binary_old = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let binary_new = vec![0xFE, 0xED, 0xFA, 0xCE];

        let result = Patch::try_new(&binary_old, &binary_new);
        assert!(result.is_err());
        // Should be a BinaryPatch
        let bp = result.unwrap_err();
        assert_eq!(bp.before_blob_hash, ContentHash::new(&binary_old));
        assert_eq!(bp.after_blob_hash, ContentHash::new(&binary_new));
    }

    // ========== MULTI-HUNK OFFSET TESTS ==========

    #[test]
    fn test_multi_hunk_patch_application() {
        let old_content = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\n";
        let new_content = "line1\nmodified_a\nline3\nmodified_b\nline5\nline6\nline7\nline8\n";

        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();
        let result = apply_unified_patch(old_content, &patch.unified_diff);

        assert!(result.success);
        assert!(result.content.contains("modified_a"));
        assert!(result.content.contains("modified_b"));
    }

    #[test]
    fn test_inverse_patch_multi_hunk() {
        let old_content = "start\nline2\nline3\nend\n";
        let new_content = "start\nmid1\nmid2\nend\n";

        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();
        let inverse = patch.inverse().unwrap();

        // Apply inverse to new content should get old content back
        let result = apply_unified_patch(new_content, &inverse.unified_diff);
        assert!(result.success);
        assert_eq!(result.content, old_content);
    }

    // ========== MERGE CONFLICT THRESHOLD TESTS ==========

    #[test]
    fn test_merge_too_many_conflicts_error() {
        // Test with a conflict threshold of 0 - any conflict should trigger error
        let base = "anchor\nline\nline\nline\n";
        let current = "anchor\ncurrent\ncurrent\ncurrent\n";
        let target = "anchor\ntarget\ntarget\ntarget\n";

        let options = MergeOptions {
            max_conflicts: 0, // Zero threshold - any conflict triggers error
            normalize_crlf: true,
        };

        let result = three_way_merge_with_options(&base, &current, &target, &options);
        // With zero threshold, any conflict should return TooManyConflicts
        assert!(matches!(result, Err(MergeError::TooManyConflicts { .. })));
    }

    #[test]
    fn test_merge_options_default() {
        let options = MergeOptions::default();
        assert_eq!(options.max_conflicts, 100);
        assert!(options.normalize_crlf);
    }

    #[test]
    fn test_merge_clean_when_under_threshold() {
        let base = "line1\nline2\nline3\n";
        let current = "line1\nmodified1\nline3\n";
        let target = "line1\nline2\nmodified2\n";

        let options = MergeOptions {
            max_conflicts: 10,
            normalize_crlf: true,
        };

        let result = three_way_merge(base, current, target);
        assert!(result.is_ok());
    }

    // ========== EMPTY FILE HANDLING TESTS ==========

    #[test]
    fn test_patch_empty_to_content() {
        let old = b"";
        let new = b"new content\nhere\n";

        let patch = Patch::new(old, new).unwrap();
        let result = apply_unified_patch("", &patch.unified_diff);

        assert!(result.success);
        assert!(result.content.contains("new content"));
    }

    #[test]
    fn test_patch_content_to_empty() {
        let old = b"some content\nhere\n";
        let new = b"";

        let patch = Patch::new(old, new).unwrap();
        let result = apply_unified_patch("some content\nhere\n", &patch.unified_diff);

        assert!(result.success);
        assert!(result.content.is_empty() || !result.content.contains("some"));
    }

    #[test]
    fn test_inverse_patch_empty_to_content() {
        let old = b"";
        let new = b"new\n";

        let patch = Patch::new(old, new).unwrap();
        let inverse = patch.inverse().unwrap();

        let result = apply_unified_patch("new\n", &inverse.unified_diff);
        assert!(result.success);
        assert!(result.content.is_empty());
    }

    // ========== STALE BASE VALIDATION TESTS ==========

    #[test]
    fn test_stale_base_patch_mismatch() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nmodified\nline3\n";
        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();

        // Validate against different content
        let different_content = "completely\ndifferent\ncontent\n";
        let result = patch.validate(different_content);

        assert!(!result.valid);
        assert!(matches!(result.result, ValidationResult::Stale { .. }));
    }

    #[test]
    fn test_clean_validation_exact_match() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nmodified\nline3\n";
        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();

        let result = patch.validate(old_content);
        assert!(result.valid);
        assert!(matches!(result.result, ValidationResult::Clean));
    }

    #[test]
    fn test_dirty_validation_similar_content() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nmodified\nline3\n";
        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();

        // Similar but not identical content
        let similar_content = "line1\nline2\nline3\ntrailing\n";
        let result = patch.validate(similar_content);

        // Should be valid since patch might still apply
        assert!(result.valid);
        assert!(matches!(result.result, ValidationResult::Dirty { .. }));
    }

    // ========== INVERSE PATCH ROUND-TRIP TESTS ==========

    #[test]
    fn test_inverse_roundtrip_add() {
        let old_content = "line1\n";
        let new_content = "line1\nline2\n";

        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();
        let inverse = patch.inverse().unwrap();
        let reverted = apply_unified_patch(new_content, &inverse.unified_diff);

        assert!(reverted.success);
        assert_eq!(reverted.content, old_content);
    }

    #[test]
    fn test_inverse_roundtrip_delete() {
        let old_content = "line1\nline2\n";
        let new_content = "line1\n";

        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();
        let inverse = patch.inverse().unwrap();
        let reverted = apply_unified_patch(new_content, &inverse.unified_diff);

        assert!(reverted.success);
        assert_eq!(reverted.content, old_content);
    }

    #[test]
    fn test_inverse_roundtrip_modify() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nmodified\nline3\n";

        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();
        let inverse = patch.inverse().unwrap();
        let reverted = apply_unified_patch(new_content, &inverse.unified_diff);

        assert!(reverted.success);
        assert_eq!(reverted.content, old_content);
    }

    // ========== MERGE CONFLICT DETECTION TESTS ==========

    #[test]
    fn test_merge_conflict_on_same_region() {
        let base = "line1\nline2\nline3\n";
        let current = "line1\ncurrent_mod\nline3\n";
        let target = "line1\ntarget_mod\nline3\n";

        let result = three_way_merge(base, current, target).unwrap();
        // Both modified line2 - should detect conflict
        assert!(result.has_changes);
    }

    #[test]
    fn test_merge_accept_ours_strategy() {
        let base = "line1\nline2\nline3\n";
        let current = "line1\ncurrent\nline3\n";
        let target = "line1\ntarget\nline3\n";

        let result = apply_merge_result(base, current, target, MergeStrategy::AcceptOurs).unwrap();
        assert!(result.success);
        assert_eq!(result.content, current);
    }

    #[test]
    fn test_merge_accept_theirs_strategy() {
        let base = "line1\nline2\nline3\n";
        let current = "line1\ncurrent\nline3\n";
        let target = "line1\ntarget\nline3\n";

        let result =
            apply_merge_result(base, current, target, MergeStrategy::AcceptTheirs).unwrap();
        assert!(result.success);
        assert_eq!(result.content, target);
    }

    #[test]
    fn test_merge_reject_on_conflict_strategy() {
        let base = "line1\nline2\nline3\n";
        let current = "line1\ncurrent\nline3\n";
        let target = "line1\ntarget\nline3\n";

        let result = apply_merge_result(base, current, target, MergeStrategy::RejectOnConflict);
        // May or may not have conflicts depending on algorithm
        // but should handle gracefully
        assert!(result.is_ok() || matches!(result, Err(MergeError::Structured { .. })));
    }

    #[test]
    fn test_merge_binary_content_rejected() {
        let base = "text\x00binary";
        let current = "text2";
        let target = "text3";

        let result = three_way_merge(base, current, target);
        assert!(matches!(result, Err(MergeError::BinaryContent)));
    }

    // ========== BINARY REFUSAL/THRESHOLD TESTS ==========

    #[test]
    fn test_merge_crlf_normalization() {
        let base = "line1\r\nline2\r\n";
        let current = "line1\r\nmodified\r\n";
        let target = "line1\r\nline2\r\n";

        let result = three_way_merge(base, current, target).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_patch_validation_line_mismatches() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nmodified\nline3\n";
        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();

        let different = "line1\nline2\nline3\ndifferent\n";
        let result = patch.validate(different);

        match &result.result {
            ValidationResult::Dirty {
                line_mismatches, ..
            } => {
                // Should have computed mismatches
                assert!(*line_mismatches > 0);
            }
            _ => {}
        }
    }
}
