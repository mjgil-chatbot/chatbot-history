//! Conflict resolution for workspace restore
//!
//! Provides conflict outcomes:
//! - Keep current, restore checkpoint version, merge when possible
//! - Save current as branch/safety snapshot, or cancel restore

use crate::{CheckpointId, ContentHash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Conflict outcome choices
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictOutcome {
    /// Keep the current workspace state (skip this file)
    KeepCurrent,
    /// Restore the checkpoint version (overwrite current)
    RestoreCheckpoint,
    /// Attempt to merge changes
    Merge,
    /// Save current as safety snapshot then restore checkpoint
    SaveAndRestore,
    /// Cancel this file's restore
    SkipFile,
    /// Cancel entire restore operation
    CancelRestore,
}

impl ConflictOutcome {
    pub fn display_name(&self) -> &'static str {
        match self {
            ConflictOutcome::KeepCurrent => "Keep Current",
            ConflictOutcome::RestoreCheckpoint => "Restore Checkpoint",
            ConflictOutcome::Merge => "Merge",
            ConflictOutcome::SaveAndRestore => "Save & Restore",
            ConflictOutcome::SkipFile => "Skip File",
            ConflictOutcome::CancelRestore => "Cancel All",
        }
    }

    pub fn is_cancellable(&self) -> bool {
        matches!(
            self,
            ConflictOutcome::CancelRestore | ConflictOutcome::SkipFile
        )
    }

    pub fn preserves_current(&self) -> bool {
        matches!(
            self,
            ConflictOutcome::KeepCurrent | ConflictOutcome::Merge | ConflictOutcome::SaveAndRestore
        )
    }
}

/// A single file conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConflict {
    pub path: String,
    pub current_hash: Option<ContentHash>,
    pub checkpoint_hash: Option<ContentHash>,
    pub conflict_type: ConflictType,
    pub suggested_outcome: ConflictOutcome,
}

impl FileConflict {
    pub fn new(
        path: String,
        current_hash: Option<ContentHash>,
        checkpoint_hash: Option<ContentHash>,
        conflict_type: ConflictType,
    ) -> Self {
        let suggested_outcome = Self::suggest_outcome(conflict_type);

        Self {
            path,
            current_hash,
            checkpoint_hash,
            conflict_type,
            suggested_outcome,
        }
    }

    fn suggest_outcome(conflict_type: ConflictType) -> ConflictOutcome {
        match conflict_type {
            ConflictType::DeletedInCheckpoint => ConflictOutcome::KeepCurrent,
            ConflictType::DeletedInCurrent => ConflictOutcome::RestoreCheckpoint,
            ConflictType::ModifiedBoth => ConflictOutcome::Merge,
            ConflictType::AddedInBoth => ConflictOutcome::KeepCurrent,
        }
    }

    pub fn resolve(&self, outcome: ConflictOutcome) -> FileResolution {
        FileResolution {
            path: self.path.clone(),
            outcome,
            current_hash: self.current_hash,
            checkpoint_hash: self.checkpoint_hash,
        }
    }
}

/// Type of conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    /// File was deleted in checkpoint but exists in current
    DeletedInCheckpoint,
    /// File was deleted in current but exists in checkpoint
    DeletedInCurrent,
    /// Both checkpoint and current have modified the file
    ModifiedBoth,
    /// Both added the same file (path conflict)
    AddedInBoth,
}

impl ConflictType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ConflictType::DeletedInCheckpoint => "Deleted in Checkpoint",
            ConflictType::DeletedInCurrent => "Deleted in Current",
            ConflictType::ModifiedBoth => "Modified Both",
            ConflictType::AddedInBoth => "Added in Both",
        }
    }

    pub fn can_merge(&self) -> bool {
        matches!(self, ConflictType::ModifiedBoth)
    }
}

/// Resolution for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResolution {
    pub path: String,
    pub outcome: ConflictOutcome,
    pub current_hash: Option<ContentHash>,
    pub checkpoint_hash: Option<ContentHash>,
}

/// Complete conflict resolution for a restore operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub conflicts: Vec<FileConflict>,
    pub resolutions: HashMap<String, ConflictOutcome>,
    pub was_cancelled: bool,
    pub saved_safety_snapshot_id: Option<CheckpointId>,
}

impl ConflictResolution {
    pub fn new() -> Self {
        Self {
            conflicts: Vec::new(),
            resolutions: HashMap::new(),
            was_cancelled: false,
            saved_safety_snapshot_id: None,
        }
    }

    pub fn with_conflicts(mut self, conflicts: Vec<FileConflict>) -> Self {
        self.conflicts = conflicts;
        self
    }

    /// Add a resolution for a path
    pub fn resolve_path(&mut self, path: &str, outcome: ConflictOutcome) {
        self.resolutions.insert(path.to_string(), outcome);
    }

    /// Set cancelled flag
    pub fn set_cancelled(&mut self) {
        self.was_cancelled = true;
    }

