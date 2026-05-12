//! Workspace snapshot manifest
//!
//! A manifest maps file paths to blob hashes with metadata.
//! Manifests are content-addressed: the manifest hash is derived from
//! its canonical serialized form, enabling deduplication.

use ai_history_core::{ContentHash, ManifestId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A manifest describes the contents of a workspace snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Unique identifier for this manifest
    pub id: ManifestId,
    /// Content-addressed hash of this manifest (derived from canonical serialization)
    pub content_hash: ContentHash,
    /// Optional reference to parent manifest (for snapshot chains)
    pub parent_id: Option<ManifestId>,
    /// All files in this snapshot
    pub entries: HashMap<String, FileEntry>,
    /// Total size of all files
    pub total_size: u64,
    /// Number of files
    pub file_count: usize,
}

/// File entry in a manifest
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub blob_hash: ContentHash,
    pub size: u64,
    pub mode: u32,
    pub mime_type: Option<String>,
    pub executable: bool,
    pub symlink_target: Option<String>,
    pub modified: Option<String>,
    /// Ownership provenance - who made the last change
    pub ownership: Option<FileOwnership>,
}

/// Ownership provenance for a file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOwnership {
    /// File was modified by the AI assistant
    AiOwned,
    /// File was modified by the user
    UserOwned,
    /// File was modified externally (outside AI history)
    External,
}

impl FileOwnership {
    pub fn is_ai_owned(&self) -> bool {
        matches!(self, FileOwnership::AiOwned)
    }

    pub fn is_user_owned(&self) -> bool {
        matches!(self, FileOwnership::UserOwned)
    }

    pub fn is_external(&self) -> bool {
        matches!(self, FileOwnership::External)
    }
}

/// Metadata for computing manifest content hash
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestContent {
    entries: Vec<FileEntry>,
    total_size: u64,
    file_count: usize,
}

impl Manifest {
    /// Create a new manifest with a generated ID
    pub fn new(id: ManifestId) -> Self {
        Self {
            id,
            content_hash: ContentHash::default(), // Will be computed when finalizing
            parent_id: None,
            entries: HashMap::new(),
            total_size: 0,
            file_count: 0,
        }
    }

    /// Create a manifest with a parent reference (for snapshot chains)
    pub fn with_parent(id: ManifestId, parent: ManifestId) -> Self {
        Self {
            id,
            content_hash: ContentHash::default(),
            parent_id: Some(parent),
            entries: HashMap::new(),
            total_size: 0,
            file_count: 0,
        }
    }

    /// Add an entry and recompute content hash if needed
    pub fn add_entry(&mut self, entry: FileEntry) {
        self.total_size += entry.size;
        self.file_count += 1;
        self.entries.insert(entry.path.clone(), entry);
    }

    /// Remove an entry and recompute content hash if needed
    pub fn remove_entry(&mut self, path: &str) -> Option<FileEntry> {
        if let Some(entry) = self.entries.remove(path) {
            self.total_size = self.total_size.saturating_sub(entry.size);
            self.file_count = self.file_count.saturating_sub(1);
            Some(entry)
        } else {
            None
        }
    }

    /// Get the content-addressed hash of this manifest
    pub fn content_hash(&self) -> ContentHash {
        self.content_hash
    }

    /// Finalize the manifest by computing its content hash
    /// Call this after all entries have been added
    pub fn finalize(&mut self) {
        let content = self.to_content();
        self.content_hash = ContentHash::new(&serde_json::to_vec(&content).unwrap());
    }

    /// Convert to content struct for hashing (canonical form)
    fn to_content(&self) -> ManifestContent {
        // Sort entries by path for canonical serialization
        let mut entries: Vec<_> = self.entries.values().cloned().collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        ManifestContent {
            entries,
            total_size: self.total_size,
            file_count: self.file_count,
        }
    }

    /// Check if another manifest has identical content (same files, same metadata)
    pub fn has_same_content(&self, other: &Manifest) -> bool {
        self.content_hash == other.content_hash && self.content_hash != ContentHash::default()
    }

    /// Get the parent manifest ID if any
    pub fn parent(&self) -> Option<ManifestId> {
        self.parent_id
    }

    pub fn get_entry(&self, path: &str) -> Option<&FileEntry> {
        self.entries.get(path)
    }

    pub fn paths(&self) -> Vec<&String> {
        self.entries.keys().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Check if this manifest is byte-identical to another
    pub fn is_identical(&self, other: &Manifest) -> bool {
        self.total_size == other.total_size
            && self.file_count == other.file_count
            && self.entries == other.entries
    }

    /// Compare two manifests and return changed paths
    pub fn diff(&self, other: &Manifest) -> ManifestDiff {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut modified = Vec::new();

        for (path, entry) in &self.entries {
            match other.entries.get(path) {
                Some(other_entry) => {
                    if entry.blob_hash != other_entry.blob_hash {
                        modified.push(path.clone());
                    }
                }
                None => {
                    removed.push(path.clone());
                }
            }
        }

        for path in other.entries.keys() {
            if !self.entries.contains_key(path) {
                added.push(path.clone());
            }
        }

        ManifestDiff {
            added,
            removed,
            modified,
        }
    }
}

/// Result of comparing two manifests
#[derive(Debug, Clone, PartialEq)]
pub struct ManifestDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub modified: Vec<String>,
}

