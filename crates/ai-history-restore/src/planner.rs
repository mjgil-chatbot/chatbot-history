//! Restore planning

use super::conflict::{detect_conflicts, ConflictPlan, ConflictResolution, DirtyState};
use ai_history_core::{CheckpointId, ContentHash, CoreResult, SessionId};
use ai_history_snapshot::manifest::Manifest;

/// Mode for restore operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RestoreMode {
    ConversationOnly,
    WorkspaceOnly,
    Both,
    Fork,
}

impl RestoreMode {
    pub fn is_workspace_only(self) -> bool {
        matches!(self, RestoreMode::WorkspaceOnly)
    }

    pub fn is_conversation_only(self) -> bool {
        matches!(self, RestoreMode::ConversationOnly)
    }

    pub fn includes_workspace(self) -> bool {
        matches!(self, RestoreMode::WorkspaceOnly | RestoreMode::Both)
    }

    pub fn includes_conversation(self) -> bool {
        matches!(self, RestoreMode::ConversationOnly | RestoreMode::Both)
    }
}

/// A restore plan describing what will happen
#[derive(Debug, Clone)]
pub struct RestorePlan {
    pub checkpoint_id: CheckpointId,
    pub session_id: SessionId,
    pub mode: RestoreMode,
    pub safety_checkpoint_id: Option<CheckpointId>,
    pub operations: Vec<RestoreOperation>,
    pub conflicts: Vec<ConflictPlan>,
}

impl RestorePlan {
    pub fn new(checkpoint_id: CheckpointId, session_id: SessionId, mode: RestoreMode) -> Self {
        Self {
            checkpoint_id,
            session_id,
            mode,
            safety_checkpoint_id: None,
            operations: Vec::new(),
            conflicts: Vec::new(),
        }
    }

    pub fn with_safety_checkpoint(mut self, id: CheckpointId) -> Self {
        self.safety_checkpoint_id = Some(id);
        self
    }

    pub fn add_operation(&mut self, op: RestoreOperation) {
        self.operations.push(op);
    }

    pub fn add_conflict(&mut self, conflict: ConflictPlan) {
        self.conflicts.push(conflict);
    }

    pub fn is_safe(&self) -> bool {
        self.conflicts.is_empty() && !self.operations.is_empty()
    }

    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    pub fn total_files(&self) -> usize {
        self.operations.len()
    }
}

/// A single restore operation
#[derive(Debug, Clone)]
pub enum RestoreOperation {
    CreateFile {
        path: String,
        blob_hash: ContentHash,
    },
    UpdateFile {
        path: String,
        blob_hash: ContentHash,
    },
    DeleteFile {
        path: String,
    },
    CreateDir {
        path: String,
    },
    NoOp {
        path: String,
    },
}

impl RestoreOperation {
    pub fn path(&self) -> &str {
        match self {
            RestoreOperation::CreateFile { path, .. } => path,
            RestoreOperation::UpdateFile { path, .. } => path,
            RestoreOperation::DeleteFile { path, .. } => path,
            RestoreOperation::CreateDir { path, .. } => path,
            RestoreOperation::NoOp { path } => path,
        }
    }
}

/// Mode for conflict policy during restore
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictPolicy {
    /// Automatically resolve conflicts using default behavior
    Automatic,
    /// Always prompt user for conflict resolution
    Prompt,
    /// Cancel restore when conflicts are detected
    Cancel,
}

impl Default for ConflictPolicy {
    fn default() -> Self {
        ConflictPolicy::Automatic
    }
}

