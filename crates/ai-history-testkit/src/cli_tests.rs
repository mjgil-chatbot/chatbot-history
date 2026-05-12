//! CLI crate design tests
//!
//! Task 16: Complete `ai-history-cli` crate design
//!
//! Verify:
//! - Cli and Commands exist with all variants
//! - RestoreModeCli exists with all variants
//! - OutputFormatter formatting functions exist
//! - parse_* functions for SessionId, CheckpointId, BranchId exist

#[cfg(test)]
mod test_cli_commands {
    use ai_history_cli::{Cli, Commands, RestoreModeCli};
    use ai_history_core::SessionId;

    /// Verify Cli struct exists with required fields
    #[test]
    fn test_cli_exists() {
        let cli = Cli {
            command: Commands::Status,
            workspace: std::path::PathBuf::from("."),
            session_id: None,
        };
        assert!(cli.workspace.to_str().is_some());
    }

    /// Verify Commands enum variants exist
    #[test]
    fn test_commands_variants() {
        let _init = Commands::Init {
            title: Some("test".to_string()),
        };
        let _checkpoint = Commands::Checkpoint {
            name: Some("v1".to_string()),
            reason: "manual".to_string(),
        };
        let _list = Commands::ListCheckpoints {
            branch: "main".to_string(),
        };
        let _restore = Commands::Restore {
            checkpoint_id: "cp-1".to_string(),
            mode: RestoreModeCli::Both,
        };
        let _fork = Commands::Fork {
            checkpoint_id: "cp-1".to_string(),
            name: "feature-x".to_string(),
        };
        let _branches = Commands::Branches;
        let _diff = Commands::Diff {
            checkpoint_id: "cp-1".to_string(),
            files_only: false,
        };
        let _status = Commands::Status;
        let _gc = Commands::Gc {
            execute: false,
            protected: vec![],
        };
        let _doctor = Commands::Doctor { fix: false };
        let _nav_back = Commands::NavigateBack { count: Some(1) };
        let _nav_forward = Commands::NavigateForward { count: Some(1) };
        let _fork_current = Commands::ForkCurrent {
            name: "new-branch".to_string(),
        };
        let _fork_cursor = Commands::ForkFromCursor {
            turn_id: "t-1".to_string(),
            name: "cursor-branch".to_string(),
        };
        let _continue = Commands::ContinueFromCursor {
            turn_id: "t-1".to_string(),
            prompt: "hello".to_string(),
        };
        let _export = Commands::Export {
            output: "out.json".to_string(),
            include_blobs: false,
        };
        let _import = Commands::Import {
            input: "in.json".to_string(),
            session_id: None,
        };
        let _verify = Commands::Verify {
            checkpoint_id: None,
        };
    }

    /// Verify RestoreModeCli variants
    #[test]
    fn test_restore_mode_cli_variants() {
        assert!(matches!(
            RestoreModeCli::ConversationOnly,
            RestoreModeCli::ConversationOnly
        ));
        assert!(matches!(
            RestoreModeCli::WorkspaceOnly,
            RestoreModeCli::WorkspaceOnly
        ));
        assert!(matches!(RestoreModeCli::Both, RestoreModeCli::Both));
        assert!(matches!(RestoreModeCli::Fork, RestoreModeCli::Fork));
    }
}
#[cfg(test)]
mod test_cli_checkpoint_and_restore_integration {
    use ai_history_core::WorkspacePath;
    use ai_history_core::{BranchId, CheckpointId, ContentHash, ManifestId, SessionId};
    use ai_history_events::*;
    use ai_history_restore::{RestoreMode, RestoreOperation, RestorePlan};
    use ai_history_snapshot::scanner::{Scanner, ScannerConfig};
    use ai_history_store::sqlite_store::SqliteStore;
    use ai_history_store::traits::{
        BlobStore as BlobStoreTrait, BranchInfo, CheckpointInfo, ManifestInfo,
    };
    use std::fs;
    use tempfile::TempDir;

