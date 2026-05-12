//! Conflict resolution views

use serde::{Deserialize, Serialize};

/// Conflict item showing a single file conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictItem {
    pub path: String,
    pub conflict_type: ConflictType,
    pub checkpoint_version: String,
    pub current_version: String,
    pub selected_resolution: Option<ConflictResolutionUi>,
}

/// Type of conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    Modified,
    Deleted,
    Renamed,
    Created,
}

impl Default for ConflictType {
    fn default() -> Self {
        Self::Modified
    }
}

/// User-selectable conflict resolution options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolutionUi {
    KeepCurrent,
    RestoreCheckpoint,
    Merge,
    SaveAsBranch,
    Skip,
}

impl Default for ConflictResolutionUi {
    fn default() -> Self {
        Self::RestoreCheckpoint
    }
}

/// Conflict screen state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictScreenState {
    pub checkpoint_id: String,
    pub conflicts: Vec<ConflictItem>,
    pub show_diff: bool,
    pub is_executing: bool,
    pub auto_resolved_count: usize,
}

impl ConflictScreenState {
    pub fn new(checkpoint_id: String) -> Self {
        Self {
            checkpoint_id,
            conflicts: Vec::new(),
            show_diff: false,
            is_executing: false,
            auto_resolved_count: 0,
        }
    }

    pub fn add_conflict(&mut self, conflict: ConflictItem) {
        self.conflicts.push(conflict);
    }

    pub fn unresolved_count(&self) -> usize {
        self.conflicts
            .iter()
            .filter(|c| c.selected_resolution.is_none())
            .count()
    }

    pub fn all_resolved(&self) -> bool {
        self.unresolved_count() == 0
    }

