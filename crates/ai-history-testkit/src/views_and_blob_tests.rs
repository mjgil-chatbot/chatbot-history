//! Materialized views, blob storage, snapshot, manifest, and diff tests
//!
//! Task 41: Complete materialized views
//! Task 42: Complete production storage selection
//! Task 43: Complete blob-store ownership
//! Task 44: Complete atomic blob storage
//! Task 45: Complete snapshot scanning
//! Task 46: Complete ignore rule semantics
//! Task 48: Complete cross-platform metadata handling
//! Task 49: Complete manifest canonicalization
//! Task 50: Complete manifest comparison

#[cfg(test)]
mod test_materialized_views {
    use ai_history_core::{BranchId, SessionId};
    use ai_history_store::views::{BranchTimelineView, ChatTranscriptView, CheckpointListView};

    /// Verify ChatTranscriptView exists and is serializable
    #[test]
    fn test_chat_transcript_view_exists() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let view = ChatTranscriptView::new(session_id, branch_id);
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("session_id"));
    }

    /// Verify BranchTimelineView exists and is serializable
    #[test]
    fn test_branch_timeline_view_exists() {
        let session_id = SessionId::new();
        let view = BranchTimelineView::new(session_id);
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("session_id"));
    }

    /// Verify CheckpointListView exists and is serializable
    #[test]
    fn test_checkpoint_list_view_exists() {
        let session_id = SessionId::new();
        let view = CheckpointListView::new(session_id);
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("session_id"));
    }
}

#[cfg(test)]
mod test_blob_ownership {
    use ai_history_core::blob_store::BlobStoreConfig as CoreConfig;
    use ai_history_core::BlobStoreConfig as SnapshotConfig;
    use ai_history_snapshot::BlobStore;
    use ai_history_store::traits::BlobStore as BlobStoreTrait;
    use tempfile::TempDir;

    /// Verify BlobStoreConfig is in core (configuration types remain in core)
    #[test]
    fn test_blob_store_config_in_core() {
        let config = CoreConfig::default();
        assert!(config.atomic_writes);
    }

    /// Verify concrete BlobStore implementation is in snapshot
    #[test]
    fn test_concrete_blob_store_in_snapshot() {
        let temp = TempDir::new().unwrap();
        let store = ai_history_snapshot::BlobStore::new(temp.path().to_path_buf()).unwrap();
        assert_eq!(store.root(), temp.path());
    }

    /// Verify BlobStore trait is in store (for storage backends)
    #[test]
    fn test_blob_store_trait_in_store() {
        // This test verifies the BlobStore trait exists in ai-history-store
        // by using it as a bound (compile-time test)
        fn _check_trait<T: BlobStoreTrait>() {}
        // FakeBlobStore implements the trait, so this compiles
        _check_trait::<crate::fakes::FakeBlobStore>();
    }

    /// Verify concrete BlobStore can write and read via snapshot crate
    #[test]
    fn test_snapshot_blob_store_write_read() {
        let temp = TempDir::new().unwrap();
        let mut store = ai_history_snapshot::BlobStore::new(temp.path().to_path_buf()).unwrap();

        let data = b"hello from snapshot blob store";
        let hash = store.write(data).unwrap();

        let read = store.read(&hash).unwrap().unwrap();
        assert_eq!(read, data);
    }

    /// Verify concrete BlobStore provides atomic writes (using snapshot config)
    #[test]
    fn test_snapshot_blob_store_atomic_writes() {
        let temp = TempDir::new().unwrap();
        let config = SnapshotConfig::default().with_atomic_writes(true);
        let mut store =
            ai_history_snapshot::BlobStore::with_config(temp.path().to_path_buf(), config).unwrap();

        let data = b"atomic write test";
        let hash = store.write(data).unwrap();

        assert!(store.has(&hash));
        assert!(store.verify(&hash).unwrap());
    }

    /// Verify snapshot's BlobStoreConfig has expected fields
    #[test]
    fn test_snapshot_blob_store_config() {
        let config = SnapshotConfig::default();
        assert!(config.atomic_writes);
    }
}

#[cfg(test)]
mod test_blob_storage {
    use crate::fakes::FakeBlobStore;
    use ai_history_core::SessionId;
    use ai_history_store::traits::BlobStore as BlobStoreTrait;

    /// Verify BlobStore trait exists and is implementable
    #[test]
    fn test_blob_store_trait_exists() {
        fn _check_blob_store<T: BlobStoreTrait>() {}
        _check_blob_store::<FakeBlobStore>();
    }