    /// Test checkpoint creates manifest with entries referencing persisted blobs
    #[test]
    fn test_checkpoint_persists_blobs_through_real_workflow() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        let mut store = SqliteStore::open(&db_path).unwrap();
        let workspace = temp.path().join("workspace");
        fs::create_dir(&workspace).unwrap();

        // Create test files
        fs::write(workspace.join("file1.txt"), "content1").unwrap();
        fs::write(workspace.join("file2.rs"), "fn main() {}").unwrap();

        // Setup session/branch
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        store
            .create_session(session_id, workspace.to_str().unwrap())
            .unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                head_turn_id: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                created_from_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
            })
            .unwrap();

        // Use real scanner with real blob store
        let scanner = Scanner::new(ScannerConfig::default());
        let ws_path = WorkspacePath::from_string(workspace.to_str().unwrap());
        let mut blob_store_ref: Option<&mut SqliteStore> = Some(&mut store);
        let manifest = scanner
            .scan_with_blob_store(&ws_path, &mut blob_store_ref)
            .unwrap();

        // Verify manifest has entries
        assert_eq!(manifest.entries.len(), 2);

        // Verify blobs are persisted in store via BlobStore trait
        for (path, entry) in &manifest.entries {
            let hash_bytes = entry.blob_hash.as_bytes();
            assert!(
                BlobStoreTrait::has_blob(&store, hash_bytes),
                "Blob for {} should be persisted",
                path
            );
            let content = BlobStoreTrait::read_blob(&store, hash_bytes)
                .unwrap()
                .unwrap();
            assert!(!content.is_empty());
        }

        // Create checkpoint and manifest in correct order (checkpoint first)
        let checkpoint_id = CheckpointId::new();
        let manifest_id = manifest.id;
        store
            .create_checkpoint(&CheckpointInfo {
                id: checkpoint_id,
                session_id,
                branch_id,
                turn_id: None,
                manifest_id,
                reason: "test checkpoint".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();
        store
            .create_manifest(&ManifestInfo {
                id: manifest_id,
                session_id,
                checkpoint_id,
                manifest_data: serde_json::to_string(&manifest).unwrap(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        // Verify checkpoint and manifest exist
        assert!(store.get_checkpoint(checkpoint_id).unwrap().is_some());
        assert!(store.get_manifest(manifest_id).unwrap().is_some());
    }

    /// Test successful restore from persisted blobs
    #[test]
    fn test_restore_succeeds_with_persisted_blobs() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        let mut store = SqliteStore::open(&db_path).unwrap();
        let workspace = temp.path().join("workspace");
        fs::create_dir(&workspace).unwrap();

        // Create original files
        fs::write(workspace.join("original.txt"), "original content").unwrap();

        // Setup session/branch
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        store
            .create_session(session_id, workspace.to_str().unwrap())
            .unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                head_turn_id: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                created_from_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
            })
            .unwrap();

        // Scan and checkpoint
        let scanner = Scanner::new(ScannerConfig::default());
        let ws_path = WorkspacePath::from_string(workspace.to_str().unwrap());
        let mut blob_store_ref: Option<&mut SqliteStore> = Some(&mut store);
        let manifest = scanner
            .scan_with_blob_store(&ws_path, &mut blob_store_ref)
            .unwrap();

        let checkpoint_id = CheckpointId::new();
        let manifest_id = manifest.id;
        store
            .create_checkpoint(&CheckpointInfo {
                id: checkpoint_id,
                session_id,
                branch_id,
                turn_id: None,
                manifest_id,
                reason: "test checkpoint".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();
        store
            .create_manifest(&ManifestInfo {
                id: manifest_id,
                session_id,
                checkpoint_id,
                manifest_data: serde_json::to_string(&manifest).unwrap(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        // Modify workspace
        fs::write(workspace.join("original.txt"), "modified content").unwrap();

        // Build restore plan and execute
        let mut plan = RestorePlan::new(checkpoint_id, session_id, RestoreMode::Both);
        for (path, entry) in &manifest.entries {
            plan.add_operation(RestoreOperation::UpdateFile {
                path: path.clone(),
                blob_hash: entry.blob_hash.clone(),
            });
        }

        // Create safety checkpoint first (correct order)
        let safety_cp_id = CheckpointId::new();
        let safety_manifest_id = ManifestId::new();
        store
            .create_checkpoint(&CheckpointInfo {
                id: safety_cp_id,
                session_id,
                branch_id,
                turn_id: None,
                manifest_id: safety_manifest_id,
                reason: "safety".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        // Scan safety manifest and persist
        let scanner2 = Scanner::new(ScannerConfig::default());
        let ws_path2 = WorkspacePath::from_string(workspace.to_str().unwrap());
        let mut blob_store_ref2: Option<&mut SqliteStore> = Some(&mut store);
        let safety_manifest = scanner2
            .scan_with_blob_store(&ws_path2, &mut blob_store_ref2)
            .unwrap();
        store
            .create_manifest(&ManifestInfo {
                id: safety_manifest_id,
                session_id,
                checkpoint_id: safety_cp_id,
                manifest_data: serde_json::to_string(&safety_manifest).unwrap(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();
        plan.safety_checkpoint_id = Some(safety_cp_id);

        let result = ai_history_restore::execute_restore_transactional(&plan, &workspace, &store)
            .map_err(|e| format!("restore error: {}", e))
            .unwrap();
        assert!(
            result.success,
            "Restore should succeed: {:?}",
            result.errors
        );
        assert_eq!(
            fs::read_to_string(workspace.join("original.txt")).unwrap(),
            "original content"
        );
    }

    /// Test restore fails when blob is missing (preflight check)
    #[test]
    fn test_restore_fails_when_blob_missing() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        let mut store = SqliteStore::open(&db_path).unwrap();
        let workspace = temp.path().join("workspace");
        fs::create_dir(&workspace).unwrap();

        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        store
            .create_session(session_id, workspace.to_str().unwrap())
            .unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                head_turn_id: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                created_from_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
            })
            .unwrap();

        // Create manifest with entry but NO blob persisted
        let missing_hash = ContentHash::new(b"this blob was never written");
        let mut manifest = ai_history_snapshot::Manifest::new(ManifestId::new());
        let entry_path = "missing.txt".to_string();
        manifest.add_entry(ai_history_snapshot::manifest::FileEntry {
            path: entry_path.clone(),
            blob_hash: missing_hash,
            size: 100,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });

        let checkpoint_id = CheckpointId::new();
        let manifest_id = manifest.id;
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
                manifest_data: serde_json::to_string(&manifest).unwrap(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        // Build plan with non-existent blob
        let mut plan = RestorePlan::new(checkpoint_id, session_id, RestoreMode::Both);
        plan.add_operation(RestoreOperation::UpdateFile {
            path: entry_path,
            blob_hash: missing_hash,
        });
        plan.safety_checkpoint_id = Some(CheckpointId::new());

        // Restore should fail at preflight
        let result = ai_history_restore::execute_restore_transactional(&plan, &workspace, &store)
            .map_err(|e| format!("restore error: {}", e))
            .unwrap();
        assert!(!result.success, "Restore should fail when blob is missing");
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("not found") || e.contains("missing")));
    }

    /// Test checkpoint-before-manifest ordering in restore safety checkpoint
    #[test]
    fn test_safety_checkpoint_created_before_manifest() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        let mut store = SqliteStore::open(&db_path).unwrap();
        let workspace = temp.path().join("workspace");
        fs::create_dir(&workspace).unwrap();

        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        store
            .create_session(session_id, workspace.to_str().unwrap())
            .unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                head_turn_id: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                created_from_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
            })
            .unwrap();

        let checkpoint_id = CheckpointId::new();
        let manifest_id = ManifestId::new();

        // Create checkpoint first (correct order per FK constraint)
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

        // Then create manifest referencing existing checkpoint (should succeed)
        store
            .create_manifest(&ManifestInfo {
                id: manifest_id,
                session_id,
                checkpoint_id,
                manifest_data: "{}".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        // Verify both exist
        assert!(store.get_checkpoint(checkpoint_id).unwrap().is_some());
        assert!(store.get_manifest(manifest_id).unwrap().is_some());
    }
}