    pub fn set_show_diff(&mut self, show: bool) {
        self.show_diff = show;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_screen_state_new() {
        let state = ConflictScreenState::new("checkpoint-123".to_string());
        assert_eq!(state.checkpoint_id, "checkpoint-123");
        assert!(!state.show_diff);
        assert!(!state.is_executing);
    }

    #[test]
    fn test_conflict_screen_state_unresolved() {
        let mut state = ConflictScreenState::new("cp-1".to_string());
        state.add_conflict(ConflictItem {
            path: "foo.txt".to_string(),
            conflict_type: ConflictType::Modified,
            checkpoint_version: "v1".to_string(),
            current_version: "v2".to_string(),
            selected_resolution: None,
        });
        state.add_conflict(ConflictItem {
            path: "bar.txt".to_string(),
            conflict_type: ConflictType::Deleted,
            checkpoint_version: "v1".to_string(),
            current_version: "".to_string(),
            selected_resolution: Some(ConflictResolutionUi::RestoreCheckpoint),
        });
        assert_eq!(state.unresolved_count(), 1);
        assert!(!state.all_resolved());
    }

    #[test]
    fn test_conflict_type_default() {
        assert_eq!(ConflictType::default(), ConflictType::Modified);
    }

    #[test]
    fn test_conflict_resolution_ui_default() {
        assert_eq!(
            ConflictResolutionUi::default(),
            ConflictResolutionUi::RestoreCheckpoint
        );
    }

    #[test]
    fn test_conflict_item_creation() {
        let item = ConflictItem {
            path: "src/main.rs".to_string(),
            conflict_type: ConflictType::Modified,
            checkpoint_version: "abc123".to_string(),
            current_version: "def456".to_string(),
            selected_resolution: None,
        };
        assert_eq!(item.path, "src/main.rs");
        assert_eq!(item.conflict_type, ConflictType::Modified);
        assert!(item.selected_resolution.is_none());
    }

    #[test]
    fn test_conflict_item_with_resolution() {
        let item = ConflictItem {
            path: "Cargo.toml".to_string(),
            conflict_type: ConflictType::Modified,
            checkpoint_version: "v1".to_string(),
            current_version: "v2".to_string(),
            selected_resolution: Some(ConflictResolutionUi::KeepCurrent),
        };
        assert_eq!(
            item.selected_resolution,
            Some(ConflictResolutionUi::KeepCurrent)
        );
    }

    #[test]
    fn test_conflict_type_all_variants() {
        assert_eq!(ConflictType::Modified, ConflictType::Modified);
        assert_eq!(ConflictType::Deleted, ConflictType::Deleted);
        assert_eq!(ConflictType::Renamed, ConflictType::Renamed);
        assert_eq!(ConflictType::Created, ConflictType::Created);
    }

    #[test]
    fn test_conflict_resolution_ui_all_variants() {
        assert_eq!(
            ConflictResolutionUi::KeepCurrent,
            ConflictResolutionUi::KeepCurrent
        );
        assert_eq!(
            ConflictResolutionUi::RestoreCheckpoint,
            ConflictResolutionUi::RestoreCheckpoint
        );
        assert_eq!(ConflictResolutionUi::Merge, ConflictResolutionUi::Merge);
        assert_eq!(
            ConflictResolutionUi::SaveAsBranch,
            ConflictResolutionUi::SaveAsBranch
        );
        assert_eq!(ConflictResolutionUi::Skip, ConflictResolutionUi::Skip);
    }

    #[test]
    fn test_conflict_screen_state_all_resolved() {
        let mut state = ConflictScreenState::new("cp-1".to_string());
        state.add_conflict(ConflictItem {
            path: "a.txt".to_string(),
            conflict_type: ConflictType::Modified,
            checkpoint_version: "v1".to_string(),
            current_version: "v2".to_string(),
            selected_resolution: Some(ConflictResolutionUi::KeepCurrent),
        });
        state.add_conflict(ConflictItem {
            path: "b.txt".to_string(),
            conflict_type: ConflictType::Deleted,
            checkpoint_version: "v1".to_string(),
            current_version: "".to_string(),
            selected_resolution: Some(ConflictResolutionUi::RestoreCheckpoint),
        });

        assert!(state.all_resolved());
        assert_eq!(state.unresolved_count(), 0);
    }

    #[test]
    fn test_conflict_screen_state_set_show_diff() {
        let mut state = ConflictScreenState::new("cp-1".to_string());
        assert!(!state.show_diff);

        state.set_show_diff(true);
        assert!(state.show_diff);

        state.set_show_diff(false);
        assert!(!state.show_diff);
    }

    #[test]
    fn test_conflict_screen_state_auto_resolved_count() {
        let mut state = ConflictScreenState::new("cp-1".to_string());
        assert_eq!(state.auto_resolved_count, 0);

        state.auto_resolved_count = 5;
        assert_eq!(state.auto_resolved_count, 5);
    }

    #[test]
    fn test_conflict_screen_state_multiple_conflicts() {
        let mut state = ConflictScreenState::new("cp-1".to_string());
        let paths = ["file1.txt", "file2.txt", "file3.txt"];

        for (i, path) in paths.iter().enumerate() {
            state.add_conflict(ConflictItem {
                path: path.to_string(),
                conflict_type: if i == 0 {
                    ConflictType::Modified
                } else {
                    ConflictType::Created
                },
                checkpoint_version: format!("v{}", i),
                current_version: format!("v{}", i + 1),
                selected_resolution: None,
            });
        }

        assert_eq!(state.conflicts.len(), 3);
        assert_eq!(state.unresolved_count(), 3);
        assert!(!state.all_resolved());
    }

    #[test]
    fn test_conflict_item_serialization() {
        let item = ConflictItem {
            path: "test.rs".to_string(),
            conflict_type: ConflictType::Renamed,
            checkpoint_version: "old".to_string(),
            current_version: "new".to_string(),
            selected_resolution: Some(ConflictResolutionUi::Merge),
        };

        let json = serde_json::to_string(&item).unwrap();
        let parsed: ConflictItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.path, "test.rs");
        assert_eq!(parsed.conflict_type, ConflictType::Renamed);
        assert_eq!(
            parsed.selected_resolution,
            Some(ConflictResolutionUi::Merge)
        );
    }

    #[test]
    fn test_conflict_screen_state_execution() {
        let mut state = ConflictScreenState::new("cp-1".to_string());
        assert!(!state.is_executing);

        state.is_executing = true;
        assert!(state.is_executing);

        state.is_executing = false;
        assert!(!state.is_executing);
    }

    #[test]
    fn test_conflict_type_default_is_modified() {
        assert_eq!(ConflictType::default(), ConflictType::Modified);
    }

    #[test]
    fn test_conflict_resolution_ui_default_is_restore_checkpoint() {
        assert_eq!(
            ConflictResolutionUi::default(),
            ConflictResolutionUi::RestoreCheckpoint
        );
    }
}
