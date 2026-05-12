//! Snapshot comparison utilities
//!
//! Provides three-way manifest comparison for change classification.

use super::manifest::Manifest;
use serde::{Deserialize, Serialize};

/// Result of three-way manifest comparison
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreeWayDiff {
    /// Files that are added in target (not in base or current)
    pub added: Vec<String>,
    /// Files that are deleted from target (in base, not in target)
    pub deleted: Vec<String>,
    /// Files that exist in both but content changed
    pub modified: Vec<String>,
    /// Files that are unchanged between current and target
    pub unchanged: Vec<String>,
    /// Files with same path but different blob in base vs target (potential rename source)
    pub renamed: Vec<String>,
    /// Files that conflict (different in current vs target, both changed from base)
    pub conflicts: Vec<String>,
}

impl ThreeWayDiff {
    /// Total number of changed files (excluding unchanged)
    pub fn total_changes(&self) -> usize {
        self.added.len() + self.deleted.len() + self.modified.len() + self.conflicts.len()
    }

    /// Check if there are any conflicts
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }
}

/// Compare base, current, and target manifests (three-way comparison)
pub fn three_way_compare(base: &Manifest, current: &Manifest, target: &Manifest) -> ThreeWayDiff {
    let mut added = Vec::new();
    let mut deleted = Vec::new();
    let mut modified = Vec::new();
    let mut unchanged = Vec::new();
    let renamed = Vec::new();
    let mut conflicts = Vec::new();

    // Get all paths from all three manifests
    let all_paths: std::collections::HashSet<_> = base
        .entries
        .keys()
        .chain(current.entries.keys())
        .chain(target.entries.keys())
        .collect();

    for path in all_paths {
        let path_str = path.as_str();
        let base_entry = base.entries.get(path_str);
        let current_entry = current.entries.get(path_str);
        let target_entry = target.entries.get(path_str);

        match (base_entry, current_entry, target_entry) {
            // File only in target - added
            (None, None, Some(_)) => {
                added.push(path_str.to_string());
            }
            // File only in current - could be deleted in target or added in current
            (None, Some(_), None) => {
                added.push(path_str.to_string());
            }
            // File in base and current but not target - deleted
            (Some(_), Some(_), None) => {
                deleted.push(path_str.to_string());
            }
            // File in base and target but not current - added (by others)
            (Some(_), None, Some(_)) => {
                added.push(path_str.to_string());
            }
            // File only in base - deleted
            (Some(_), None, None) => {
                deleted.push(path_str.to_string());
            }
            // File in all three - check relationship
            (Some(base_e), Some(current_e), Some(target_e)) => {
                if base_e.blob_hash == current_e.blob_hash && base_e.blob_hash == target_e.blob_hash
                {
                    // All three same - unchanged
                    unchanged.push(path_str.to_string());
                } else if current_e.blob_hash == target_e.blob_hash {
                    // Current matches target - unchanged (target wins)
                    unchanged.push(path_str.to_string());
                } else if base_e.blob_hash == current_e.blob_hash {
                    // Base matches current, target different - modified
                    modified.push(path_str.to_string());
                } else if base_e.blob_hash == target_e.blob_hash {
                    // Base matches target, current different - modified
                    modified.push(path_str.to_string());
                } else {
                    // All three different - conflict
                    conflicts.push(path_str.to_string());
                }
            }
            // File in current and target but not base
            (None, Some(current_e), Some(target_e)) => {
                if current_e.blob_hash == target_e.blob_hash {
                    unchanged.push(path_str.to_string());
                } else {
                    conflicts.push(path_str.to_string());
                }
            }
            // This case is actually impossible given how all_paths is constructed,
            // but Rust needs to know we've handled all patterns
            (_, _, _) => {}
        }
    }

    ThreeWayDiff {
        added,
        deleted,
        modified,
        unchanged,
        renamed,
        conflicts,
    }
}

/// Compare two manifests for equality
pub fn compare_manifests(a: &Manifest, b: &Manifest) -> super::manifest::ManifestDiff {
    a.diff(b)
}

/// Check if a manifest represents an empty workspace
pub fn is_empty_manifest(manifest: &Manifest) -> bool {
    manifest.is_empty()
}

/// Get statistics about a manifest
#[derive(Debug, Clone)]
pub struct ManifestStats {
    pub file_count: usize,
    pub total_size: u64,
    pub by_type: std::collections::HashMap<String, usize>,
}

pub fn manifest_stats(manifest: &Manifest) -> ManifestStats {
    let mut by_type = std::collections::HashMap::new();

    for entry in manifest.entries.values() {
        let file_type = entry
            .mime_type
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        *by_type.entry(file_type).or_insert(0) += 1;
    }

    ManifestStats {
        file_count: manifest.file_count,
        total_size: manifest.total_size,
        by_type,
    }
}

#[cfg(test)]
mod tests {
    use super::super::manifest::FileEntry;
    use super::*;

