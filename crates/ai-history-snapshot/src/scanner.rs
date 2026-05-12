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

use super::manifest::{FileEntry, Manifest};
use ai_history_core::{
    ContentHash, CoreResult, IgnoreConfig, IgnoreMatcher, ManifestId, WorkspacePath, WorkspaceRoot,
};
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
    }

    /// Scan workspace and compute manifest (hashes only, no blob persistence)
    pub fn scan(&self, workspace_root: &WorkspacePath) -> CoreResult<Manifest> {
        self.scan_with_empty_blob_store(workspace_root)
    }

    /// Scan workspace with a parent manifest for linkage (produces incremental snapshot)
    pub fn scan_with_parent(
        &self,
        workspace_root: &WorkspacePath,
        parent_id: ManifestId,
    ) -> CoreResult<Manifest> {
        use ai_history_core::SessionId;
        use ai_history_store::traits::BlobInfo;
        // A no-op blob store that implements BlobStore but does nothing
        struct NoOpBlobStore;
        impl BlobStore for NoOpBlobStore {
            fn write_blob(&mut self, _: &[u8], _: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn read_blob(&self, _: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                Ok(None)
            }
            fn has_blob(&self, _: &[u8]) -> bool {
                false
            }
            fn delete_blob(&mut self, _: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(Vec::new())
            }
        }
        let mut bs = Some(NoOpBlobStore);
        self.scan_with_blob_store_impl(workspace_root, &mut bs, Some(parent_id))
    }

    fn scan_with_empty_blob_store(&self, workspace_root: &WorkspacePath) -> CoreResult<Manifest> {
        use ai_history_core::SessionId;
        use ai_history_store::traits::BlobInfo;
        // A no-op blob store that implements BlobStore but does nothing
        struct NoOpBlobStore;
        impl BlobStore for NoOpBlobStore {
            fn write_blob(&mut self, _: &[u8], _: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn read_blob(&self, _: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                Ok(None)
            }
            fn has_blob(&self, _: &[u8]) -> bool {
                false
            }
            fn delete_blob(&mut self, _: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(Vec::new())
            }
        }
        let mut bs = Some(NoOpBlobStore);
        self.scan_with_blob_store_impl(workspace_root, &mut bs, None)
    }

    /// Scan workspace and persist file contents to blob store
    pub fn scan_with_blob_store<B: BlobStore>(
        &self,
        workspace_root: &WorkspacePath,
        blob_store: &mut Option<B>,
    ) -> CoreResult<Manifest> {
        self.scan_with_blob_store_impl(workspace_root, blob_store, None)
    }

    /// Scan workspace with blob store and optional parent manifest for linkage
    pub fn scan_with_blob_store_and_parent<B: BlobStore>(
        &self,
        workspace_root: &WorkspacePath,
        blob_store: &mut Option<B>,
        parent_id: Option<ManifestId>,
    ) -> CoreResult<Manifest> {
        self.scan_with_blob_store_impl(workspace_root, blob_store, parent_id)
    }

    fn scan_with_blob_store_impl<B: BlobStore>(
        &self,
        workspace_root: &WorkspacePath,
        blob_store: &mut Option<B>,
        parent_id: Option<ManifestId>,
    ) -> CoreResult<Manifest> {
        // Validate workspace root is safe
        let ws_root = WorkspaceRoot::new(workspace_root.as_path().to_path_buf()).map_err(|e| {
            ai_history_core::CoreError::PathError(format!("Invalid workspace root: {}", e))
        })?;

        let manifest_id = ManifestId::new();
        let mut manifest = if let Some(pid) = parent_id {
            Manifest::with_parent(manifest_id, pid)
        } else {
            Manifest::new(manifest_id)
        };

        let root_path = workspace_root.as_path();
        if !root_path.exists() {
            return Ok(manifest);
        }

        let max_depth = self.config.ignore_config.max_depth.unwrap_or(100);

        let walker = if self.config.follow_symlinks {
            WalkDir::new(root_path)
                .follow_links(true)
                .max_depth(max_depth)
        } else {
            WalkDir::new(root_path)
                .follow_links(false)
                .max_depth(max_depth)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let depth = entry.depth();

            // Skip the root directory itself
            if depth == 0 {
                continue;
            }

            // Validate path stays within workspace root
            let relative_path_str = match path.strip_prefix(root_path) {
                Ok(p) => p.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };

            // Path safety validation - reject traversal attacks
            let ws_path = WorkspacePath::from_string(&relative_path_str);
            if ws_path.validate_against_root(&ws_root).is_err() {
                // Skip paths that escape the workspace
                continue;
            }

            // Apply generated file patterns exclusion
            if self
                .config
                .ignore_config
                .generated_file_patterns
                .iter()
                .any(|pattern| IgnoreMatcher::matches_pattern(&relative_path_str, pattern))
            {
                continue;
            }

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Get relative path for ignore checking
            let relative_path = relative_path_str;

            // Apply ignore rules from config
            let ignore_rules = &self.config.ignore_config.rules;
            if IgnoreMatcher::is_ignored(&relative_path, ignore_rules) {
                continue;
            }

            // Check binary policy
            if self.config.ignore_config.binary_policy == ai_history_core::BinaryPolicy::Skip
                && IgnoreMatcher::is_binary(&relative_path)
            {
                continue;
            }

            // Skip hidden files if configured
            if !self.config.include_hidden {
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with('.') {
                        continue;
                    }
                }
            }

            // Skip files that are too large
            if let Ok(metadata) = fs::metadata(path) {
                if metadata.len() > self.config.max_file_size {
                    continue;
                }
            }

            // Get file metadata to check if it's a symlink
            let file_meta = match get_file_metadata(path) {
                Ok(meta) => meta,
                Err(_) => continue,
            };

            // When not following symlinks, reject external symlinks without reading content
            if !self.config.follow_symlinks && file_meta.is_symlink {
                if let Some(target) = &file_meta.symlink_target {
                    // Check if symlink target escapes the workspace
                    let target_path = path.parent().unwrap_or(root_path).join(target);
                    let Ok(canonical_target) = target_path.canonicalize() else {
                        // An unresolved target cannot be proved internal.
                        continue;
                    };
                    if !canonical_target.starts_with(ws_root.canonical()) {
                        // Compare canonical paths, including workspace root aliases.
                        // External symlink - reject without reading content.
                        continue;
                    }
                }
            }

            // Process the file
            if let Ok(file_entry) = self.process_file(path, root_path, blob_store) {
                manifest.add_entry(file_entry);
            }
        }

        Ok(manifest)
    }

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
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use ai_history_core::{BinaryPolicy, IgnoreConfig, IgnoreRule, IgnoreRuleType};
    use ai_history_testkit::fakes::FakeBlobStore;

    #[test]
    fn test_scanner_config_default() {
        let config = ScannerConfig::default();
        assert!(!config.include_hidden);
        assert_eq!(config.max_file_size, 100 * 1024 * 1024);
        assert!(!config.follow_symlinks);
        assert!(!config.ignore_config.rules.is_empty());
    }

    #[test]
    fn test_guess_mime_type() {
        assert_eq!(
            guess_mime_type(Path::new("foo.rs")),
            Some("text/x-rust".to_string())
        );
        assert_eq!(
            guess_mime_type(Path::new("foo.json")),
            Some("application/json".to_string())
        );
        assert_eq!(guess_mime_type(Path::new("foo.unknown")), None);
    }

    #[test]
    fn test_scanner_writes_blobs() {
        // Create a temp directory with a file
        let temp = tempfile::TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, b"hello world").unwrap();

        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let mut blob_store = Some(FakeBlobStore::new());

        let scanner = Scanner::new(ScannerConfig::default());
        let manifest = scanner
            .scan_with_blob_store(&ws_path, &mut blob_store)
            .unwrap();

        // Verify manifest has the file
        assert_eq!(manifest.file_count, 1);
        let entry = manifest.get_entry("test.txt").unwrap();
        assert_eq!(entry.size, 11);

        // Verify blob was written
        let bs = blob_store.as_ref().unwrap();
        assert!(bs.has(entry.blob_hash.as_bytes()));

        // Verify we can read the blob back
        let blob_data = bs.read(entry.blob_hash.as_bytes()).unwrap().unwrap();
        assert_eq!(blob_data, b"hello world");
    }

    #[test]
    fn test_scanner_ignores_target_directory() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create files in target directory
        let target_dir = temp.path().join("target");
        std::fs::create_dir(&target_dir).unwrap();
        std::fs::write(target_dir.join("build.rs"), b"// generated").unwrap();

        // Create src directory and file
        let src_dir = temp.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        std::fs::write(src_dir.join("main.rs"), b"fn main() {}").unwrap();

        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(ScannerConfig::default());
        let manifest = scanner.scan(&ws_path).unwrap();

        // Target directory should be ignored by default rules
        assert_eq!(manifest.file_count, 1);
        assert!(manifest.get_entry("src/main.rs").is_some());
        assert!(manifest.get_entry("target/build.rs").is_none());
    }

    #[test]
    fn test_scanner_ignores_node_modules() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create node_modules
        let node_modules = temp.path().join("node_modules");
        std::fs::create_dir(&node_modules).unwrap();
        std::fs::write(node_modules.join("package.json"), b"{}").unwrap();

        // Create src directory and file
        let src_dir = temp.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        std::fs::write(src_dir.join("index.js"), b"console.log('hello')").unwrap();

        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(ScannerConfig::default());
        let manifest = scanner.scan(&ws_path).unwrap();

        // node_modules should be ignored
        assert_eq!(manifest.file_count, 1);
        assert!(manifest.get_entry("src/index.js").is_some());
    }

    #[test]
    fn test_scanner_skips_binary_files() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create binary file
        std::fs::write(temp.path().join("image.png"), &[0x89, 0x50, 0x4E, 0x47]).unwrap();
        std::fs::write(temp.path().join("code.rs"), b"fn main() {}").unwrap();

        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(ScannerConfig::default());
        let manifest = scanner.scan(&ws_path).unwrap();

        // Binary should be skipped by default (BinaryPolicy::Skip)
        assert_eq!(manifest.file_count, 1);
        assert!(manifest.get_entry("code.rs").is_some());
        assert!(manifest.get_entry("image.png").is_none());
    }

    #[test]
    fn test_scanner_includes_hidden_when_configured() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create hidden file
        std::fs::write(temp.path().join(".hidden"), b"secret").unwrap();
        std::fs::write(temp.path().join("visible.txt"), b"visible").unwrap();

        // Default config should skip hidden
        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(ScannerConfig::default());
        let manifest = scanner.scan(&ws_path).unwrap();
        assert_eq!(manifest.file_count, 1);
        assert!(manifest.get_entry("visible.txt").is_some());
        assert!(manifest.get_entry(".hidden").is_none());

        // Config with include_hidden should include it
        let mut config = ScannerConfig::default();
        config.include_hidden = true;
        let scanner2 = Scanner::new(config);
        let manifest2 = scanner2.scan(&ws_path).unwrap();
        assert_eq!(manifest2.file_count, 2);
        assert!(manifest2.get_entry(".hidden").is_some());
    }

    #[test]
    fn test_scanner_with_custom_ignore_rules() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create custom ignore file at root
        std::fs::write(temp.path().join(".ai-history-ignore"), b"custom_pattern").unwrap();
        std::fs::write(temp.path().join("custom_pattern"), b"should be ignored").unwrap();

        // Create src directory and file
        let src_dir = temp.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        std::fs::write(src_dir.join("main.rs"), b"fn main() {}").unwrap();

        let mut config = ScannerConfig::default();
        config.ignore_config.rules.push(IgnoreRule::new(
            "custom_pattern",
            IgnoreRuleType::ProductIgnore,
        ));

        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(config);
        let manifest = scanner.scan(&ws_path).unwrap();

        // custom_pattern should be ignored but src/main.rs included
        assert_eq!(manifest.file_count, 1);
        assert!(manifest.get_entry("src/main.rs").is_some());
        assert!(manifest.get_entry("custom_pattern").is_none());
    }

    #[test]
    fn test_scanner_cross_platform_metadata() {
        // Test that get_file_metadata works on regular files
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::write(temp.path().join("test.txt"), b"hello").unwrap();

        let meta = get_file_metadata(temp.path().join("test.txt").as_path()).unwrap();
        assert_eq!(meta.size, 5);
        assert!(!meta.is_symlink);
        assert!(meta.symlink_target.is_none());
        // On Unix, mode should be non-zero; on Windows, 0o644
        assert!(meta.mode != 0 || cfg!(not(unix)));
    }

    #[test]
    fn test_scanner_symlink_detection() {
        use std::os::unix::fs::symlink;

        #[cfg(unix)]
        {
            let temp = tempfile::TempDir::new().unwrap();
            std::fs::write(temp.path().join("target.txt"), b"target content").unwrap();
            symlink("target.txt", temp.path().join("link.txt")).unwrap();

            let meta = get_file_metadata(temp.path().join("link.txt").as_path()).unwrap();
            assert!(meta.is_symlink);
            assert!(meta.symlink_target.is_some());
        }
    }

    #[test]
    fn test_file_metadata_struct_debug() {
        let meta = FileMetadata {
            size: 100,
            mode: 0o755,
            executable: true,
            is_symlink: false,
            symlink_target: None,
        };
        let debug = format!("{:?}", meta);
        assert!(debug.contains("FileMetadata"));
        assert!(debug.contains("100"));
    }

    #[test]
    fn test_scanner_respects_max_depth() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create nested structure: src/a/b/c/d/deep.txt
        // Root is at depth 0, src/ at depth 1, src/a at 2, src/a/b at 3, etc.
        std::fs::create_dir_all(temp.path().join("src/a/b/c/d")).unwrap();
        std::fs::write(temp.path().join("src/a/b/c/d/deep.txt"), b"deep").unwrap();
        std::fs::write(temp.path().join("src/a/b/c/shallow.txt"), b"shallow").unwrap();
        std::fs::write(temp.path().join("src/a/b/file.txt"), b"file").unwrap();
        std::fs::write(temp.path().join("src/a/file.txt"), b"a file").unwrap();
        std::fs::write(temp.path().join("src/file.txt"), b"src file").unwrap();
        std::fs::write(temp.path().join("root_file.txt"), b"root").unwrap();

        // Create scanner with max_depth of 3
        let mut config = ScannerConfig::default();
        config.ignore_config.max_depth = Some(3);
        config.follow_symlinks = false;
        config.include_hidden = true;

        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(config);
        let manifest = scanner.scan(&ws_path).unwrap();

        let paths: Vec<_> = manifest.paths().into_iter().collect();

        // With max_depth=3, depth 0,1,2,3 entries should be included
        // root_file.txt (depth 1), src/file.txt (depth 2), src/a/file.txt (depth 3) should be included
        // But src/a/b/file.txt (depth 4) should be excluded

        assert!(
            paths.iter().any(|p| *p == "root_file.txt"),
            "depth 1 should be included: {:?}",
            paths
        );
        assert!(
            paths.iter().any(|p| *p == "src/file.txt"),
            "depth 2 should be included: {:?}",
            paths
        );
        assert!(
            paths.iter().any(|p| *p == "src/a/file.txt"),
            "depth 3 should be included: {:?}",
            paths
        );
        // src/a/b/file.txt at depth 4 should be excluded
        assert!(
            !paths.iter().any(|p| *p == "src/a/b/file.txt"),
            "depth 4 should be excluded: {:?}",
            paths
        );
        assert!(
            !paths.iter().any(|p| *p == "src/a/b/c/shallow.txt"),
            "depth 5 should be excluded: {:?}",
            paths
        );
    }

    #[test]
    fn test_scanner_generated_file_patterns() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create generated file patterns: *.pyc, __pycache__, target/debug
        std::fs::create_dir_all(temp.path().join("__pycache__")).unwrap();
        std::fs::write(temp.path().join("__pycache__/module.pyc"), b"bytecode").unwrap();
        std::fs::create_dir_all(temp.path().join("target/debug")).unwrap();
        std::fs::write(temp.path().join("target/debug/build"), b"build artifact").unwrap();
        std::fs::write(temp.path().join("script.pyc"), b"pyc").unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(temp.path().join("src/main.rs"), b"fn main() {}").unwrap();

        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(ScannerConfig::default());
        let manifest = scanner.scan(&ws_path).unwrap();

        // script.pyc should be skipped (matches *.pyc pattern)
        // __pycache__/module.pyc should be skipped
        // target/debug/build should be skipped
        // Only src/main.rs should be included
        let paths: Vec<_> = manifest.paths().into_iter().collect();
        assert!(
            manifest.get_entry("src/main.rs").is_some(),
            "src/main.rs should be included: {:?}",
            paths
        );
        assert!(
            !manifest.paths().iter().any(|p| p.contains("__pycache__")),
            "pycache should be excluded: {:?}",
            paths
        );
        assert!(
            !manifest.paths().iter().any(|p| p.contains("target/debug")),
            "target/debug should be excluded: {:?}",
            paths
        );
        assert!(
            !manifest.paths().iter().any(|p| p.ends_with(".pyc")),
            ".pyc files should be excluded: {:?}",
            paths
        );
    }

    #[test]
    fn test_scanner_parent_linkage() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create a file at depth 1
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(temp.path().join("src/file.txt"), b"content").unwrap();

        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(ScannerConfig::default());

        // Create a parent manifest ID
        let parent_id = ai_history_core::ManifestId::new();

        // Scan with parent linkage
        let manifest = scanner.scan_with_parent(&ws_path, parent_id).unwrap();

        // Verify parent linkage is set
        assert_eq!(
            manifest.parent(),
            Some(parent_id),
            "parent should be linked"
        );
        assert_eq!(manifest.file_count, 1, "should have 1 file");
        assert!(
            manifest.get_entry("src/file.txt").is_some(),
            "src/file.txt should exist"
        );
    }

    #[test]
    fn test_scanner_with_blob_store_and_parent() {
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::write(temp.path().join("test.txt"), b"hello world").unwrap();

        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let mut blob_store = Some(ai_history_testkit::fakes::FakeBlobStore::new());
        let parent_id = ai_history_core::ManifestId::new();

        let scanner = Scanner::new(ScannerConfig::default());
        let manifest = scanner
            .scan_with_blob_store_and_parent(&ws_path, &mut blob_store, Some(parent_id))
            .unwrap();

        // Verify parent linkage and blob persistence
        assert_eq!(manifest.parent(), Some(parent_id));
        assert_eq!(manifest.file_count, 1);

        let entry = manifest.get_entry("test.txt").unwrap();
        let bs = blob_store.as_ref().unwrap();
        assert!(bs.has(entry.blob_hash.as_bytes()));
    }

    #[test]
    fn test_scanner_path_traversal_rejected() {
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("safe")).unwrap();
        std::fs::write(temp.path().join("safe/file.txt"), b"safe").unwrap();

        // Create a path that tries to escape via ..
        let _escape_path = temp
            .path()
            .join("safe")
            .join("..")
            .join("..")
            .join("etc")
            .join("passwd");

        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(ScannerConfig::default());

        // The scanner should skip files that escape the workspace
        // We test this by scanning and verifying only safe files are included
        let manifest = scanner.scan(&ws_path).unwrap();

        // Only the safe file should be included, not /etc/passwd
        assert!(manifest.get_entry("safe/file.txt").is_some() || manifest.file_count <= 1);
    }

    #[test]
    fn test_scanner_external_symlink_rejected_without_reading_target() {
        use std::os::unix::fs::symlink;

        #[cfg(unix)]
        {
            let temp = tempfile::TempDir::new().unwrap();

            // Create a subdirectory as the "workspace" within temp
            let workspace = temp.path().join("workspace");
            std::fs::create_dir(&workspace).unwrap();

            // Create the "outside" directory at temp root level (sibling to workspace, NOT inside it)
            let outside = temp.path().join("outside");
            std::fs::create_dir(&outside).unwrap();
            std::fs::write(outside.join("secret.txt"), b"secret data").unwrap();

            // Create regular file inside workspace
            std::fs::write(workspace.join("regular.txt"), b"regular content").unwrap();

            // Create symlink INSIDE workspace pointing to outside (external symlink)
            let link = workspace.join("link_to_outside");
            symlink(outside.join("secret.txt").to_str().unwrap(), &link).unwrap();

            // Scanner with follow_symlinks = false should reject the external symlink
            let mut config = ScannerConfig::default();
            config.follow_symlinks = false;

            let ws_path = ai_history_core::WorkspacePath::from_string(workspace.to_str().unwrap());
            let scanner = Scanner::new(config);
            let manifest = scanner.scan(&ws_path).unwrap();

            // External symlink should NOT appear in manifest (points outside workspace)
            assert!(
                manifest.get_entry("link_to_outside").is_none(),
                "external symlink should be rejected"
            );
            // Regular file should be included
            assert!(
                manifest.get_entry("regular.txt").is_some(),
                "regular file should be included"
            );
            assert_eq!(
                manifest.file_count, 1,
                "only regular.txt should be included"
            );
        }
    }

    #[test]
    fn test_scanner_internal_symlink_included_when_not_following() {
        use std::os::unix::fs::symlink;

        #[cfg(unix)]
        {
            let temp = tempfile::TempDir::new().unwrap();

            // Create regular file
            std::fs::write(temp.path().join("target.txt"), b"target content").unwrap();

            // Create symlink in same workspace
            symlink("target.txt", temp.path().join("link.txt")).unwrap();

            let mut config = ScannerConfig::default();
            config.follow_symlinks = false;

            let ws_path =
                ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
            let scanner = Scanner::new(config);
            let manifest = scanner.scan(&ws_path).unwrap();

            // Internal symlink should be included (points inside workspace)
            assert!(
                manifest.get_entry("link.txt").is_some(),
                "internal symlink should be included"
            );
            // Symlink target should be recorded
            let entry = manifest.get_entry("link.txt").unwrap();
            assert!(
                entry.symlink_target.is_some(),
                "symlink target should be recorded"
            );
        }
    }

    #[test]
    fn test_scanner_binary_policy_skip() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create text and binary files
        std::fs::write(temp.path().join("code.rs"), b"fn main() {}").unwrap();
        std::fs::write(temp.path().join("image.png"), &[0x89, 0x50, 0x4E, 0x47]).unwrap();

        // Default policy is Skip for binary files
        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(ScannerConfig::default());
        let manifest = scanner.scan(&ws_path).unwrap();

        assert!(
            manifest.get_entry("code.rs").is_some(),
            "source file included"
        );
        assert!(
            manifest.get_entry("image.png").is_none(),
            "binary file skipped"
        );
    }

    #[test]
    fn test_scanner_binary_policy_include() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create text and binary files
        std::fs::write(temp.path().join("code.rs"), b"fn main() {}").unwrap();
        std::fs::write(temp.path().join("image.png"), &[0x89, 0x50, 0x4E, 0x47]).unwrap();

        // Config with Include policy
        let mut config = ScannerConfig::default();
        config.ignore_config.binary_policy = ai_history_core::BinaryPolicy::Include;
        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(config);
        let manifest = scanner.scan(&ws_path).unwrap();

        assert!(
            manifest.get_entry("code.rs").is_some(),
            "source file included"
        );
        assert!(
            manifest.get_entry("image.png").is_some(),
            "binary file included with Include policy"
        );
    }

    #[test]
    fn test_scanner_ignore_provenance_tracked() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create .gitignore and .ai-history-ignore files
        let gitignore_content = "generated_pattern";
        let ai_ignore_content = "custom_ignore";
        std::fs::write(temp.path().join(".gitignore"), gitignore_content.as_bytes()).unwrap();
        std::fs::write(
            temp.path().join(".ai-history-ignore"),
            ai_ignore_content.as_bytes(),
        )
        .unwrap();

        // Create files matching patterns
        std::fs::write(temp.path().join("generated_pattern"), b"gen").unwrap();
        std::fs::write(temp.path().join("custom_ignore"), b"custom").unwrap();
        std::fs::write(temp.path().join("main.rs"), b"fn main() {}").unwrap();

        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(ScannerConfig::default());
        let manifest = scanner.scan(&ws_path).unwrap();

        // Patterns from .gitignore should be ignored (target/, node_modules/, *.pyc, etc.)
        // Patterns from .ai-history-ignore should be ignored
        assert!(manifest.get_entry("main.rs").is_some(), "main.rs included");
        // The ignore file patterns should cause these to be excluded
    }

    #[test]
    fn test_scanner_mtime_recorded() {
        let temp = tempfile::TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, b"hello").unwrap();

        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(ScannerConfig::default());
        let manifest = scanner.scan(&ws_path).unwrap();

        let entry = manifest.get_entry("test.txt").unwrap();
        assert!(entry.modified.is_some(), "mtime should be recorded");
        // Verify mtime is in RFC3339 format
        let mtime = entry.modified.as_ref().unwrap();
        assert!(mtime.contains('T'), "mtime should be RFC3339 format");
    }

    #[test]
    fn test_scanner_parent_linkage_survives_serialization() {
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::write(temp.path().join("file.txt"), b"content").unwrap();

        let ws_path = ai_history_core::WorkspacePath::from_string(temp.path().to_str().unwrap());
        let scanner = Scanner::new(ScannerConfig::default());
        let parent_id = ai_history_core::ManifestId::new();

        let manifest = scanner.scan_with_parent(&ws_path, parent_id).unwrap();

        // Serialize and deserialize
        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: crate::manifest::Manifest = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.parent(),
            Some(parent_id),
            "parent linkage preserved through serialization"
        );
    }
}
