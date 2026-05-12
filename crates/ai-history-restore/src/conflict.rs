//! Conflict detection and resolution

use serde::{Deserialize, Serialize};
use std::fmt;

/// Result of comparing current workspace with target checkpoint
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DirtyState {
    /// File was modified by the AI assistant
    AiOwned,
    /// File was modified by the user
    UserOwned,
    /// File was modified externally
    External,
    /// File has mixed ownership
    Mixed,
    /// File was deleted
    Deleted,
    /// File was renamed
    Renamed { old_path: String },
    /// Unknown state
    Unknown,
}

impl fmt::Display for DirtyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DirtyState::AiOwned => write!(f, "AI-owned"),
            DirtyState::UserOwned => write!(f, "user-owned"),
            DirtyState::External => write!(f, "external"),
            DirtyState::Mixed => write!(f, "mixed"),
            DirtyState::Deleted => write!(f, "deleted"),
            DirtyState::Renamed { old_path } => write!(f, "renamed from {}", old_path),
            DirtyState::Unknown => write!(f, "unknown"),
        }
    }
}

/// A detected conflict during restore
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictPlan {
    pub path: String,
    pub checkpoint_blob_hash: String,
    pub current_blob_hash: String,
    pub dirty_state: DirtyState,
    pub resolution: Option<ConflictResolution>,
}

/// How to resolve a conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Keep the current workspace state
    KeepCurrent,
    /// Restore the checkpoint version
    RestoreCheckpoint,
    /// Attempt to merge changes
    Merge,
    /// Save current as a safety snapshot
    SaveCurrentAsBranch,
    /// Cancel the restore operation
    Cancel,
}

impl Default for ConflictResolution {
    fn default() -> Self {
        Self::RestoreCheckpoint
    }
}

/// Compare current workspace state with a checkpoint to detect conflicts
pub fn detect_conflicts(
    current_manifest: &ai_history_snapshot::Manifest,
    checkpoint_manifest: &ai_history_snapshot::Manifest,
) -> Vec<ConflictPlan> {
    let mut conflicts = Vec::new();
    let current_paths: std::collections::HashSet<_> = current_manifest.entries.keys().collect();
    let checkpoint_paths: std::collections::HashSet<_> =
        checkpoint_manifest.entries.keys().collect();

    // Find files in checkpoint that differ or don't exist in current
    for (path, entry) in &checkpoint_manifest.entries {
        if let Some(current_entry) = current_manifest.entries.get(path) {
            if current_entry.blob_hash != entry.blob_hash {
                // Use ownership to determine dirty state
                let dirty_state = determine_dirty_state_from_ownership(&current_entry.ownership);
                conflicts.push(ConflictPlan {
                    path: path.clone(),
                    checkpoint_blob_hash: entry.blob_hash.to_string(),
                    current_blob_hash: current_entry.blob_hash.to_string(),
                    dirty_state,
                    resolution: None,
                });
            }
        } else {
            // File exists in checkpoint but not in current - deleted
            conflicts.push(ConflictPlan {
                path: path.clone(),
                checkpoint_blob_hash: entry.blob_hash.to_string(),
                current_blob_hash: String::new(),
                dirty_state: DirtyState::Deleted,
                resolution: None,
            });
        }
    }

    // Find files in current but not in checkpoint (new files)
    for path in current_paths.difference(&checkpoint_paths) {
        let current_entry = current_manifest.entries.get(*path).unwrap();
        // New files - use ownership if available, otherwise External
        let dirty_state = determine_dirty_state_from_ownership(&current_entry.ownership);
        conflicts.push(ConflictPlan {
            path: (*path).clone(),
            checkpoint_blob_hash: String::new(),
            current_blob_hash: current_entry.blob_hash.to_string(),
            dirty_state,
            resolution: None,
        });
    }

    conflicts
}

/// Determine dirty state from file ownership
fn determine_dirty_state_from_ownership(
    ownership: &Option<ai_history_snapshot::manifest::FileOwnership>,
) -> DirtyState {
    match ownership {
        Some(ai_history_snapshot::manifest::FileOwnership::AiOwned) => DirtyState::AiOwned,
        Some(ai_history_snapshot::manifest::FileOwnership::UserOwned) => DirtyState::UserOwned,
        Some(ai_history_snapshot::manifest::FileOwnership::External) => DirtyState::External,
        None => DirtyState::Unknown,
    }
}

/// Overall dirty workspace state
#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceDirtyState {
    /// Workspace matches the latest known snapshot
    Clean,
    /// Workspace has AI-owned changes
    AiDirty,
    /// Workspace has user-owned changes
    UserDirty,
    /// Workspace has external changes
    ExternalDirty,
    /// Workspace has mixed ownership changes
    MixedDirty,
}

