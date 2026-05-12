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