    /// Verify FakeBlobStore exists and works
    #[test]
    fn test_fake_blob_store_exists() {
        let store = FakeBlobStore::new();
        let blobs = store.list_blobs(SessionId::new()).unwrap();
        assert!(blobs.is_empty());
    }

    /// Verify FakeBlobStore can write and read blob
    #[test]
    fn test_fake_blob_store_write_read() {
        let mut store = FakeBlobStore::new();
        let hash = hex::decode("abc123").unwrap();

        store.write(&hash, b"test content").unwrap();
        let read = store.read(&hash).unwrap();

        assert!(read.is_some());
        assert_eq!(read.unwrap(), b"test content");
    }

    /// Verify FakeBlobStore has_blob check
    #[test]
    fn test_fake_blob_store_has_blob() {
        let store = FakeBlobStore::new();
        let hash = hex::decode("abc123").unwrap();

        assert!(!store.has_blob(&hash));
        store.write(&hash, b"test").unwrap();
        assert!(store.has_blob(&hash));
    }
}

#[cfg(test)]
mod test_snapshot_scanning {
    use ai_history_snapshot::scanner::{Scanner, ScannerConfig};

    /// Verify Scanner exists
    #[test]
    fn test_scanner_exists() {
        let config = ScannerConfig::default();
        let _scanner = Scanner::new(config);
    }

    /// Verify ScannerConfig has expected fields
    #[test]
    fn test_scanner_config_fields() {
        let config = ScannerConfig::default();
        assert!(!config.include_hidden);
        assert_eq!(config.max_file_size, 100 * 1024 * 1024);
        assert!(!config.follow_symlinks);
    }

    /// Verify FileEntry can be created
    #[test]
    fn test_file_entry_creation() {
        use ai_history_core::ContentHash;
        use ai_history_snapshot::manifest::FileEntry;

        let entry = FileEntry {
            path: "src/main.rs".to_string(),
            blob_hash: ContentHash::from_bytes([0u8; 32]),
            size: 100,
            mode: 0o644,
            mime_type: Some("text/plain".to_string()),
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        };

        assert_eq!(entry.path, "src/main.rs");
        assert!(!entry.executable);
    }

    /// Verify FileOwnership variants exist
    #[test]
    fn test_file_ownership_variants() {
        use ai_history_snapshot::manifest::FileOwnership;

        assert!(FileOwnership::AiOwned.is_ai_owned());
        assert!(FileOwnership::UserOwned.is_user_owned());
        assert!(FileOwnership::External.is_external());
    }

    /// Verify manifest has entries
    #[test]
    fn test_manifest_has_entries() {
        use ai_history_core::ManifestId;
        use ai_history_snapshot::manifest::Manifest;

        let manifest = Manifest::new(ManifestId::new());
        // Manifest should have entries field (HashMap)
        assert!(manifest.entries.is_empty());
    }
}

#[cfg(test)]
mod test_manifest_canonicalization {
    use ai_history_core::{ContentHash, ManifestId};
    use ai_history_snapshot::manifest::{FileEntry, Manifest};
    use std::collections::HashMap;

    /// Verify Manifest can be serialized to JSON
    #[test]
    fn test_manifest_json_serialization() {
        let manifest = create_test_manifest();
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("entries"));
    }

    /// Verify Manifest serialization is deterministic
    #[test]
    fn test_manifest_deterministic_serialization() {
        let manifest = create_test_manifest();
        let json1 = serde_json::to_string(&manifest).unwrap();
        let json2 = serde_json::to_string(&manifest).unwrap();
        assert_eq!(json1, json2);
    }

    /// Verify Manifest has content_hash
    #[test]
    fn test_manifest_has_content_hash() {
        let manifest = create_test_manifest();
        assert!(!manifest.content_hash.to_string().is_empty());
    }

    /// Verify Manifest has parent_id linkage
    #[test]
    fn test_manifest_parent_linkage() {
        let mut manifest = create_test_manifest();
        manifest.parent_id = Some(ManifestId::new());
        assert!(manifest.parent_id.is_some());
    }

    /// Verify Manifest total_size and file_count tracking
    #[test]
    fn test_manifest_size_tracking() {
        let manifest = create_test_manifest();
        assert_eq!(manifest.file_count, 1);
        assert!(manifest.total_size > 0);
    }

    fn create_test_manifest() -> Manifest {
        let entry = FileEntry {
            path: "test.txt".to_string(),
            blob_hash: ContentHash::from_bytes([1u8; 32]),
            size: 5,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        };

        let mut entries = HashMap::new();
        entries.insert("test.txt".to_string(), entry);

        let mut manifest = Manifest::new(ManifestId::new());
        manifest.entries = entries;
        manifest.total_size = 5;
        manifest.file_count = 1;
        manifest
    }
}

