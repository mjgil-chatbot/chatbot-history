//! Snapshot scope verification tests
//!
//! Task 9: Verify snapshot owns scanning, ignore rules, path validation,
//! blob writes, manifest creation/canonicalization, parent linkage,
//! manifest comparison, and platform metadata.

#[cfg(test)]
mod tests {
    use crate::{three_way_compare, BlobStore, FileEntry, FileMetadata, Manifest, ScannerConfig};
    use ai_history_core::{BlobStoreConfig, ContentHash, IgnoreConfig, ManifestId};
    use tempfile::TempDir;

    fn create_blob_store() -> (BlobStore, TempDir) {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("blobs");
        let store = BlobStore::new(root).unwrap();
        (store, temp)
    }

    #[test]
    fn test_blob_store_config_default() {
        let config = BlobStoreConfig::default();
        assert!(config.atomic_writes);
    }

    #[test]
    fn test_blob_store_config_with_atomic_writes_disabled() {
        let config = BlobStoreConfig::default().with_atomic_writes(false);
        assert!(!config.atomic_writes);
    }

    #[test]
    fn test_blob_store_write_and_read() {
        let (mut store, _temp) = create_blob_store();
        let data = b"hello world";
        let hash = store.write(data).unwrap();
        assert_eq!(hash.to_hex().len(), 64);
        let read = store.read(&hash).unwrap();
        assert_eq!(read, Some(data.to_vec()));
    }

    #[test]
    fn test_blob_store_deduplication() {
        let (mut store, _temp) = create_blob_store();
        let data = b"test content";
        let hash1 = store.write(data).unwrap();
        let hash2 = store.write(data).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_blob_store_has() {
        let (mut store, _temp) = create_blob_store();
        let data = b"test";
        let hash = store.write(data).unwrap();
        assert!(store.has(&hash));
        let fake_hash = ContentHash::new(b"nonexistent");
        assert!(!store.has(&fake_hash));
    }

    #[test]
    fn test_blob_store_missing_returns_none() {
        let (mut store, _temp) = create_blob_store();
        let fake_hash = ContentHash::new(b"missing");
        let result = store.read(&fake_hash);
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_file_metadata_creation() {
        let metadata = FileMetadata {
            size: 1024,
            mode: 0o755,
            executable: true,
            is_symlink: false,
            symlink_target: None,
        };
        assert_eq!(metadata.size, 1024);
        assert!(metadata.executable);
    }

    #[test]
    fn test_scanner_config_default() {
        let config = ScannerConfig::default();
        assert!(!config.include_hidden);
        assert_eq!(config.max_file_size, 100 * 1024 * 1024); // 100MB
        assert!(!config.follow_symlinks);
    }

    #[test]
    fn test_scanner_config_custom() {
        let config = ScannerConfig {
            include_hidden: true,
            max_file_size: 50 * 1024 * 1024,
            follow_symlinks: true,
            ignore_config: IgnoreConfig::default(),
        };
        assert!(config.include_hidden);
        assert_eq!(config.max_file_size, 50 * 1024 * 1024);
        assert!(config.follow_symlinks);
    }

    #[test]
    fn test_ignore_config_default() {
        let config = IgnoreConfig::default();
        // Default has built-in generated file patterns
        assert!(!config.generated_file_patterns.is_empty());
    }

    #[test]
    fn test_manifest_creation() {
        let manifest = Manifest::new(ManifestId::new());
        assert!(manifest.entries.is_empty());
        assert!(manifest.parent().is_none());
    }

    #[test]
    fn test_manifest_with_entries() {
        let mut manifest = Manifest::new(ManifestId::new());
        let entry = FileEntry {
            path: "file.txt".to_string(),
            blob_hash: ContentHash::new(b"content"),
            size: 100,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        };
        manifest.add_entry(entry);
        assert_eq!(manifest.entries.len(), 1);
    }

    #[test]
    fn test_manifest_parent_linkage() {
        let parent_id = ManifestId::new();
        let manifest = Manifest::with_parent(ManifestId::new(), parent_id);
        assert_eq!(manifest.parent(), Some(parent_id));
    }

    #[test]
    fn test_file_entry_creation() {
        let entry = FileEntry {
            path: "test.txt".to_string(),
            blob_hash: ContentHash::new(b"content"),
            size: 100,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        };
        assert_eq!(entry.path, "test.txt");
        assert!(!entry.executable);
    }

    #[test]
    fn test_file_ownership_enum() {
        use crate::FileOwnership;
        let ai_owned = FileOwnership::AiOwned;
        let user_owned = FileOwnership::UserOwned;
        let external = FileOwnership::External;
        assert!(ai_owned.is_ai_owned());
        assert!(!ai_owned.is_user_owned());
        assert!(user_owned.is_user_owned());
        assert!(external.is_external());
    }

    #[test]
    fn test_content_hash_deterministic() {
        let data = b"test content";
        let hash1 = ContentHash::new(data);
        let hash2 = ContentHash::new(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_content_hash_different_for_different_content() {
        let hash1 = ContentHash::new(b"content A");
        let hash2 = ContentHash::new(b"content B");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_manifest_serialization_round_trip() {
        let mut manifest = Manifest::new(ManifestId::new());
        let entry = FileEntry {
            path: "test.txt".to_string(),
            blob_hash: ContentHash::new(b"hello"),
            size: 5,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        };
        manifest.add_entry(entry);
        manifest.finalize();
        let serialized = serde_json::to_string(&manifest).unwrap();
        let deserialized: Manifest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(manifest.entries.len(), deserialized.entries.len());
    }

    #[test]
    fn test_blob_store_multiple_writes() {
        let (mut store, _temp) = create_blob_store();
        let data1 = b"first content";
        let data2 = b"second content";
        let hash1 = store.write(data1).unwrap();
        let hash2 = store.write(data2).unwrap();
        assert_ne!(hash1, hash2);
        assert_eq!(store.read(&hash1).unwrap(), Some(data1.to_vec()));
        assert_eq!(store.read(&hash2).unwrap(), Some(data2.to_vec()));
    }

    #[test]
    fn test_manifest_finalize_computes_hash() {
        let mut manifest = Manifest::new(ManifestId::new());
        let entry = FileEntry {
            path: "test.txt".to_string(),
            blob_hash: ContentHash::new(b"hello"),
            size: 5,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        };
        manifest.add_entry(entry);
        manifest.finalize();
        // Content hash should be computed after finalize
        assert!(manifest.content_hash() != ContentHash::default());
    }

    #[test]
    fn test_three_way_diff_empty() {
        let base = Manifest::new(ManifestId::new());
        let current = Manifest::new(ManifestId::new());
        let target = Manifest::new(ManifestId::new());
        let diff = three_way_compare(&base, &current, &target);
        assert!(diff.added.is_empty());
        assert!(diff.deleted.is_empty());
        assert!(diff.modified.is_empty());
        assert!(diff.unchanged.is_empty());
    }

    #[test]
    fn test_three_way_diff_total_changes() {
        let base = Manifest::new(ManifestId::new());
        let current = Manifest::new(ManifestId::new());
        let target = Manifest::new(ManifestId::new());
        let diff = three_way_compare(&base, &current, &target);
        assert_eq!(diff.total_changes(), 0);
        assert!(!diff.has_conflicts());
    }
}
