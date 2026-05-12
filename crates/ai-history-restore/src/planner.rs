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
    }}
impl RestoreMode {
    pub fn is_conversation_only(self) -> bool {
        matches!(self, RestoreMode::ConversationOnly)
    }}
impl RestoreMode {
    pub fn includes_workspace(self) -> bool {
        matches!(self, RestoreMode::WorkspaceOnly | RestoreMode::Both)
    }}
impl RestoreMode {
    pub fn includes_conversation(self) -> bool {
        matches!(self, RestoreMode::ConversationOnly | RestoreMode::Both)
    }}
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
    }}
impl RestorePlan {
    pub fn with_safety_checkpoint(mut self, id: CheckpointId) -> Self {
        self.safety_checkpoint_id = Some(id);
        self
    }}
impl RestorePlan {
    pub fn add_operation(&mut self, op: RestoreOperation) {
        self.operations.push(op);
    }}
impl RestorePlan {
    pub fn add_conflict(&mut self, conflict: ConflictPlan) {
        self.conflicts.push(conflict);
    }}
impl RestorePlan {
    pub fn is_safe(&self) -> bool {
        self.conflicts.is_empty() && !self.operations.is_empty()
    }}
impl RestorePlan {
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }}
impl RestorePlan {
    pub fn total_files(&self) -> usize {
        self.operations.len()
    }}
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
    }}
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