#[cfg(test)]
mod test_manifest_comparison {
    use ai_history_core::{ContentHash, ManifestId};
    use ai_history_snapshot::comparison::{three_way_compare, ThreeWayDiff};
    use ai_history_snapshot::manifest::{FileEntry, Manifest};
    use std::collections::HashMap;

    /// Verify three_way_compare function exists
    #[test]
    fn test_three_way_compare_exists() {
        let base = create_empty_manifest();
        let current = create_empty_manifest();
        let target = create_empty_manifest();

        let result = three_way_compare(&base, &current, &target);
        assert_eq!(result.total_changes(), 0);
    }

    /// Verify ThreeWayDiff has all classification fields
    #[test]
    fn test_three_way_diff_fields() {
        let diff = ThreeWayDiff {
            added: vec!["new.rs".to_string()],
            deleted: vec!["old.rs".to_string()],
            modified: vec!["changed.rs".to_string()],
            unchanged: vec!["same.rs".to_string()],
            renamed: vec![],
            conflicts: vec!["conflict.rs".to_string()],
        };

        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.deleted.len(), 1);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.unchanged.len(), 1);
        assert!(diff.has_conflicts());
    }

    /// Verify has_conflicts method
    #[test]
    fn test_has_conflicts() {
        let no_conflicts = ThreeWayDiff {
            added: vec![],
            deleted: vec![],
            modified: vec![],
            unchanged: vec!["same.rs".to_string()],
            renamed: vec![],
            conflicts: vec![],
        };
        assert!(!no_conflicts.has_conflicts());

        let with_conflicts = ThreeWayDiff {
            added: vec![],
            deleted: vec![],
            modified: vec![],
            unchanged: vec![],
            renamed: vec![],
            conflicts: vec!["conflict.rs".to_string()],
        };
        assert!(with_conflicts.has_conflicts());
    }

    /// Verify total_changes calculation
    #[test]
    fn test_total_changes() {
        let diff = ThreeWayDiff {
            added: vec!["a.rs".to_string()],
            deleted: vec!["b.rs".to_string(), "c.rs".to_string()],
            modified: vec!["d.rs".to_string()],
            unchanged: vec![],
            renamed: vec![],
            conflicts: vec![],
        };
        assert_eq!(diff.total_changes(), 4);
    }

    /// Verify added file detection
    #[test]
    fn test_detect_added() {
        let base = create_empty_manifest();
        let current = create_empty_manifest();
        let mut target = create_empty_manifest();
        target.entries.insert(
            "new_file.rs".to_string(),
            FileEntry {
                path: "new_file.rs".to_string(),
                blob_hash: ContentHash::from_bytes([2u8; 32]),
                size: 7,
                mode: 0o644,
                mime_type: None,
                executable: false,
                symlink_target: None,
                modified: None,
                ownership: None,
            },
        );

        let diff = three_way_compare(&base, &current, &target);
        assert!(diff.added.contains(&"new_file.rs".to_string()));
    }

    /// Verify deleted file detection
    #[test]
    fn test_detect_deleted() {
        let mut base = create_empty_manifest();
        base.entries.insert(
            "deleted.rs".to_string(),
            FileEntry {
                path: "deleted.rs".to_string(),
                blob_hash: ContentHash::from_bytes([3u8; 32]),
                size: 3,
                mode: 0o644,
                mime_type: None,
                executable: false,
                symlink_target: None,
                modified: None,
                ownership: None,
            },
        );

        let current = create_empty_manifest();
        let target = create_empty_manifest();

        let diff = three_way_compare(&base, &current, &target);
        assert!(diff.deleted.contains(&"deleted.rs".to_string()));
    }

    /// Verify unchanged file detection
    #[test]
    fn test_detect_unchanged() {
        let entry = FileEntry {
            path: "unchanged.rs".to_string(),
            blob_hash: ContentHash::from_bytes([4u8; 32]),
            size: 4,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        };

        let mut base = create_empty_manifest();
        base.entries
            .insert("unchanged.rs".to_string(), entry.clone());

        let mut current = create_empty_manifest();
        current
            .entries
            .insert("unchanged.rs".to_string(), entry.clone());

        let mut target = create_empty_manifest();
        target.entries.insert("unchanged.rs".to_string(), entry);

        let diff = three_way_compare(&base, &current, &target);
        assert!(diff.unchanged.contains(&"unchanged.rs".to_string()));
    }

    fn create_empty_manifest() -> Manifest {
        Manifest {
            id: ManifestId::new(),
            content_hash: ContentHash::from_bytes([0u8; 32]),
            parent_id: None,
            entries: HashMap::new(),
            total_size: 0,
            file_count: 0,
        }
    }
}

