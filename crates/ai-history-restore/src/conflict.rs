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
