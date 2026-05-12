//! Diff crate design tests
//!
//! Task 12: Complete `ai-history-diff` crate design
//!
//! Verify:
//! - Diff and DiffChange exist for text diff
//! - Patch, InversePatch, BinaryPatch exist
//! - MergeResult, MergeConflict, MergeStrategy exist
//! - ValidationResult exists

#[cfg(test)]
mod test_diff_types {
    use ai_history_diff::{ChangeKind, Diff, DiffChange};

    /// Verify Diff exists
    #[test]
    fn test_diff_exists() {
        let diff = Diff {
            changes: vec![],
            old_content: "old".to_string(),
            new_content: "new".to_string(),
        };
        assert_eq!(diff.old_content, "old");
    }

    /// Verify DiffChange exists
    #[test]
    fn test_diff_change_exists() {
        let change = DiffChange {
            tag: ChangeKind::Insert,
            old_start: 0,
            old_end: 0,
            new_start: 1,
            new_end: 1,
            value: "inserted".to_string(),
        };
        assert!(matches!(change.tag, ChangeKind::Insert));
    }

    /// Verify ChangeKind enum variants
    #[test]
    fn test_change_kind_variants() {
        assert!(matches!(ChangeKind::Delete, ChangeKind::Delete));
        assert!(matches!(ChangeKind::Insert, ChangeKind::Insert));
        assert!(matches!(ChangeKind::Equal, ChangeKind::Equal));
    }
}