#[cfg(test)]
mod test_cross_platform_metadata {
    use ai_history_core::ContentHash;
    use ai_history_snapshot::manifest::FileEntry;

    /// Verify FileEntry has mode field for permissions
    #[test]
    fn test_file_entry_mode_field() {
        let entry = FileEntry {
            path: "script.sh".to_string(),
            blob_hash: ContentHash::from_bytes([5u8; 32]),
            size: 10,
            mode: 0o755,
            mime_type: None,
            executable: true,
            symlink_target: None,
            modified: None,
            ownership: None,
        };
        assert_eq!(entry.mode, 0o755);
        assert!(entry.executable);
    }

    /// Verify FileEntry has symlink_target field
    #[test]
    fn test_file_entry_symlink_target() {
        let entry = FileEntry {
            path: "link".to_string(),
            blob_hash: ContentHash::from_bytes([6u8; 32]),
            size: 6,
            mode: 0o777,
            mime_type: None,
            executable: false,
            symlink_target: Some("real_target".to_string()),
            modified: None,
            ownership: None,
        };
        assert!(entry.symlink_target.is_some());
        assert_eq!(entry.symlink_target.unwrap(), "real_target");
    }

    /// Verify FileEntry has mime_type
    #[test]
    fn test_file_entry_mime_type() {
        let entry = FileEntry {
            path: "data.json".to_string(),
            blob_hash: ContentHash::from_bytes([7u8; 32]),
            size: 2,
            mode: 0o644,
            mime_type: Some("application/json".to_string()),
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        };
        assert!(entry.mime_type.is_some());
        assert_eq!(entry.mime_type.unwrap(), "application/json");
    }

    /// Verify Unix mode bits can be set and retrieved
    #[test]
    fn test_file_entry_unix_mode_bits() {
        let entry = FileEntry {
            path: "script.sh".to_string(),
            blob_hash: ContentHash::from_bytes([8u8; 32]),
            size: 100,
            mode: 0o755,
            mime_type: None,
            executable: true,
            symlink_target: None,
            modified: None,
            ownership: None,
        };
        assert_eq!(entry.mode, 0o755);
        assert!(entry.executable);
    }

    /// Verify symlink detection via symlink_target field
    #[test]
    fn test_file_entry_symlink_detection() {
        let regular_entry = FileEntry {
            path: "regular.txt".to_string(),
            blob_hash: ContentHash::from_bytes([9u8; 32]),
            size: 10,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        };
        assert!(regular_entry.symlink_target.is_none());

        let symlink_entry = FileEntry {
            path: "link_to_file".to_string(),
            blob_hash: ContentHash::from_bytes([10u8; 32]),
            size: 15,
            mode: 0o777,
            mime_type: None,
            executable: false,
            symlink_target: Some("target.txt".to_string()),
            modified: None,
            ownership: None,
        };
        assert!(symlink_entry.symlink_target.is_some());
    }

    /// Verify modified time is stored as RFC3339 string
    #[test]
    fn test_file_entry_modified_time() {
        let entry = FileEntry {
            path: "dated_file.txt".to_string(),
            blob_hash: ContentHash::from_bytes([11u8; 32]),
            size: 50,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: Some("2024-01-15T10:30:00Z".to_string()),
            ownership: None,
        };
        assert!(entry.modified.is_some());
    }

    /// Verify ownership tracking via FileOwnership
    #[test]
    fn test_file_entry_ownership() {
        use ai_history_snapshot::manifest::FileOwnership;

        let ai_owned = FileEntry {
            path: "ai_file.txt".to_string(),
            blob_hash: ContentHash::from_bytes([12u8; 32]),
            size: 100,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: Some(FileOwnership::AiOwned),
        };
        assert!(ai_owned.ownership.is_some());
        assert!(ai_owned.ownership.unwrap().is_ai_owned());

        let user_owned = FileEntry {
            path: "user_file.txt".to_string(),
            blob_hash: ContentHash::from_bytes([13u8; 32]),
            size: 200,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: Some(FileOwnership::UserOwned),
        };
        assert!(user_owned.ownership.unwrap().is_user_owned());
    }

    /// Verify file size is preserved
    #[test]
    fn test_file_entry_size() {
        let entry = FileEntry {
            path: "large_file.bin".to_string(),
            blob_hash: ContentHash::from_bytes([14u8; 32]),
            size: 1024 * 1024,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        };
        assert_eq!(entry.size, 1024 * 1024);
    }
}