/// Detect the overall dirty state of a workspace
pub fn detect_dirty_state(
    current_manifest: &ai_history_snapshot::Manifest,
    checkpoint_manifest: &ai_history_snapshot::Manifest,
) -> WorkspaceDirtyState {
    let conflicts = detect_conflicts(current_manifest, checkpoint_manifest);

    if conflicts.is_empty() {
        return WorkspaceDirtyState::Clean;
    }

    // Count dirty states
    let has_ai = conflicts
        .iter()
        .any(|c| matches!(c.dirty_state, DirtyState::AiOwned));
    let has_user = conflicts
        .iter()
        .any(|c| matches!(c.dirty_state, DirtyState::UserOwned));
    let has_external = conflicts
        .iter()
        .any(|c| matches!(c.dirty_state, DirtyState::External));
    let has_mixed = conflicts
        .iter()
        .any(|c| matches!(c.dirty_state, DirtyState::Mixed));
    // Determine overall state
    if has_mixed || (has_ai && has_user) || (has_ai && has_external && !has_user) {
        WorkspaceDirtyState::MixedDirty
    } else if has_ai && !has_user && !has_external {
        WorkspaceDirtyState::AiDirty
    } else if has_user && !has_ai && !has_external {
        WorkspaceDirtyState::UserDirty
    } else if has_external && !has_ai && !has_user {
        WorkspaceDirtyState::ExternalDirty
    } else {
        WorkspaceDirtyState::MixedDirty
    }
}

/// Check if any conflicts would prevent restore
pub fn has_unresolved_conflicts(conflicts: &[ConflictPlan]) -> bool {
    !conflicts.is_empty()
}

/// Get files that need resolution before restore
pub fn get_conflicted_files(conflicts: &[ConflictPlan]) -> Vec<&str> {
    conflicts.iter().map(|c| c.path.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_state_clean() {
        let manifest = ai_history_snapshot::Manifest::new(ai_history_core::ManifestId::new());
        let state = detect_dirty_state(&manifest, &manifest);
        assert_eq!(state, WorkspaceDirtyState::Clean);
    }

    #[test]
    fn test_detect_conflicts_identical() {
        let manifest = ai_history_snapshot::Manifest::new(ai_history_core::ManifestId::new());
        let conflicts = detect_conflicts(&manifest, &manifest);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_has_unresolved_conflicts() {
        let manifest = ai_history_snapshot::Manifest::new(ai_history_core::ManifestId::new());
        let conflicts = detect_conflicts(&manifest, &manifest);
        assert!(!has_unresolved_conflicts(&conflicts));
    }

    #[test]
    fn test_get_conflicted_files() {
        use ai_history_core::ContentHash;
        use ai_history_snapshot::manifest::FileEntry;

        let mut current = ai_history_snapshot::Manifest::new(ai_history_core::ManifestId::new());
        current.add_entry(FileEntry {
            path: "file1.txt".to_string(),
            blob_hash: ContentHash::new(b"current"),
            size: 7,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });

        let mut checkpoint = ai_history_snapshot::Manifest::new(ai_history_core::ManifestId::new());
        checkpoint.add_entry(FileEntry {
            path: "file2.txt".to_string(),
            blob_hash: ContentHash::new(b"checkpoint"),
            size: 9,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });

        let conflicts = detect_conflicts(&current, &checkpoint);
        let files = get_conflicted_files(&conflicts);
        // file1 is new (Unknown ownership), file2 is deleted in current
        assert_eq!(files.len(), 2);
        // Verify specific dirty states
        let file1_conflict = conflicts.iter().find(|c| c.path == "file1.txt").unwrap();
        assert_eq!(file1_conflict.dirty_state, DirtyState::Unknown);
        let file2_conflict = conflicts.iter().find(|c| c.path == "file2.txt").unwrap();
        assert_eq!(file2_conflict.dirty_state, DirtyState::Deleted);
    }

    #[test]
    fn test_detect_conflicts_with_ownership() {
        use ai_history_core::ContentHash;
        use ai_history_snapshot::manifest::{FileEntry, FileOwnership};

        let mut current = ai_history_snapshot::Manifest::new(ai_history_core::ManifestId::new());
        // AI-modified file
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
        // User-modified file
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
        // External file (no ownership info or external)
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
        // File with no ownership info
        current.add_entry(FileEntry {
            path: "unknown.txt".to_string(),
            blob_hash: ContentHash::new(b"unknown"),
            size: 7,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });

        let mut checkpoint = ai_history_snapshot::Manifest::new(ai_history_core::ManifestId::new());
        // Same files but with different content
        checkpoint.add_entry(FileEntry {
            path: "ai_modified.rs".to_string(),
            blob_hash: ContentHash::new(b"old content"),
            size: 11,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        checkpoint.add_entry(FileEntry {
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
        checkpoint.add_entry(FileEntry {
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
        checkpoint.add_entry(FileEntry {
            path: "unknown.txt".to_string(),
            blob_hash: ContentHash::new(b"old unknown"),
            size: 12,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });

        let conflicts = detect_conflicts(&current, &checkpoint);
        assert_eq!(conflicts.len(), 4);

        // Check each dirty state based on ownership
        let ai_conflict = conflicts
            .iter()
            .find(|c| c.path == "ai_modified.rs")
            .unwrap();
        assert_eq!(ai_conflict.dirty_state, DirtyState::AiOwned);

        let user_conflict = conflicts
            .iter()
            .find(|c| c.path == "user_modified.rs")
            .unwrap();
        assert_eq!(user_conflict.dirty_state, DirtyState::UserOwned);

        let external_conflict = conflicts.iter().find(|c| c.path == "external.txt").unwrap();
        assert_eq!(external_conflict.dirty_state, DirtyState::External);

        let unknown_conflict = conflicts.iter().find(|c| c.path == "unknown.txt").unwrap();
        assert_eq!(unknown_conflict.dirty_state, DirtyState::Unknown);
    }
}