impl ManifestDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }

    pub fn total_changes(&self) -> usize {
        self.added.len() + self.removed.len() + self.modified.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_add_entry() {
        let mut manifest = Manifest::new(ManifestId::new());
        manifest.add_entry(FileEntry {
            path: "foo.txt".to_string(),
            blob_hash: ContentHash::new(b"hello"),
            size: 5,
            mode: 0o644,
            mime_type: Some("text/plain".to_string()),
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        assert_eq!(manifest.file_count, 1);
        assert_eq!(manifest.total_size, 5);
    }

    #[test]
    fn test_manifest_diff() {
        let mut manifest1 = Manifest::new(ManifestId::new());
        manifest1.add_entry(FileEntry {
            path: "foo.txt".to_string(),
            blob_hash: ContentHash::new(b"hello"),
            size: 5,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });

        let mut manifest2 = Manifest::new(ManifestId::new());
        manifest2.add_entry(FileEntry {
            path: "foo.txt".to_string(),
            blob_hash: ContentHash::new(b"hello world"),
            size: 11,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        manifest2.add_entry(FileEntry {
            path: "bar.txt".to_string(),
            blob_hash: ContentHash::new(b"new"),
            size: 3,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });

        let diff = manifest1.diff(&manifest2);
        assert!(diff.added.contains(&"bar.txt".to_string()));
        assert!(diff.modified.contains(&"foo.txt".to_string()));
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn test_manifest_is_identical() {
        let id = ManifestId::new();
        let mut manifest1 = Manifest::new(id);
        manifest1.add_entry(FileEntry {
            path: "foo.txt".to_string(),
            blob_hash: ContentHash::new(b"hello"),
            size: 5,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });

        let mut manifest2 = Manifest::new(id);
        manifest2.add_entry(FileEntry {
            path: "foo.txt".to_string(),
            blob_hash: ContentHash::new(b"hello"),
            size: 5,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });

        assert!(manifest1.is_identical(&manifest2));
    }

    #[test]
    fn test_manifest_content_hash() {
        let mut manifest = Manifest::new(ManifestId::new());
        manifest.add_entry(FileEntry {
            path: "foo.txt".to_string(),
            blob_hash: ContentHash::new(b"hello"),
            size: 5,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });

        // Content hash should be default before finalization
        assert_eq!(manifest.content_hash, ContentHash::default());

        // Finalize and check hash is computed
        manifest.finalize();
        assert_ne!(manifest.content_hash, ContentHash::default());
    }

    #[test]
    fn test_manifest_deduplication() {
        // Two manifests with identical content should have same hash
        let mut manifest1 = Manifest::new(ManifestId::new());
        manifest1.add_entry(FileEntry {
            path: "foo.txt".to_string(),
            blob_hash: ContentHash::new(b"hello"),
            size: 5,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        manifest1.finalize();

        let mut manifest2 = Manifest::new(ManifestId::new());
        manifest2.add_entry(FileEntry {
            path: "foo.txt".to_string(),
            blob_hash: ContentHash::new(b"hello"),
            size: 5,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        manifest2.finalize();

        // Same content should give same hash
        assert_eq!(manifest1.content_hash, manifest2.content_hash);
        assert!(manifest1.has_same_content(&manifest2));
    }

    #[test]
    fn test_manifest_parent_linkage() {
        let parent_id = ManifestId::new();
        let child = Manifest::with_parent(ManifestId::new(), parent_id);

        assert_eq!(child.parent(), Some(parent_id));
        assert_eq!(child.id != parent_id, true); // IDs are different
    }

    #[test]
    fn test_manifest_different_content_different_hash() {
        let mut manifest1 = Manifest::new(ManifestId::new());
        manifest1.add_entry(FileEntry {
            path: "foo.txt".to_string(),
            blob_hash: ContentHash::new(b"hello"),
            size: 5,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        manifest1.finalize();

        let mut manifest2 = Manifest::new(ManifestId::new());
        manifest2.add_entry(FileEntry {
            path: "foo.txt".to_string(),
            blob_hash: ContentHash::new(b"different"),
            size: 9,
            mode: 0o644,
            mime_type: None,
            executable: false,
            symlink_target: None,
            modified: None,
            ownership: None,
        });
        manifest2.finalize();

        // Different content should give different hash
        assert_ne!(manifest1.content_hash, manifest2.content_hash);
        assert!(!manifest1.has_same_content(&manifest2));
    }
}