    /// Set safety snapshot
    pub fn set_safety_snapshot(&mut self, checkpoint_id: CheckpointId) {
        self.saved_safety_snapshot_id = Some(checkpoint_id);
    }

    /// Check if all conflicts are resolved
    pub fn is_complete(&self) -> bool {
        self.conflicts
            .iter()
            .all(|c| self.resolutions.contains_key(&c.path))
    }

    /// Check if any conflict requires user attention
    pub fn needs_user_input(&self) -> bool {
        self.conflicts.iter().any(|c| {
            matches!(
                c.conflict_type,
                ConflictType::ModifiedBoth | ConflictType::AddedInBoth
            )
        })
    }

    /// Get unresolved conflicts
    pub fn unresolved(&self) -> Vec<FileConflict> {
        self.conflicts
            .iter()
            .filter(|c| !self.resolutions.contains_key(&c.path))
            .cloned()
            .collect()
    }
}

impl Default for ConflictResolution {
    fn default() -> Self {
        Self::new()
    }
}

/// Conflict UI state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictUiState {
    pub conflict_count: usize,
    pub current_conflict_index: usize,
    pub current_conflict: Option<FileConflict>,
    pub resolutions: HashMap<String, ConflictOutcome>,
    pub can_proceed: bool,
    pub must_cancel: bool,
}

impl ConflictUiState {
    pub fn from_resolution(resolution: &ConflictResolution) -> Self {
        let unresolved = resolution.unresolved();
        let current_conflict = unresolved.first().cloned();

        Self {
            conflict_count: resolution.conflicts.len(),
            current_conflict_index: 0,
            current_conflict,
            resolutions: resolution.resolutions.clone(),
            can_proceed: resolution.is_complete() && !resolution.was_cancelled,
            must_cancel: resolution.was_cancelled,
        }
    }

