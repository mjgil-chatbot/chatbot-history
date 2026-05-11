//! Dirty workspace detection
//!
//! Compares current filesystem state with latest known snapshot.
//! Classifies changes as: AI-owned, user-owned, external, mixed, deleted, renamed, unknown.

use crate::ContentHash;
use serde::{Deserialize, Serialize};

/// Change ownership classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeOwnership {
    /// Changes made by the AI assistant
    AiOwned,
    /// Changes made by the user
    UserOwned,
    /// Changes made by external process
    External,
    /// Mixed ownership (hard to classify)
    Mixed,
    /// File was deleted
    Deleted,
    /// File was renamed
    Renamed,
    /// Cannot determine ownership
    Unknown,
}

impl ChangeOwnership {
    pub fn display_name(&self) -> &'static str {
        match self {
            ChangeOwnership::AiOwned => "AI-Owned",
            ChangeOwnership::UserOwned => "User-Owned",
            ChangeOwnership::External => "External",
            ChangeOwnership::Mixed => "Mixed",
            ChangeOwnership::Deleted => "Deleted",
            ChangeOwnership::Renamed => "Renamed",
            ChangeOwnership::Unknown => "Unknown",
        }
    }

    pub fn requires_restore_warning(&self) -> bool {
        matches!(
            self,
            ChangeOwnership::UserOwned | ChangeOwnership::External | ChangeOwnership::Mixed
        )
    }
}

/// A file change detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub ownership: ChangeOwnership,
    pub before_hash: Option<ContentHash>,
    pub after_hash: Option<ContentHash>,
    pub change_type: ChangeType,
}

impl FileChange {
    pub fn new(path: String, ownership: ChangeOwnership, change_type: ChangeType) -> Self {
        Self {
            path,
            ownership,
            before_hash: None,
            after_hash: None,
            change_type,
        }
    }

    pub fn with_before_hash(mut self, hash: ContentHash) -> Self {
        self.before_hash = Some(hash);
        self
    }

    pub fn with_after_hash(mut self, hash: ContentHash) -> Self {
        self.after_hash = Some(hash);
        self
    }

    pub fn is_deletive(&self) -> bool {
        matches!(self.change_type, ChangeType::Deleted)
    }

    pub fn is_additive(&self) -> bool {
        matches!(self.change_type, ChangeType::Added)
    }
}

/// Type of change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// Dirty workspace detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirtyWorkspace {
    pub is_dirty: bool,
    pub changes: Vec<FileChange>,
    pub total_files_changed: usize,
    pub ai_owned_count: usize,
    pub user_owned_count: usize,
    pub external_count: usize,
}

impl DirtyWorkspace {
    pub fn new() -> Self {
        Self {
            is_dirty: false,
            changes: Vec::new(),
            total_files_changed: 0,
            ai_owned_count: 0,
            user_owned_count: 0,
            external_count: 0,
        }
    }

    pub fn from_changes(changes: Vec<FileChange>) -> Self {
        let ai_owned_count = changes
            .iter()
            .filter(|c| c.ownership == ChangeOwnership::AiOwned)
            .count();
        let user_owned_count = changes
            .iter()
            .filter(|c| c.ownership == ChangeOwnership::UserOwned)
            .count();
        let external_count = changes
            .iter()
            .filter(|c| c.ownership == ChangeOwnership::External)
            .count();

        Self {
            is_dirty: !changes.is_empty(),
            total_files_changed: changes.len(),
            ai_owned_count,
            user_owned_count,
            external_count,
            changes,
        }
    }

    /// Check if any changes require user warning
    pub fn has_user_owned_changes(&self) -> bool {
        self.user_owned_count > 0
    }

    /// Check if any external changes
    pub fn has_external_changes(&self) -> bool {
        self.external_count > 0
    }

    /// Check if all changes are AI-owned (safe to restore)
    pub fn is_all_ai_owned(&self) -> bool {
        self.is_dirty && self.user_owned_count == 0 && self.external_count == 0
    }
}

impl Default for DirtyWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

/// Workspace change classifier
pub struct WorkspaceClassifier;