/// Build a restore plan from current manifest, target manifest, and policies
pub fn build_restore_plan(
    current_manifest: &Manifest,
    target_manifest: &Manifest,
    _safety_snapshot_manifest: Option<&Manifest>,
    policy: ConflictPolicy,
    mode: RestoreMode,
) -> RestorePlan {
    let mut plan = RestorePlan::new(
        ai_history_core::CheckpointId::new(),
        ai_history_core::SessionId::new(),
        mode,
    );

    // Detect conflicts between current and target manifests
    let conflicts = detect_conflicts(current_manifest, target_manifest);

    // Apply conflict policy to determine which conflicts to resolve automatically
    let resolved_conflicts = resolve_conflicts_with_policy(&conflicts, policy);
    for conflict in resolved_conflicts {
        plan.add_conflict(conflict);
    }

    // Collect all paths from both manifests
    let all_paths: std::collections::HashSet<_> = current_manifest
        .entries
        .keys()
        .chain(target_manifest.entries.keys())
        .collect();

    for path in all_paths {
        let path_str: &str = &path;
        let current_entry = current_manifest.entries.get(path_str);
        let target_entry = target_manifest.entries.get(path_str);

        match (current_entry, target_entry) {
            // File in target but not current -> Create
            (None, Some(target_e)) => {
                plan.add_operation(RestoreOperation::CreateFile {
                    path: path.to_string(),
                    blob_hash: target_e.blob_hash,
                });
            }
            // File in current but not target -> Delete
            (Some(_), None) => {
                // Check if this file was modified (has a conflict)
                if let Some(conflict) = plan.conflicts.iter().find(|c| c.path == path_str) {
                    // If conflict is resolved with KeepCurrent, skip delete
                    if conflict.resolution == Some(ConflictResolution::KeepCurrent) {
                        continue;
                    }
                }
                plan.add_operation(RestoreOperation::DeleteFile {
                    path: path.to_string(),
                });
            }
            // File in both - check if content changed
            (Some(current_e), Some(target_e)) => {
                if current_e.blob_hash != target_e.blob_hash {
                    // Check if this file has a conflict
                    if let Some(conflict) = plan.conflicts.iter().find(|c| c.path == path_str) {
                        // If conflict is resolved with KeepCurrent, skip update
                        if conflict.resolution == Some(ConflictResolution::KeepCurrent) {
                            plan.add_operation(RestoreOperation::NoOp {
                                path: path.to_string(),
                            });
                            continue;
                        }
                    }
                    // Content differs -> Update
                    plan.add_operation(RestoreOperation::UpdateFile {
                        path: path.to_string(),
                        blob_hash: target_e.blob_hash,
                    });
                } else {
                    // Same content -> NoOp
                    plan.add_operation(RestoreOperation::NoOp {
                        path: path.to_string(),
                    });
                }
            }
            // Impossible case since path comes from union of both manifests
            (None, None) => unreachable!(),
        }
    }

    // If policy is to cancel on conflicts, clear the operations
    if matches!(policy, ConflictPolicy::Cancel) && plan.has_conflicts() {
        plan.operations.clear();
    }

    plan
}

/// Apply conflict policy to resolve conflicts automatically
fn resolve_conflicts_with_policy(
    conflicts: &[ConflictPlan],
    policy: ConflictPolicy,
) -> Vec<ConflictPlan> {
    match policy {
        ConflictPolicy::Cancel => {
            // Don't resolve - caller will handle
            conflicts.to_vec()
        }
        ConflictPolicy::Prompt => {
            // Don't resolve automatically - caller will prompt
            conflicts.to_vec()
        }
        ConflictPolicy::Automatic => {
            // Auto-resolve based on dirty state
            conflicts
                .iter()
                .map(|conflict| {
                    let resolution = match conflict.dirty_state {
                        // AI-owned changes are typically safe to keep
                        DirtyState::AiOwned => Some(ConflictResolution::KeepCurrent),
                        // User-owned changes should be preserved
                        DirtyState::UserOwned => Some(ConflictResolution::KeepCurrent),
                        // External changes - default to restoring checkpoint for safety
                        DirtyState::External => Some(ConflictResolution::RestoreCheckpoint),
                        // Mixed changes - be conservative and restore checkpoint
                        DirtyState::Mixed => Some(ConflictResolution::RestoreCheckpoint),
                        // Deleted files - restore them
                        DirtyState::Deleted => Some(ConflictResolution::RestoreCheckpoint),
                        // Renamed files - restore them
                        DirtyState::Renamed { .. } => Some(ConflictResolution::RestoreCheckpoint),
                        // Unknown - default to restoring checkpoint for safety
                        DirtyState::Unknown => Some(ConflictResolution::RestoreCheckpoint),
                    };
                    ConflictPlan {
                        resolution,
                        ..conflict.clone()
                    }
                })
                .collect()
        }
    }
}

