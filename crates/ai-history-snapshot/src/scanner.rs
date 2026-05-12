//! Workspace scanner for creating snapshots
//!
//! Scans workspace and persists file contents as blobs.
//! Supports ignore rules, file size limits, hidden files, and binary policies.
//!
//! # Path Safety
//! The scanner validates all paths against the workspace root, rejecting:
//! - Traversal outside root (`..` components)
//! - Absolute path escapes
//! - Empty paths
//! - External symlinks (when follow_symlinks is disabled, symlinks pointing outside workspace are rejected before reading content)
//!
//! # Ignore Precedence
//! Files are excluded in order: ignore rules → binary policy → hidden files → size limit
//!
//! # Blob Persistence
//! Files are content-addressed via SHA-256. Blobs are written to the provided BlobStore
//! when using `scan_with_blob_store()`. The scanner deduplicates based on content hash.
//!
//! # Symlink Policy
//! - When `follow_symlinks=false` (default): External symlinks are rejected before reading content; internal symlinks are included with target recorded
//! - When `follow_symlinks=true`: Symlinks are followed and target content is included

use super::manifest::FileEntry;
use ai_history_core::ContentHash;
use ai_history_core::CoreResult;
use ai_history_core::IgnoreConfig;
use ai_history_store::traits::BlobStore;
use std::fs;
use std::io::Read;
use std::path::Path;
use walkdir::WalkDir;
/// Cross-platform file metadata
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub mode: u32,
    pub executable: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<String>,
}
#[cfg(unix)]
fn get_file_metadata(path: &Path) -> std::io::Result<FileMetadata> {
    use std::os::unix::fs::MetadataExt;
    // Use symlink_metadata to get info about the symlink itself, not the target
    let metadata = fs::symlink_metadata(path)?;
    let mode = metadata.mode();
    Ok(FileMetadata {
        size: metadata.len(),
        mode,
        executable: (mode & 0o111) != 0,
        is_symlink: metadata.file_type().is_symlink(),
        symlink_target: if metadata.file_type().is_symlink() {
            fs::read_link(path)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        } else {
            None
        },
    })
}
#[cfg(not(unix))]
fn get_file_metadata(path: &Path) -> std::io::Result<FileMetadata> {
    // Use symlink_metadata to detect symlinks without following them
    let metadata = fs::symlink_metadata(path)?;
    let is_symlink = metadata.file_type().is_symlink();
    Ok(FileMetadata {
        size: metadata.len(),
        mode: 0o644,       // Default mode for non-Unix
        executable: false, // Cannot determine on non-Unix without platform-specific code
        is_symlink,
        symlink_target: if is_symlink {
            fs::read_link(path)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        } else {
            None
        },
    })
}
/// Scanner configuration combining ignore rules and scanning behavior
#[derive(Debug, Clone)]
pub struct ScannerConfig {
    /// Include hidden files (starting with .)
    pub include_hidden: bool,
    /// Maximum file size to include
    pub max_file_size: u64,
    /// Follow symbolic links
    pub follow_symlinks: bool,
    /// Ignore rules configuration
    pub ignore_config: IgnoreConfig,
}
impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            include_hidden: false,
            max_file_size: 100 * 1024 * 1024, // 100MB
            follow_symlinks: false,
            ignore_config: IgnoreConfig::default(),
        }
    }
}
/// Scanner for workspace files
pub struct Scanner {
    config: ScannerConfig,
}
impl Scanner {
    pub fn new(config: ScannerConfig) -> Self {
        Self { config }
    }}
impl Scanner {
    fn process_file<B: BlobStore>(
        &self,
        path: &Path,
        root: &Path,
        blob_store: &mut Option<B>,
    ) -> CoreResult<FileEntry> {
        let relative_path = path.strip_prefix(root).map_err(|_| {
            ai_history_core::CoreError::PathError("Failed to get relative path".to_string())
        })?;
        let path_str = relative_path.to_string_lossy().replace('\\', "/");

        // Use cross-platform metadata retrieval
        let file_meta = get_file_metadata(path)?;
        let size = file_meta.size;
        let mode = file_meta.mode;
        let executable = file_meta.executable;
        let symlink_target = file_meta.symlink_target;

        // Read file content
        let mut file = fs::File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let blob_hash = ContentHash::new(&buffer);

        // Persist to blob store if provided
        if let Some(bs) = blob_store {
            bs.write_blob(blob_hash.as_bytes(), &buffer)?;
        }

        let mime_type = guess_mime_type(path);

        // Get modified time (platform-independent)
        let modified = get_modified_time(path);

        Ok(FileEntry {
            path: path_str,
            blob_hash,
            size,
            mode,
            mime_type,
            executable,
            symlink_target,
            modified,
            ownership: None,
        })
    }}
fn guess_mime_type(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?;
    match extension {
        "rs" => Some("text/x-rust".to_string()),
        "js" => Some("text/javascript".to_string()),
        "ts" => Some("text/typescript".to_string()),
        "json" => Some("application/json".to_string()),
        "txt" => Some("text/plain".to_string()),
        "html" => Some("text/html".to_string()),
        "css" => Some("text/css".to_string()),
        "md" => Some("text/markdown".to_string()),
        "png" => Some("image/png".to_string()),
        "jpg" | "jpeg" => Some("image/jpeg".to_string()),
        "gif" => Some("image/gif".to_string()),
        "svg" => Some("image/svg+xml".to_string()),
        _ => None,
    }
}
/// Get file modification time in RFC3339 format (cross-platform)
fn get_modified_time(path: &Path) -> Option<String> {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
}