impl WorkspaceClassifier {
    /// Classify a change based on heuristics
    pub fn classify_change(
        path: &str,
        before_hash: Option<&ContentHash>,
        after_hash: Option<&ContentHash>,
    ) -> ChangeOwnership {
        // Deleted file
        if after_hash.is_none() {
            return ChangeOwnership::Deleted;
        }

        // New file - could be AI or user
        if before_hash.is_none() {
            // Heuristic: AI tends to create certain file types
            if Self::is_likely_ai_generated(path) {
                return ChangeOwnership::AiOwned;
            }
            return ChangeOwnership::Unknown;
        }

        // Modified file
        ChangeOwnership::Unknown
    }

    /// Check if path suggests AI generation
    fn is_likely_ai_generated(path: &str) -> bool {
        let ai_extensions = [".txt", ".md", ".rs", ".py", ".js", ".ts", ".go"];
        ai_extensions.iter().any(|ext| path.ends_with(ext))
    }
}

/// Restore policy based on dirty state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestorePolicy {
    /// Safe to restore without warning
    Safe,
    /// Warning required before restore
    Warning,
    /// Cannot restore without explicit override
    Blocked,
}

impl RestorePolicy {
    pub fn for_dirty_workspace(dirty: &DirtyWorkspace) -> Self {
        if !dirty.is_dirty {
            return RestorePolicy::Safe;
        }

        if dirty.has_external_changes() {
            return RestorePolicy::Blocked;
        }

        if dirty.has_user_owned_changes() {
            return RestorePolicy::Warning;
        }

        RestorePolicy::Safe
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_ownership_display() {
        assert_eq!(ChangeOwnership::AiOwned.display_name(), "AI-Owned");
        assert_eq!(ChangeOwnership::UserOwned.display_name(), "User-Owned");
        assert_eq!(ChangeOwnership::External.display_name(), "External");
        assert_eq!(ChangeOwnership::Unknown.display_name(), "Unknown");
    }

    #[test]
    fn test_change_ownership_requires_warning() {
        assert!(!ChangeOwnership::AiOwned.requires_restore_warning());
        assert!(ChangeOwnership::UserOwned.requires_restore_warning());
        assert!(ChangeOwnership::External.requires_restore_warning());
        assert!(ChangeOwnership::Mixed.requires_restore_warning());
    }

    #[test]
    fn test_file_change_new() {
        let change = FileChange::new(
            "test.rs".to_string(),
            ChangeOwnership::AiOwned,
            ChangeType::Modified,
        );

        assert_eq!(change.path, "test.rs");
        assert_eq!(change.ownership, ChangeOwnership::AiOwned);
        assert!(change.before_hash.is_none());
        assert!(change.after_hash.is_none());
    }

    #[test]
    fn test_file_change_with_hashes() {
        let before = ContentHash::new(b"before");
        let after = ContentHash::new(b"after");

        let change = FileChange::new(
            "test.rs".to_string(),
            ChangeOwnership::Unknown,
            ChangeType::Modified,
        )
        .with_before_hash(before)
        .with_after_hash(after);

        assert!(change.before_hash.is_some());
        assert!(change.after_hash.is_some());
    }

    #[test]
    fn test_file_change_is_deletive() {
        let change = FileChange::new(
            "test.rs".to_string(),
            ChangeOwnership::Deleted,
            ChangeType::Deleted,
        );
        assert!(change.is_deletive());
        assert!(!change.is_additive());
    }

    #[test]
    fn test_file_change_is_additive() {
        let change = FileChange::new(
            "new.rs".to_string(),
            ChangeOwnership::AiOwned,
            ChangeType::Added,
        );
        assert!(change.is_additive());
        assert!(!change.is_deletive());
    }

    #[test]
    fn test_dirty_workspace_empty() {
        let dirty = DirtyWorkspace::new();
        assert!(!dirty.is_dirty);
        assert_eq!(dirty.total_files_changed, 0);
    }

    #[test]
    fn test_dirty_workspace_from_changes() {
        let changes = vec![
            FileChange::new(
                "a.rs".to_string(),
                ChangeOwnership::AiOwned,
                ChangeType::Modified,
            ),
            FileChange::new(
                "b.rs".to_string(),
                ChangeOwnership::UserOwned,
                ChangeType::Modified,
            ),
            FileChange::new(
                "c.rs".to_string(),
                ChangeOwnership::External,
                ChangeType::Deleted,
            ),
        ];

        let dirty = DirtyWorkspace::from_changes(changes);

        assert!(dirty.is_dirty);
        assert_eq!(dirty.total_files_changed, 3);
        assert_eq!(dirty.ai_owned_count, 1);
        assert_eq!(dirty.user_owned_count, 1);
        assert_eq!(dirty.external_count, 1);
    }

    #[test]
    fn test_dirty_workspace_has_user_owned() {
        let changes = vec![FileChange::new(
            "user_file.rs".to_string(),
            ChangeOwnership::UserOwned,
            ChangeType::Modified,
        )];
        let dirty = DirtyWorkspace::from_changes(changes);
        assert!(dirty.has_user_owned_changes());
    }

    #[test]
    fn test_dirty_workspace_is_all_ai_owned() {
        let changes = vec![
            FileChange::new(
                "ai1.rs".to_string(),
                ChangeOwnership::AiOwned,
                ChangeType::Modified,
            ),
            FileChange::new(
                "ai2.rs".to_string(),
                ChangeOwnership::AiOwned,
                ChangeType::Added,
            ),
        ];
        let dirty = DirtyWorkspace::from_changes(changes);
        assert!(dirty.is_all_ai_owned());
    }

    #[test]
    fn test_dirty_workspace_not_all_ai_owned_with_user() {
        let changes = vec![
            FileChange::new(
                "ai.rs".to_string(),
                ChangeOwnership::AiOwned,
                ChangeType::Modified,
            ),
            FileChange::new(
                "user.rs".to_string(),
                ChangeOwnership::UserOwned,
                ChangeType::Modified,
            ),
        ];
        let dirty = DirtyWorkspace::from_changes(changes);
        assert!(!dirty.is_all_ai_owned());
    }

    #[test]
    fn test_restore_policy_safe() {
        let dirty = DirtyWorkspace::new();
        assert_eq!(
            RestorePolicy::for_dirty_workspace(&dirty),
            RestorePolicy::Safe
        );
    }

    #[test]
    fn test_restore_policy_warning() {
        let changes = vec![FileChange::new(
            "user.rs".to_string(),
            ChangeOwnership::UserOwned,
            ChangeType::Modified,
        )];
        let dirty = DirtyWorkspace::from_changes(changes);
        assert_eq!(
            RestorePolicy::for_dirty_workspace(&dirty),
            RestorePolicy::Warning
        );
    }

    #[test]
    fn test_restore_policy_blocked() {
        let changes = vec![FileChange::new(
            "external.txt".to_string(),
            ChangeOwnership::External,
            ChangeType::Modified,
        )];
        let dirty = DirtyWorkspace::from_changes(changes);
        assert_eq!(
            RestorePolicy::for_dirty_workspace(&dirty),
            RestorePolicy::Blocked
        );
    }

    #[test]
    fn test_workspace_classifier_deleted() {
        let before = ContentHash::new(b"old");
        let ownership = WorkspaceClassifier::classify_change("test.txt", Some(&before), None);
        assert_eq!(ownership, ChangeOwnership::Deleted);
    }

    #[test]
    fn test_workspace_classifier_ai_generated() {
        let before = ContentHash::new(b"old");
        let after = ContentHash::new(b"new");
        let ownership =
            WorkspaceClassifier::classify_change("test.rs", Some(&before), Some(&after));
        assert_eq!(ownership, ChangeOwnership::Unknown);
    }

    #[test]
    fn test_change_type_values() {
        assert_eq!(ChangeType::Added, ChangeType::Added);
        assert_eq!(ChangeType::Modified, ChangeType::Modified);
        assert_eq!(ChangeType::Deleted, ChangeType::Deleted);
        assert_eq!(ChangeType::Renamed, ChangeType::Renamed);
    }
}