    pub fn select_outcome(&mut self, path: &str, outcome: ConflictOutcome) {
        self.resolutions.insert(path.to_string(), outcome);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_outcome_display() {
        assert_eq!(ConflictOutcome::KeepCurrent.display_name(), "Keep Current");
        assert_eq!(
            ConflictOutcome::RestoreCheckpoint.display_name(),
            "Restore Checkpoint"
        );
        assert_eq!(ConflictOutcome::Merge.display_name(), "Merge");
        assert_eq!(ConflictOutcome::CancelRestore.display_name(), "Cancel All");
    }

    #[test]
    fn test_conflict_outcome_is_cancellable() {
        assert!(!ConflictOutcome::KeepCurrent.is_cancellable());
        assert!(!ConflictOutcome::RestoreCheckpoint.is_cancellable());
        assert!(ConflictOutcome::SkipFile.is_cancellable());
        assert!(ConflictOutcome::CancelRestore.is_cancellable());
    }

    #[test]
    fn test_conflict_outcome_preserves_current() {
        assert!(ConflictOutcome::KeepCurrent.preserves_current());
        assert!(ConflictOutcome::Merge.preserves_current());
        assert!(!ConflictOutcome::RestoreCheckpoint.preserves_current());
    }

    #[test]
    fn test_conflict_type_display() {
        assert_eq!(
            ConflictType::DeletedInCheckpoint.display_name(),
            "Deleted in Checkpoint"
        );
        assert_eq!(ConflictType::ModifiedBoth.display_name(), "Modified Both");
    }

    #[test]
    fn test_conflict_type_can_merge() {
        assert!(!ConflictType::DeletedInCheckpoint.can_merge());
        assert!(!ConflictType::DeletedInCurrent.can_merge());
        assert!(ConflictType::ModifiedBoth.can_merge());
        assert!(!ConflictType::AddedInBoth.can_merge());
    }

    #[test]
    fn test_file_conflict_new() {
        let current = ContentHash::new(b"current");
        let checkpoint = ContentHash::new(b"checkpoint");

        let conflict = FileConflict::new(
            "test.txt".to_string(),
            Some(current),
            Some(checkpoint),
            ConflictType::ModifiedBoth,
        );

        assert_eq!(conflict.path, "test.txt");
        assert!(conflict.current_hash.is_some());
        assert!(conflict.checkpoint_hash.is_some());
        assert_eq!(conflict.suggested_outcome, ConflictOutcome::Merge);
    }

    #[test]
    fn test_file_conflict_deleted_in_checkpoint() {
        let conflict = FileConflict::new(
            "deleted.txt".to_string(),
            Some(ContentHash::new(b"exists")),
            None,
            ConflictType::DeletedInCheckpoint,
        );

        assert_eq!(conflict.suggested_outcome, ConflictOutcome::KeepCurrent);
    }

    #[test]
    fn test_file_conflict_deleted_in_current() {
        let conflict = FileConflict::new(
            "missing.txt".to_string(),
            None,
            Some(ContentHash::new(b"was_there")),
            ConflictType::DeletedInCurrent,
        );

        assert_eq!(
            conflict.suggested_outcome,
            ConflictOutcome::RestoreCheckpoint
        );
    }

    #[test]
    fn test_file_conflict_resolve() {
        let conflict = FileConflict::new(
            "test.txt".to_string(),
            Some(ContentHash::new(b"a")),
            Some(ContentHash::new(b"b")),
            ConflictType::ModifiedBoth,
        );

        let resolution = conflict.resolve(ConflictOutcome::Merge);
        assert_eq!(resolution.outcome, ConflictOutcome::Merge);
        assert_eq!(resolution.path, "test.txt");
    }

    #[test]
    fn test_conflict_resolution_new() {
        let resolution = ConflictResolution::new();
        assert!(resolution.conflicts.is_empty());
        assert!(!resolution.was_cancelled);
    }

    #[test]
    fn test_conflict_resolution_with_conflicts() {
        let conflicts = vec![
            FileConflict::new(
                "a.txt".to_string(),
                Some(ContentHash::new(b"1")),
                Some(ContentHash::new(b"2")),
                ConflictType::ModifiedBoth,
            ),
            FileConflict::new(
                "b.txt".to_string(),
                None,
                Some(ContentHash::new(b"3")),
                ConflictType::DeletedInCurrent,
            ),
        ];

        let resolution = ConflictResolution::new().with_conflicts(conflicts);
        assert_eq!(resolution.conflicts.len(), 2);
    }

    #[test]
    fn test_conflict_resolution_resolve_path() {
        let mut resolution = ConflictResolution::new();
        resolution.resolve_path("test.txt", ConflictOutcome::KeepCurrent);

        assert_eq!(
            resolution.resolutions.get("test.txt"),
            Some(&ConflictOutcome::KeepCurrent)
        );
    }

    #[test]
    fn test_conflict_resolution_is_complete() {
        let conflicts = vec![
            FileConflict::new(
                "a.txt".to_string(),
                Some(ContentHash::new(b"1")),
                Some(ContentHash::new(b"2")),
                ConflictType::ModifiedBoth,
            ),
            FileConflict::new(
                "b.txt".to_string(),
                None,
                Some(ContentHash::new(b"3")),
                ConflictType::DeletedInCurrent,
            ),
        ];

        let mut resolution = ConflictResolution::new().with_conflicts(conflicts);
        assert!(!resolution.is_complete());

        resolution.resolve_path("a.txt", ConflictOutcome::Merge);
        assert!(!resolution.is_complete());

        resolution.resolve_path("b.txt", ConflictOutcome::RestoreCheckpoint);
        assert!(resolution.is_complete());
    }

    #[test]
    fn test_conflict_resolution_needs_user_input() {
        let conflicts = vec![FileConflict::new(
            "modified.txt".to_string(),
            Some(ContentHash::new(b"1")),
            Some(ContentHash::new(b"2")),
            ConflictType::ModifiedBoth,
        )];

        let resolution = ConflictResolution::new().with_conflicts(conflicts);
        assert!(resolution.needs_user_input());

        let delete_conflicts = vec![FileConflict::new(
            "deleted.txt".to_string(),
            None,
            Some(ContentHash::new(b"3")),
            ConflictType::DeletedInCurrent,
        )];

        let resolution2 = ConflictResolution::new().with_conflicts(delete_conflicts);
        assert!(!resolution2.needs_user_input());
    }

    #[test]
    fn test_conflict_ui_state_from_resolution() {
        let conflicts = vec![FileConflict::new(
            "test.txt".to_string(),
            Some(ContentHash::new(b"1")),
            Some(ContentHash::new(b"2")),
            ConflictType::ModifiedBoth,
        )];

        let resolution = ConflictResolution::new().with_conflicts(conflicts);
        let ui_state = ConflictUiState::from_resolution(&resolution);

        assert_eq!(ui_state.conflict_count, 1);
        assert!(ui_state.current_conflict.is_some());
        assert!(!ui_state.can_proceed);
    }

    #[test]
    fn test_conflict_ui_state_select_outcome() {
        let conflicts = vec![FileConflict::new(
            "test.txt".to_string(),
            Some(ContentHash::new(b"1")),
            Some(ContentHash::new(b"2")),
            ConflictType::ModifiedBoth,
        )];

        let resolution = ConflictResolution::new().with_conflicts(conflicts);
        let mut ui_state = ConflictUiState::from_resolution(&resolution);

        ui_state.select_outcome("test.txt", ConflictOutcome::Merge);

        assert_eq!(
            ui_state.resolutions.get("test.txt"),
            Some(&ConflictOutcome::Merge)
        );
    }

    #[test]
    fn test_conflict_resolution_set_cancelled() {
        let mut resolution = ConflictResolution::new();
        resolution.set_cancelled();

        assert!(resolution.was_cancelled);
    }

    #[test]
    fn test_conflict_resolution_set_safety_snapshot() {
        let mut resolution = ConflictResolution::new();
        let safety_id = CheckpointId::new();
        resolution.set_safety_snapshot(safety_id);

        assert_eq!(resolution.saved_safety_snapshot_id, Some(safety_id));
    }
}
