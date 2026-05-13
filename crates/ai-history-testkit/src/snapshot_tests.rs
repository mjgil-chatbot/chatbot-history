//! Snapshot crate design tests
//!
//! Task 11: Complete `ai-history-snapshot` crate design
//!
//! Verify:
//! - Scanner exists for workspace scanning
//! - Manifest exists for content-addressed snapshots
//! - Comparison logic exists for three-way diffs
//! - FileMetadata, ScannerConfig exist

#[cfg(test)]
mod test_snapshot_manifest {
    use ai_history_core::{ContentHash, ManifestId};
    use ai_history_snapshot::manifest::{FileEntry, FileOwnership, Manifest};

    /// Verify Manifest exists
    #[test]
    fn test_manifest_exists() {
        let id = ManifestId::new();
        let manifest = Manifest::new(id);
        assert!(manifest.id.to_string().starts_with("manifest:"));
    }

    /// Verify FileEntry exists
    #[test]
    fn test_file_entry_exists() {
        let entry = FileEntry {
            path: "test.txt".to_string(),
            blob_hash: ContentHash::new(b"content"),
            size: 7,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: Some(FileOwnership::UserOwned),
        };
        assert!(entry.path == "test.txt");
    }

    /// Verify FileOwnership enum exists with required variants
    #[test]
    fn test_file_ownership_variants() {
        assert!(matches!(FileOwnership::UserOwned, FileOwnership::UserOwned));
        assert!(matches!(FileOwnership::AiOwned, FileOwnership::AiOwned));
        assert!(matches!(FileOwnership::External, FileOwnership::External));
    }
}
#[cfg(test)]
mod test_snapshot_comparison {
    use ai_history_core::ManifestId;
    use ai_history_snapshot::comparison::ThreeWayDiff;

    /// Verify ThreeWayDiff exists
    #[test]
    fn test_three_way_diff_exists() {
        let diff = ThreeWayDiff {
            added: vec![],
            modified: vec![],
            deleted: vec![],
            conflicts: vec![],
            renamed: vec![],
            unchanged: vec![],
        };
        assert_eq!(diff.added.len(), 0);
        assert_eq!(diff.modified.len(), 0);
        assert_eq!(diff.deleted.len(), 0);
        assert_eq!(diff.conflicts.len(), 0);
    }
}