/// Create a restore plan from current manifest, target manifest, and policies
#[deprecated]
pub fn create_restore_plan(
    _checkpoint_id: CheckpointId,
    _session_id: SessionId,
    _mode: RestoreMode,
) -> CoreResult<RestorePlan> {
    // This is deprecated - use build_restore_plan instead
    let plan = RestorePlan::new(
        ai_history_core::CheckpointId::new(),
        ai_history_core::SessionId::new(),
        RestoreMode::Both,
    );
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_history_core::ContentHash;
    use ai_history_snapshot::manifest::FileEntry;

    fn create_test_manifest(files: Vec<(&str, &str)>) -> Manifest {
        let mut manifest = Manifest::new(ai_history_core::ManifestId::new());
        for (path, content) in files {
            manifest.add_entry(FileEntry {
                path: path.to_string(),
                blob_hash: ContentHash::new(content.as_bytes()),
                size: content.len() as u64,
                mode: 0o644,
                mime_type: Some("text/plain".to_string()),
                executable: false,
                symlink_target: None,
                modified: None,
                ownership: None,
            });
        }
        manifest.finalize();
        manifest
    }

    #[test]
    fn test_restore_plan_build_create() {
        let current = create_test_manifest(vec![]);
        let target = create_test_manifest(vec![("new.txt", "content")]);

        let plan = build_restore_plan(
            &current,
            &target,
            None,
            ConflictPolicy::Automatic,
            RestoreMode::Both,
        );

        // New file should be created
        assert!(plan.operations.iter().any(|op| {
            matches!(op, RestoreOperation::CreateFile { path, .. } if path == "new.txt")
        }));
    }

    #[test]
    fn test_restore_plan_build_delete() {
        let current = create_test_manifest(vec![("deleted.txt", "content")]);
        let target = create_test_manifest(vec![]);

        let plan = build_restore_plan(
            &current,
            &target,
            None,
            ConflictPolicy::Automatic,
            RestoreMode::Both,
        );

        // File should be deleted
        assert!(plan.operations.iter().any(|op| {
            matches!(op, RestoreOperation::DeleteFile { path } if path == "deleted.txt")
        }));
    }

    #[test]
    fn test_restore_plan_build_update() {
        let current = create_test_manifest(vec![("file.txt", "old content")]);
        let target = create_test_manifest(vec![("file.txt", "new content")]);

        let plan = build_restore_plan(
            &current,
            &target,
            None,
            ConflictPolicy::Automatic,
            RestoreMode::Both,
        );

        // File should be updated
        assert!(plan.operations.iter().any(|op| {
            matches!(op, RestoreOperation::UpdateFile { path, .. } if path == "file.txt")
        }));
    }

    #[test]
    fn test_restore_plan_conflict_resolution() {
        use ai_history_snapshot::manifest::FileOwnership;

        let mut current = Manifest::new(ai_history_core::ManifestId::new());
        current.add_entry(FileEntry {
            path: "ai_modified.rs".to_string(),
            blob_hash: ContentHash::new(b"ai content"),
            size: 10,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: Some(FileOwnership::AiOwned),
        });
        current.add_entry(FileEntry {
            path: "user_modified.rs".to_string(),
            blob_hash: ContentHash::new(b"user content"),
            size: 11,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: Some(FileOwnership::UserOwned),
        });
        current.add_entry(FileEntry {
            path: "external.txt".to_string(),
            blob_hash: ContentHash::new(b"external"),
            size: 8,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: Some(FileOwnership::External),
        });

        let mut target = Manifest::new(ai_history_core::ManifestId::new());
        target.add_entry(FileEntry {
            path: "ai_modified.rs".to_string(),
            blob_hash: ContentHash::new(b"old ai"),
            size: 7,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        target.add_entry(FileEntry {
            path: "user_modified.rs".to_string(),
            blob_hash: ContentHash::new(b"old user"),
            size: 8,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        target.add_entry(FileEntry {
            path: "external.txt".to_string(),
            blob_hash: ContentHash::new(b"old external"),
            size: 13,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });

        let plan = build_restore_plan(
            &current,
            &target,
            None,
            ConflictPolicy::Automatic,
            RestoreMode::Both,
        );

        // AI-owned and User-owned files should be kept (NoOp instead of Update)
        let ai_op = plan
            .operations
            .iter()
            .find(|op| op.path() == "ai_modified.rs");
        assert!(matches!(ai_op, Some(RestoreOperation::NoOp { .. })));

        let user_op = plan
            .operations
            .iter()
            .find(|op| op.path() == "user_modified.rs");
        assert!(matches!(user_op, Some(RestoreOperation::NoOp { .. })));

        // External files should be updated (checkpoint restored)
        let ext_op = plan
            .operations
            .iter()
            .find(|op| op.path() == "external.txt");
        assert!(matches!(ext_op, Some(RestoreOperation::UpdateFile { .. })));

        // Check conflict resolutions
        let ai_conflict = plan
            .conflicts
            .iter()
            .find(|c| c.path == "ai_modified.rs")
            .unwrap();
        assert_eq!(
            ai_conflict.resolution,
            Some(ConflictResolution::KeepCurrent)
        );

        let ext_conflict = plan
            .conflicts
            .iter()
            .find(|c| c.path == "external.txt")
            .unwrap();
        assert_eq!(
            ext_conflict.resolution,
            Some(ConflictResolution::RestoreCheckpoint)
        );
    }

    #[test]
    fn test_restore_plan_build_noop() {
        let content = "same content";
        let current = create_test_manifest(vec![("file.txt", content)]);
        let target = create_test_manifest(vec![("file.txt", content)]);

        let plan = build_restore_plan(
            &current,
            &target,
            None,
            ConflictPolicy::Automatic,
            RestoreMode::Both,
        );

        // Unchanged file should be NoOp
        assert!(plan
            .operations
            .iter()
            .any(|op| { matches!(op, RestoreOperation::NoOp { path } if path == "file.txt") }));
    }

    #[test]
    fn test_restore_mode_helper() {
        // Both mode includes both workspace and conversation
        assert!(RestoreMode::Both.includes_workspace());
        assert!(RestoreMode::Both.includes_conversation());
        assert!(!RestoreMode::Both.is_workspace_only());
        assert!(!RestoreMode::Both.is_conversation_only());
        // Single mode checks
        assert!(RestoreMode::ConversationOnly.is_conversation_only());
        assert!(!RestoreMode::ConversationOnly.is_workspace_only());
        assert!(RestoreMode::WorkspaceOnly.is_workspace_only());
        assert!(!RestoreMode::WorkspaceOnly.is_conversation_only());
    }

    #[test]
    fn test_restore_operation_path() {
        let op = RestoreOperation::CreateFile {
            path: "test.txt".to_string(),
            blob_hash: ContentHash::new(b"test"),
        };
        assert_eq!(op.path(), "test.txt");
    }

    #[test]
    fn test_conflict_policy_default() {
        assert_eq!(ConflictPolicy::default(), ConflictPolicy::Automatic);
    }
}