    #[test]
    fn test_manifest_stats() {
        let mut manifest = Manifest::new(ai_history_core::ManifestId::new());
        manifest.add_entry(FileEntry {
            path: "foo.rs".to_string(),
            blob_hash: ai_history_core::ContentHash::new(b"fn main() {}"),
            size: 12,
            mode: 0o644,
            mime_type: Some("text/x-rust".to_string()),
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        manifest.add_entry(FileEntry {
            path: "bar.json".to_string(),
            blob_hash: ai_history_core::ContentHash::new(b"{}"),
            size: 2,
            mode: 0o644,
            mime_type: Some("application/json".to_string()),
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });

        let stats = manifest_stats(&manifest);
        assert_eq!(stats.file_count, 2);
        assert_eq!(stats.total_size, 14);
        assert_eq!(stats.by_type.get("text/x-rust"), Some(&1));
        assert_eq!(stats.by_type.get("application/json"), Some(&1));
    }

    #[test]
    fn test_three_way_compare_added() {
        let mut base = Manifest::new(ai_history_core::ManifestId::new());
        base.add_entry(FileEntry {
            path: "existing.txt".to_string(),
            blob_hash: ai_history_core::ContentHash::new(b"base content"),
            size: 12,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        base.finalize();

        let mut current = Manifest::new(ai_history_core::ManifestId::new());
        current.add_entry(FileEntry {
            path: "existing.txt".to_string(),
            blob_hash: ai_history_core::ContentHash::new(b"base content"),
            size: 12,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        current.finalize();

        let mut target = Manifest::new(ai_history_core::ManifestId::new());
        target.add_entry(FileEntry {
            path: "existing.txt".to_string(),
            blob_hash: ai_history_core::ContentHash::new(b"base content"),
            size: 12,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        target.add_entry(FileEntry {
            path: "newfile.txt".to_string(),
            blob_hash: ai_history_core::ContentHash::new(b"new content"),
            size: 11,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        target.finalize();

        let diff = three_way_compare(&base, &current, &target);
        assert!(diff.added.contains(&"newfile.txt".to_string()));
        assert!(diff.unchanged.contains(&"existing.txt".to_string()));
        assert!(diff.modified.is_empty());
        assert!(diff.conflicts.is_empty());
    }

    #[test]
    fn test_three_way_compare_modified() {
        let mut base = Manifest::new(ai_history_core::ManifestId::new());
        base.add_entry(FileEntry {
            path: "file.txt".to_string(),
            blob_hash: ai_history_core::ContentHash::new(b"original"),
            size: 8,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        base.finalize();

        let mut current = Manifest::new(ai_history_core::ManifestId::new());
        current.add_entry(FileEntry {
            path: "file.txt".to_string(),
            blob_hash: ai_history_core::ContentHash::new(b"current version"),
            size: 14,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        current.finalize();

        let mut target = Manifest::new(ai_history_core::ManifestId::new());
        target.add_entry(FileEntry {
            path: "file.txt".to_string(),
            blob_hash: ai_history_core::ContentHash::new(b"target version"),
            size: 13,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        target.finalize();

        let diff = three_way_compare(&base, &current, &target);
        // Both changed from base to different content - conflict
        assert!(diff.conflicts.contains(&"file.txt".to_string()));
    }

    #[test]
    fn test_three_way_compare_deleted() {
        let mut base = Manifest::new(ai_history_core::ManifestId::new());
        base.add_entry(FileEntry {
            path: "deleted.txt".to_string(),
            blob_hash: ai_history_core::ContentHash::new(b"content"),
            size: 7,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        base.finalize();

        let mut current = Manifest::new(ai_history_core::ManifestId::new());
        current.add_entry(FileEntry {
            path: "deleted.txt".to_string(),
            blob_hash: ai_history_core::ContentHash::new(b"content"),
            size: 7,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        current.finalize();

        let mut target = Manifest::new(ai_history_core::ManifestId::new());
        // file deleted from target
        target.finalize();

        let diff = three_way_compare(&base, &current, &target);
        assert!(diff.deleted.contains(&"deleted.txt".to_string()));
    }

    #[test]
    fn test_three_way_compare_unchanged() {
        let hash = ai_history_core::ContentHash::new(b"same content");
        let mut base = Manifest::new(ai_history_core::ManifestId::new());
        base.add_entry(FileEntry {
            path: "file.txt".to_string(),
            blob_hash: hash,
            size: 12,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        base.finalize();

        let mut current = Manifest::new(ai_history_core::ManifestId::new());
        current.add_entry(FileEntry {
            path: "file.txt".to_string(),
            blob_hash: hash,
            size: 12,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        current.finalize();

        let mut target = Manifest::new(ai_history_core::ManifestId::new());
        target.add_entry(FileEntry {
            path: "file.txt".to_string(),
            blob_hash: hash,
            size: 12,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        target.finalize();

        let diff = three_way_compare(&base, &current, &target);
        assert!(diff.unchanged.contains(&"file.txt".to_string()));
        assert!(diff.added.is_empty());
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn test_three_way_diff_empty() {
        let diff = ThreeWayDiff {
            added: vec![],
            deleted: vec![],
            modified: vec![],
            unchanged: vec![],
            renamed: vec![],
            conflicts: vec![],
        };
        assert!(diff.added.is_empty());
        assert!(diff.conflicts.is_empty());
        assert_eq!(diff.total_changes(), 0);
    }
}
