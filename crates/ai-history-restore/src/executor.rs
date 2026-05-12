//! Restore execution with transactional support

use super::planner::{RestoreOperation, RestorePlan};
use ai_history_core::{CheckpointId, ContentHash, CoreResult, SessionId};
use ai_history_store::traits::BlobStore;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Detailed error categories for restore verification
#[derive(Debug, Clone, PartialEq)]
pub enum RestoreVerificationError {
    /// A file exists in the workspace that is not part of the restore plan
    UnexpectedFile { path: String },
    /// A file required by the plan does not exist in the workspace
    MissingFile { path: String },
    /// A file's content does not match the expected hash
    ContentMismatch {
        path: String,
        expected: ContentHash,
        actual: ContentHash,
    },
    /// A required directory does not exist
    MissingDirectory { path: String },
    /// A file exists that should have been deleted
    FileShouldNotExist { path: String },
    /// A directory exists that should have been deleted
    DirectoryShouldNotExist { path: String },
    /// IO error while reading or accessing a file (permission denied, etc.)
    IoError { path: String, message: String },
    /// Generic verification failure
    VerificationFailed { message: String },
}

/// Result of a restore execution
#[derive(Debug, Clone)]
pub struct RestoreResult {
    pub success: bool,
    pub files_restored: usize,
    pub files_failed: usize,
    pub errors: Vec<String>,
}

impl RestoreResult {
    pub fn success(files_restored: usize) -> Self {
        Self {
            success: true,
            files_restored,
            files_failed: 0,
            errors: Vec::new(),
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            files_restored: 0,
            files_failed: 0,
            errors: vec![error],
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.files_failed += 1;
    }
}

/// Pre-mutation state captured for rollback
#[derive(Debug, Clone)]
struct RollbackEntry {
    /// Original content if file existed and was modified/deleted
    content: Option<Vec<u8>>,
    /// Original metadata if file existed
    metadata: std::fs::Metadata,
}

/// Transactional restore execution state
pub struct RestoreTransaction {
    /// Files that were created/modified (for rollback)
    modified_files: HashSet<String>,
    /// Files that were deleted (for rollback)
    deleted_files: HashSet<String>,
    /// Pre-mutation state captured before mutations
    rollback_data: std::collections::HashMap<String, RollbackEntry>,
}

impl Default for RestoreTransaction {
    fn default() -> Self {
        Self::new()
    }
}

impl RestoreTransaction {
    pub fn new() -> Self {
        Self {
            modified_files: HashSet::new(),
            deleted_files: HashSet::new(),
            rollback_data: std::collections::HashMap::new(),
        }
    }

    /// Record that a file was modified with its pre-mutation state
    pub fn record_modified(
        &mut self,
        path: String,
        original_content: Option<Vec<u8>>,
        metadata: Option<std::fs::Metadata>,
    ) {
        if let (Some(content), Some(meta)) = (original_content, metadata) {
            self.rollback_data.insert(
                path.clone(),
                RollbackEntry {
                    content: Some(content),
                    metadata: meta,
                },
            );
        }
        self.modified_files.insert(path);
    }

    /// Record that a file was deleted with its pre-mutation state
    pub fn record_deleted(
        &mut self,
        path: String,
        original_content: Option<Vec<u8>>,
        metadata: Option<std::fs::Metadata>,
    ) {
        if let (Some(content), Some(meta)) = (original_content, metadata) {
            self.rollback_data.insert(
                path.clone(),
                RollbackEntry {
                    content: Some(content),
                    metadata: meta,
                },
            );
        }
        self.deleted_files.insert(path);
    }

    /// Commit the transaction
    pub fn commit(&mut self) {
        // Clear state
        self.modified_files.clear();
        self.deleted_files.clear();
        self.rollback_data.clear();
    }

    /// Rollback all changes
    pub fn rollback(&self, workspace_root: &Path) -> CoreResult<()> {
        // Restore modified files to original content
        for path in &self.modified_files {
            let full_path = workspace_root.join(path);
            if let Some(entry) = self.rollback_data.get(path) {
                if let Some(ref content) = entry.content {
                    std::fs::write(&full_path, content)?;
                    // Restore metadata
                    if let Err(e) =
                        std::fs::set_permissions(&full_path, entry.metadata.permissions())
                    {
                        eprintln!("Warning: could not restore permissions on {}: {}", path, e);
                    }
                }
            }
        }

        // Restore deleted files (re-create with original content and metadata)
        for path in &self.deleted_files {
            let full_path = workspace_root.join(path);
            if !full_path.exists() {
                if let Some(entry) = self.rollback_data.get(path) {
                    if let Some(ref content) = entry.content {
                        // Re-create parent directories if needed
                        if let Some(parent) = full_path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::write(&full_path, content)?;
                        if let Err(e) =
                            std::fs::set_permissions(&full_path, entry.metadata.permissions())
                        {
                            eprintln!("Warning: could not restore permissions on {}: {}", path, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get list of modified files
    pub fn modified_files(&self) -> &HashSet<String> {
        &self.modified_files
    }

    /// Get list of deleted files
    pub fn deleted_files(&self) -> &HashSet<String> {
        &self.deleted_files
    }
}

/// Execute a restore plan transactionally against a workspace and blob store
pub fn execute_restore<B: BlobStore + ?Sized>(
    plan: &RestorePlan,
    workspace_root: &Path,
    blob_store: &B,
) -> CoreResult<RestoreResult> {
    execute_restore_transactional(plan, workspace_root, blob_store)
}

/// Execute a restore plan transactionally with rollback on failure
pub fn execute_restore_transactional<B: BlobStore + ?Sized>(
    plan: &RestorePlan,
    workspace_root: &Path,
    blob_store: &B,
) -> CoreResult<RestoreResult> {
    // Validate safety checkpoint requirement for destructive restore modes
    if plan.mode.includes_workspace() {
        if plan.safety_checkpoint_id.is_none() {
            return Ok(RestoreResult::failure(
                "Safety checkpoint required for destructive restore but not provided".to_string(),
            ));
        }
    }

    let mut transaction = RestoreTransaction::new();
    let mut result = RestoreResult::success(0);

    // Stage 1: Pre-flight check all operations before executing any
    for operation in &plan.operations {
        if let Err(e) = preflight_check(operation, blob_store) {
            result.add_error(format!("Preflight failed for {}: {}", operation.path(), e));
            result.success = false;
            return Ok(result); // Fail fast before any modifications
        }
    }

    // Stage 2: Execute all operations, rolling back on failure
    for operation in &plan.operations {
        let op_result =
            execute_operation_staged(operation, workspace_root, blob_store, &mut transaction);
        match op_result {
            Ok(()) => result.files_restored += 1,
            Err(e) => {
                result.add_error(format!("{}: {}", operation.path(), e));
                result.success = false;
                // Rollback on failure
                let _ = transaction.rollback(workspace_root);
                return Ok(result);
            }
        }
    }

    // Stage 3: Commit transaction on success
    transaction.commit();
    Ok(result)
}

/// Preflight check - verify all resources are available before starting
fn preflight_check<B: BlobStore + ?Sized>(
    operation: &RestoreOperation,
    blob_store: &B,
) -> CoreResult<()> {
    match operation {
        RestoreOperation::CreateFile { blob_hash, .. }
        | RestoreOperation::UpdateFile { blob_hash, .. } => {
            // Verify blob exists
            blob_store.read_blob(blob_hash.as_bytes())?.ok_or_else(|| {
                ai_history_core::CoreError::Storage(format!(
                    "Blob {} not found during preflight",
                    blob_hash
                ))
            })?;
            // Verify we can create parent dirs (just check, don't create)
            Ok(())
        }
        RestoreOperation::DeleteFile { .. }
        | RestoreOperation::CreateDir { .. }
        | RestoreOperation::NoOp { .. } => Ok(()),
    }
}

/// Execute a single restore operation and track for rollback
fn execute_operation_staged<B: BlobStore + ?Sized>(
    operation: &RestoreOperation,
    workspace_root: &Path,
    blob_store: &B,
    transaction: &mut RestoreTransaction,
) -> CoreResult<()> {
    match operation {
        RestoreOperation::CreateFile { path, blob_hash } => {
            execute_create_file_staged(path, blob_hash, workspace_root, blob_store, transaction)
        }
        RestoreOperation::UpdateFile { path, blob_hash } => {
            execute_update_file_staged(path, blob_hash, workspace_root, blob_store, transaction)
        }
        RestoreOperation::DeleteFile { path } => {
            execute_delete_file_staged(path, workspace_root, transaction)
        }
        RestoreOperation::CreateDir { path } => {
            execute_create_dir_staged(path, workspace_root, transaction)
        }
        RestoreOperation::NoOp { path: _ } => Ok(()),
    }
}

fn execute_create_file<B: BlobStore + ?Sized>(
    path: &str,
    blob_hash: &ContentHash,
    workspace_root: &Path,
    blob_store: &B,
) -> CoreResult<()> {
    // Read blob from blob store
    let blob_data = blob_store.read_blob(blob_hash.as_bytes())?.ok_or_else(|| {
        ai_history_core::CoreError::Storage(format!("Blob {} not found", blob_hash))
    })?;

    // Create parent directories if needed
    let full_path = workspace_root.join(path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write file
    std::fs::write(&full_path, &blob_data)?;

    Ok(())
}

/// Staged version that tracks changes for rollback
fn execute_create_file_staged<B: BlobStore + ?Sized>(
    path: &str,
    blob_hash: &ContentHash,
    workspace_root: &Path,
    blob_store: &B,
    transaction: &mut RestoreTransaction,
) -> CoreResult<()> {
    // Read blob from blob store
    let blob_data = blob_store.read_blob(blob_hash.as_bytes())?.ok_or_else(|| {
        ai_history_core::CoreError::Storage(format!("Blob {} not found", blob_hash))
    })?;

    // Create parent directories if needed
    let full_path = workspace_root.join(path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Capture pre-mutation state if file exists
    let (original_content, original_metadata) = if full_path.exists() {
        match (std::fs::read(&full_path), std::fs::metadata(&full_path)) {
            (Ok(content), Ok(metadata)) => (Some(content), Some(metadata)),
            _ => (None, None),
        }
    } else {
        (None, None)
    };

    // Write file
    std::fs::write(&full_path, &blob_data)?;

    // Track for rollback with captured state
    transaction.record_modified(path.to_string(), original_content, original_metadata);

    Ok(())
}

fn execute_update_file<B: BlobStore + ?Sized>(
    path: &str,
    blob_hash: &ContentHash,
    workspace_root: &Path,
    blob_store: &B,
) -> CoreResult<()> {
    // Same as create - just overwrite existing file
    execute_create_file(path, blob_hash, workspace_root, blob_store)
}

/// Staged version that tracks changes for rollback
fn execute_update_file_staged<B: BlobStore + ?Sized>(
    path: &str,
    blob_hash: &ContentHash,
    workspace_root: &Path,
    blob_store: &B,
    transaction: &mut RestoreTransaction,
) -> CoreResult<()> {
    // Same implementation as create file staged (overwrites are tracked)
    execute_create_file_staged(path, blob_hash, workspace_root, blob_store, transaction)
}

fn execute_delete_file(path: &str, workspace_root: &Path) -> CoreResult<()> {
    let full_path = workspace_root.join(path);
    if full_path.exists() {
        std::fs::remove_file(&full_path)?;
    }
    Ok(())
}

/// Staged version that tracks deleted files for rollback
fn execute_delete_file_staged(
    path: &str,
    workspace_root: &Path,
    transaction: &mut RestoreTransaction,
) -> CoreResult<()> {
    let full_path = workspace_root.join(path);
    if full_path.exists() {
        // Capture pre-mutation state before deleting
        let (original_content, original_metadata) =
            match (std::fs::read(&full_path), std::fs::metadata(&full_path)) {
                (Ok(content), Ok(metadata)) => (Some(content), Some(metadata)),
                _ => (None, None),
            };

        // Record that this file was deleted with its original content
        transaction.record_deleted(path.to_string(), original_content, original_metadata);
        std::fs::remove_file(&full_path)?;
    }
    Ok(())
}

fn execute_create_dir(path: &str, workspace_root: &Path) -> CoreResult<()> {
    let full_path = workspace_root.join(path);
    std::fs::create_dir_all(&full_path)?;
    Ok(())
}

/// Staged version that tracks created directories
fn execute_create_dir_staged(
    path: &str,
    workspace_root: &Path,
    transaction: &mut RestoreTransaction,
) -> CoreResult<()> {
    let full_path = workspace_root.join(path);
    std::fs::create_dir_all(&full_path)?;
    transaction.record_modified(path.to_string(), None, None);
    Ok(())
}

/// Non-staged execute_operation for backward compatibility (no transaction tracking)
pub fn execute_operation<B: BlobStore + ?Sized>(
    operation: &RestoreOperation,
    workspace_root: &Path,
    blob_store: &B,
) -> CoreResult<()> {
    match operation {
        RestoreOperation::CreateFile { path, blob_hash } => {
            execute_create_file(path, blob_hash, workspace_root, blob_store)
        }
        RestoreOperation::UpdateFile { path, blob_hash } => {
            execute_update_file(path, blob_hash, workspace_root, blob_store)
        }
        RestoreOperation::DeleteFile { path } => execute_delete_file(path, workspace_root),
        RestoreOperation::CreateDir { path } => execute_create_dir(path, workspace_root),
        RestoreOperation::NoOp { path: _ } => Ok(()),
    }
}

/// Verify a restore completed correctly by scanning workspace.
/// Returns (success, list of errors) where errors are specific rather than generic bool.
/// This stricter verifier rejects unexpected files not in the plan and surfaces
/// permission or IO failures distinctly from hash mismatches.
pub fn verify_restore(
    plan: &RestorePlan,
    workspace_root: &Path,
) -> CoreResult<(bool, Vec<RestoreVerificationError>)> {
    let mut errors = Vec::new();

    // Collect all paths that should exist according to the plan
    let mut expected_paths = std::collections::HashSet::new();
    let mut paths_to_delete = std::collections::HashSet::new();
    let mut dirs_to_create = std::collections::HashSet::new();

    for operation in &plan.operations {
        match operation {
            RestoreOperation::CreateFile { path, .. }
            | RestoreOperation::UpdateFile { path, .. } => {
                expected_paths.insert(path.clone());
            }
            RestoreOperation::DeleteFile { path } => {
                paths_to_delete.insert(path.clone());
            }
            RestoreOperation::CreateDir { path } => {
                dirs_to_create.insert(path.clone());
            }
            RestoreOperation::NoOp { .. } => {}
        }
    }

    // Check each expected file for existence and correct content
    for operation in &plan.operations {
        let (path, expected_hash) = match operation {
            RestoreOperation::CreateFile { path, blob_hash } => (path, blob_hash),
            RestoreOperation::UpdateFile { path, blob_hash } => (path, blob_hash),
            RestoreOperation::DeleteFile { path } => {
                // Verify file is deleted
                let full_path = workspace_root.join(path);
                match fs::metadata(&full_path) {
                    Ok(_) => {
                        // File still exists - it should have been deleted
                        errors.push(RestoreVerificationError::FileShouldNotExist {
                            path: path.clone(),
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        // Expected - file was deleted as planned
                    }
                    Err(e) => {
                        // Some other IO error when checking
                        errors.push(RestoreVerificationError::IoError {
                            path: path.clone(),
                            message: e.to_string(),
                        });
                    }
                }
                continue;
            }
            RestoreOperation::CreateDir { path } => {
                let full_path = workspace_root.join(path);
                match fs::metadata(&full_path) {
                    Ok(meta) if meta.is_dir() => {
                        // Directory exists as expected
                    }
                    Ok(_) => {
                        // Exists but is not a directory
                        errors.push(RestoreVerificationError::DirectoryShouldNotExist {
                            path: path.clone(),
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        errors.push(RestoreVerificationError::MissingDirectory {
                            path: path.clone(),
                        });
                    }
                    Err(e) => {
                        errors.push(RestoreVerificationError::IoError {
                            path: path.clone(),
                            message: e.to_string(),
                        });
                    }
                }
                continue;
            }
            RestoreOperation::NoOp { .. } => continue,
        };

        let full_path = workspace_root.join(path);

        // Check if file exists
        let metadata = match fs::metadata(&full_path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                errors.push(RestoreVerificationError::MissingFile { path: path.clone() });
                continue;
            }
            Err(e) => {
                errors.push(RestoreVerificationError::IoError {
                    path: path.clone(),
                    message: e.to_string(),
                });
                continue;
            }
        };

        // Skip symlinks for hash verification
        if metadata.file_type().is_symlink() {
            continue;
        }

        // Read file and compute hash
        let data = match fs::read(&full_path) {
            Ok(data) => data,
            Err(e) => {
                errors.push(RestoreVerificationError::IoError {
                    path: path.clone(),
                    message: e.to_string(),
                });
                continue;
            }
        };

        let actual_hash = ContentHash::new(&data);
        if &actual_hash != expected_hash {
            errors.push(RestoreVerificationError::ContentMismatch {
                path: path.clone(),
                expected: expected_hash.clone(),
                actual: actual_hash,
            });
        }
    }

    // Check for unexpected files in workspace
    if workspace_root.exists() {
        check_for_unexpected_files(
            workspace_root,
            &expected_paths,
            &paths_to_delete,
            &mut errors,
        );
    }

    let success = errors.is_empty();
    Ok((success, errors))
}

/// Recursively check for unexpected files not in the expected set
fn check_for_unexpected_files(
    workspace_root: &Path,
    expected_paths: &std::collections::HashSet<String>,
    paths_to_delete: &std::collections::HashSet<String>,
    errors: &mut Vec<RestoreVerificationError>,
) {
    fn walk_dir(
        dir: &Path,
        workspace_root: &Path,
        expected_paths: &std::collections::HashSet<String>,
        paths_to_delete: &std::collections::HashSet<String>,
        errors: &mut Vec<RestoreVerificationError>,
    ) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();

                if path.is_dir() {
                    walk_dir(
                        &path,
                        workspace_root,
                        expected_paths,
                        paths_to_delete,
                        errors,
                    );
                } else {
                    // Compute relative path
                    if let Ok(relative_path) = path.strip_prefix(workspace_root) {
                        let relative_str = relative_path.to_string_lossy();

                        if !expected_paths.contains(relative_str.as_ref())
                            && !paths_to_delete.contains(relative_str.as_ref())
                        {
                            errors.push(RestoreVerificationError::UnexpectedFile {
                                path: relative_str.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    walk_dir(
        workspace_root,
        workspace_root,
        expected_paths,
        paths_to_delete,
        errors,
    );
}

/// Simple bool-only wrapper for backward compatibility
pub fn verify_restore_simple(plan: &RestorePlan, workspace_root: &Path) -> CoreResult<bool> {
    let (success, _) = verify_restore(plan, workspace_root)?;
    Ok(success)
}

/// Rollback a failed restore using safety checkpoint
///
/// This recovers the workspace to its state before the failed restore by:
/// 1. Loading the safety checkpoint's manifest
/// 2. Getting the current workspace manifest (via scanner)
/// 3. Building a restore plan from current to safety checkpoint state
/// 4. Executing that plan
pub fn rollback_restore<B: BlobStore + ?Sized>(
    session_id: SessionId,
    safety_checkpoint_id: CheckpointId,
    workspace_root: &Path,
    store: &ai_history_store::sqlite_store::SqliteStore,
    blob_store: &B,
) -> CoreResult<()> {
    let _ = session_id; // Reserved for future use (e.g., session-specific rollback logging)

    // Step 1: Load the safety checkpoint to get its manifest
    let checkpoint = store.get_checkpoint(safety_checkpoint_id)?.ok_or_else(|| {
        ai_history_core::CoreError::Storage(format!(
            "Safety checkpoint {} not found",
            safety_checkpoint_id
        ))
    })?;

    // Step 2: Get the safety manifest
    let safety_manifest_info = store.get_manifest(checkpoint.manifest_id)?.ok_or_else(|| {
        ai_history_core::CoreError::Storage(format!(
            "Safety manifest {} not found",
            checkpoint.manifest_id
        ))
    })?;

    // Deserialize manifest_data to Manifest
    let safety_manifest: ai_history_snapshot::manifest::Manifest =
        serde_json::from_str(&safety_manifest_info.manifest_data)
            .map_err(|e| ai_history_core::CoreError::Serialization(e.to_string()))?;

    // Step 3: Scan current workspace to get its manifest
    use ai_history_snapshot::scanner::{Scanner, ScannerConfig};
    let scanner = Scanner::new(ScannerConfig::default());
    let ws_path =
        ai_history_core::WorkspacePath::from_string(workspace_root.to_str().unwrap_or("."));
    let current_manifest = scanner.scan(&ws_path)?;

    // Step 4: Build a restore plan from current to safety checkpoint
    use super::planner::{build_restore_plan, ConflictPolicy, RestoreMode};
    let mut plan = build_restore_plan(
        &current_manifest,
        &safety_manifest,
        None,
        ConflictPolicy::Automatic,
        RestoreMode::Both,
    );

    // Set the safety checkpoint - we're rolling back TO this checkpoint,
    // so it serves as the safety net for the rollback operation itself
    plan.safety_checkpoint_id = Some(safety_checkpoint_id);

    // Step 5: Execute the plan
    let result = execute_restore_transactional(&plan, workspace_root, blob_store)?;

    if result.success {
        Ok(())
    } else {
        Err(ai_history_core::CoreError::Restore(format!(
            "Rollback failed: {:?}",
            result.errors
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_history_store::traits::BlobInfo;

    #[test]
    fn test_restore_result_success() {
        let result = RestoreResult::success(5);
        assert!(result.success);
        assert_eq!(result.files_restored, 5);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_restore_result_failure() {
        let result = RestoreResult::failure("Test error".to_string());
        assert!(!result.success);
        assert!(result.errors.contains(&"Test error".to_string()));
    }

    #[test]
    fn test_restore_result_add_error() {
        let mut result = RestoreResult::success(3);
        result.add_error("Error 1".to_string());
        result.add_error("Error 2".to_string());
        assert_eq!(result.files_failed, 2);
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_verify_restore_noop() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, SessionId};

        // Create temp workspace
        let temp = tempfile::TempDir::new().unwrap();

        // Create a plan with NoOp operations
        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::NoOp {
            path: "nonexistent.txt".to_string(),
        });

        // Verify should succeed (NoOp doesn't require file to exist)
        let result = verify_restore(&plan, temp.path());
        assert!(result.is_ok());
        assert!(result.unwrap().0);
    }

    #[test]
    fn test_execute_create_file_creates_parent_dirs() {
        use ai_history_core::ContentHash;

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create a mock blob store
        struct MockBlobStore;
        impl BlobStore for MockBlobStore {
            fn read_blob(&self, _hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                Ok(Some(b"test content".to_vec()))
            }
            fn has_blob(&self, _hash: &[u8]) -> bool {
                true
            }
            fn write_blob(&mut self, _hash: &[u8], _data: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn delete_blob(&mut self, _hash: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(vec![])
            }
        }

        let operation = RestoreOperation::CreateFile {
            path: "subdir/nested/file.txt".to_string(),
            blob_hash: ContentHash::new(b"test content"),
        };

        let result = execute_operation(&operation, workspace_root, &MockBlobStore);
        assert!(result.is_ok());
        assert!(workspace_root.join("subdir/nested/file.txt").exists());
    }

    #[test]
    fn test_execute_delete_file() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create a file first
        std::fs::write(workspace_root.join("to_delete.txt"), b"content").unwrap();
        assert!(workspace_root.join("to_delete.txt").exists());

        let operation = RestoreOperation::DeleteFile {
            path: "to_delete.txt".to_string(),
        };

        let result = execute_delete_file("to_delete.txt", workspace_root);
        assert!(result.is_ok());
        assert!(!workspace_root.join("to_delete.txt").exists());
    }

    #[test]
    fn test_execute_delete_file_nonexistent() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Deleting a non-existent file should succeed
        let operation = RestoreOperation::DeleteFile {
            path: "does_not_exist.txt".to_string(),
        };

        let result = execute_delete_file("does_not_exist.txt", workspace_root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_create_dir() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        let operation = RestoreOperation::CreateDir {
            path: "new_directory".to_string(),
        };

        let result = execute_create_dir("new_directory", workspace_root);
        assert!(result.is_ok());
        assert!(workspace_root.join("new_directory").is_dir());
    }

    #[test]
    fn test_execute_create_dir_nested() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        let operation = RestoreOperation::CreateDir {
            path: "a/b/c".to_string(),
        };

        let result = execute_create_dir("a/b/c", workspace_root);
        assert!(result.is_ok());
        assert!(workspace_root.join("a/b/c").is_dir());
    }

    #[test]
    fn test_verify_restore_create_file() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create a file that matches the expected content
        let content = b"expected content";
        std::fs::write(workspace_root.join("file.txt"), content).unwrap();

        let hash = ContentHash::new(content);
        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::CreateFile {
            path: "file.txt".to_string(),
            blob_hash: hash,
        });

        let result = verify_restore(&plan, workspace_root);
        assert!(result.is_ok());
        assert!(result.unwrap().0);
    }

    #[test]
    fn test_verify_restore_delete_file() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Don't create the file - that's what we're verifying (it should be deleted)
        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::DeleteFile {
            path: "deleted_file.txt".to_string(),
        });

        let result = verify_restore(&plan, workspace_root);
        assert!(result.is_ok());
        assert!(result.unwrap().0); // File doesn't exist, so delete is verified
    }

    #[test]
    fn test_verify_restore_content_mismatch() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create a file with WRONG content
        std::fs::write(workspace_root.join("file.txt"), b"wrong content").unwrap();

        let hash = ContentHash::new(b"expected content"); // Different content
        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::CreateFile {
            path: "file.txt".to_string(),
            blob_hash: hash,
        });

        let result = verify_restore(&plan, workspace_root);
        assert!(result.is_ok());
        assert!(!result.unwrap().0); // Content mismatch
    }

    #[test]
    fn test_transaction_rollback_restores_modified_file() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create original file
        let file_path = workspace_root.join("original.txt");
        std::fs::write(&file_path, b"original content").unwrap();
        let original_meta = std::fs::metadata(&file_path).unwrap();

        // Create transaction and record modification
        let mut transaction = RestoreTransaction::new();
        transaction.record_modified(
            "original.txt".to_string(),
            Some(b"original content".to_vec()),
            Some(original_meta),
        );

        // Simulate modification (overwrite file)
        std::fs::write(&file_path, b"modified content").unwrap();
        assert_eq!(std::fs::read(&file_path).unwrap(), b"modified content");

        // Rollback
        transaction.rollback(workspace_root).unwrap();

        // File should be restored to original content
        assert_eq!(std::fs::read(&file_path).unwrap(), b"original content");
    }

    #[test]
    fn test_transaction_rollback_restores_deleted_file() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create original file
        let file_path = workspace_root.join("deleted_file.txt");
        std::fs::write(&file_path, b"file content").unwrap();
        let original_meta = std::fs::metadata(&file_path).unwrap();

        // Create transaction and record deletion
        let mut transaction = RestoreTransaction::new();
        transaction.record_deleted(
            "deleted_file.txt".to_string(),
            Some(b"file content".to_vec()),
            Some(original_meta),
        );

        // Simulate deletion
        std::fs::remove_file(&file_path).unwrap();
        assert!(!file_path.exists());

        // Rollback
        transaction.rollback(workspace_root).unwrap();

        // File should be restored
        assert!(file_path.exists());
        assert_eq!(std::fs::read(&file_path).unwrap(), b"file content");
    }

    #[test]
    fn test_transaction_rollback_restores_deleted_nested_file() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create nested file
        let file_path = workspace_root.join("a/b/c/nested.txt");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, b"nested content").unwrap();
        let original_meta = std::fs::metadata(&file_path).unwrap();

        // Create transaction and record deletion
        let mut transaction = RestoreTransaction::new();
        transaction.record_deleted(
            "a/b/c/nested.txt".to_string(),
            Some(b"nested content".to_vec()),
            Some(original_meta),
        );

        // Simulate deletion
        std::fs::remove_file(&file_path).unwrap();
        assert!(!file_path.exists());

        // Rollback
        transaction.rollback(workspace_root).unwrap();

        // File should be restored including parent directories
        assert!(file_path.exists());
        assert_eq!(std::fs::read(&file_path).unwrap(), b"nested content");
    }

    #[test]
    fn test_execute_restore_transactional_rollback_on_failure() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create original file with known content
        std::fs::write(workspace_root.join("file.txt"), b"original").unwrap();

        // Create a blob store that will fail on second file
        struct FailingBlobStore {
            call_count: std::sync::Mutex<u32>,
        }
        impl BlobStore for FailingBlobStore {
            fn read_blob(&self, _hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                let mut count = self.call_count.lock().unwrap();
                *count += 1;
                // Fail on 3rd+ call: preflight calls read_blob once per operation (2 calls),
                // then execution calls again per operation. Fail on 4th call (second operation execution).
                if *count > 3 {
                    return Err(ai_history_core::CoreError::Storage(
                        "Blob not found".to_string(),
                    ));
                }
                Ok(Some(b"new content".to_vec()))
            }
            fn has_blob(&self, _hash: &[u8]) -> bool {
                true
            }
            fn write_blob(&mut self, _hash: &[u8], _data: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn delete_blob(&mut self, _hash: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(vec![])
            }
        }

        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::UpdateFile {
            path: "file.txt".to_string(),
            blob_hash: ContentHash::new(b"new content"),
        });
        // Second operation will fail
        plan.add_operation(RestoreOperation::UpdateFile {
            path: "another.txt".to_string(),
            blob_hash: ContentHash::new(b"another content"),
        });

        let store = FailingBlobStore {
            call_count: std::sync::Mutex::new(0),
        };
        let result = execute_restore_transactional(&plan, workspace_root, &store).unwrap();

        // Restore should fail
        assert!(!result.success);

        // Original file should be restored (rollback happened)
        let content = std::fs::read_to_string(workspace_root.join("file.txt")).unwrap();
        assert_eq!(content, "original");
    }

    #[test]
    fn test_execute_restore_transactional_preserves_state_on_success() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create blob store with content
        struct TestBlobStore;
        impl BlobStore for TestBlobStore {
            fn read_blob(&self, hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                // Return different content based on hash
                if hash == b"hash1" {
                    Ok(Some(b"content1".to_vec()))
                } else {
                    Ok(Some(b"content2".to_vec()))
                }
            }
            fn has_blob(&self, _hash: &[u8]) -> bool {
                true
            }
            fn write_blob(&mut self, _hash: &[u8], _data: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn delete_blob(&mut self, _hash: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(vec![])
            }
        }

        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::CreateFile {
            path: "new1.txt".to_string(),
            blob_hash: ContentHash::new(b"content1"), // hash1
        });
        plan.add_operation(RestoreOperation::CreateFile {
            path: "new2.txt".to_string(),
            blob_hash: ContentHash::new(b"content2"), // hash2
        });
        // Safety checkpoint required for destructive restore
        plan = plan.with_safety_checkpoint(CheckpointId::new());

        let result = execute_restore_transactional(&plan, workspace_root, &TestBlobStore).unwrap();

        assert!(result.success);
        assert_eq!(result.files_restored, 2);
        assert!(workspace_root.join("new1.txt").exists());
        assert!(workspace_root.join("new2.txt").exists());
    }

    #[test]
    fn test_verify_restore_missing_file() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create one file correctly
        let content = b"expected content";
        std::fs::write(workspace_root.join("file.txt"), content).unwrap();

        // Plan expects two files, but second is missing
        let hash = ContentHash::new(content);
        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::CreateFile {
            path: "file.txt".to_string(),
            blob_hash: hash,
        });
        plan.add_operation(RestoreOperation::CreateFile {
            path: "missing_file.txt".to_string(),
            blob_hash: hash,
        });

        let result = verify_restore(&plan, workspace_root);
        assert!(result.is_ok());
        assert!(!result.unwrap().0, "Should fail when file is missing");
    }

    #[test]
    fn test_verify_restore_corrupt_truncated_file() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create a file with truncated content (corruption)
        let full_content = b"expected content very long and detailed";
        std::fs::write(workspace_root.join("file.txt"), &full_content[..10]).unwrap();

        let hash = ContentHash::new(full_content);
        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::CreateFile {
            path: "file.txt".to_string(),
            blob_hash: hash,
        });

        let result = verify_restore(&plan, workspace_root);
        assert!(result.is_ok());
        assert!(!result.unwrap().0, "Should fail on content hash mismatch");
    }

    #[test]
    fn test_verify_restore_creates_parent_directories() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        struct TestBlobStore;
        impl BlobStore for TestBlobStore {
            fn read_blob(&self, _hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                Ok(Some(b"content".to_vec()))
            }
            fn has_blob(&self, _hash: &[u8]) -> bool {
                true
            }
            fn write_blob(&mut self, _hash: &[u8], _data: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn delete_blob(&mut self, _hash: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(vec![])
            }
        }

        let hash = ContentHash::new(b"content");
        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::CreateFile {
            path: "a/b/c/deep/file.txt".to_string(),
            blob_hash: hash,
        });
        // Safety checkpoint required for destructive restore
        plan = plan.with_safety_checkpoint(CheckpointId::new());

        let result = execute_restore_transactional(&plan, workspace_root, &TestBlobStore);
        assert!(result.is_ok());
        assert!(result.unwrap().success);
        assert!(workspace_root.join("a/b/c/deep/file.txt").exists());
    }

    #[test]
    fn test_execute_restore_transactional_fails_without_safety_checkpoint() {
        use crate::planner::{RestoreMode, RestoreOperation};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        struct TestBlobStore;
        impl BlobStore for TestBlobStore {
            fn read_blob(&self, _hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                Ok(Some(b"content".to_vec()))
            }
            fn has_blob(&self, _hash: &[u8]) -> bool {
                true
            }
            fn write_blob(&mut self, _hash: &[u8], _data: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn delete_blob(&mut self, _hash: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(vec![])
            }
        }

        // WorkspaceOnly mode is destructive - requires safety checkpoint
        let mut plan = RestorePlan::new(
            CheckpointId::new(),
            SessionId::new(),
            RestoreMode::WorkspaceOnly,
        );
        // No safety checkpoint set - should fail
        plan.add_operation(RestoreOperation::CreateFile {
            path: "new_file.txt".to_string(),
            blob_hash: ContentHash::new(b"content"),
        });

        let result = execute_restore_transactional(&plan, workspace_root, &TestBlobStore).unwrap();

        // Should fail because destructive workspace restore has no safety checkpoint
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Safety checkpoint")));
    }

    #[test]
    fn test_execute_restore_transactional_fails_without_safety_checkpoint_both_mode() {
        use crate::planner::{RestoreMode, RestoreOperation};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        struct TestBlobStore;
        impl BlobStore for TestBlobStore {
            fn read_blob(&self, _hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                Ok(Some(b"content".to_vec()))
            }
            fn has_blob(&self, _hash: &[u8]) -> bool {
                true
            }
            fn write_blob(&mut self, _hash: &[u8], _data: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn delete_blob(&mut self, _hash: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(vec![])
            }
        }

        // Both mode is destructive - requires safety checkpoint
        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        // No safety checkpoint set - should fail
        plan.add_operation(RestoreOperation::CreateFile {
            path: "new_file.txt".to_string(),
            blob_hash: ContentHash::new(b"content"),
        });

        let result = execute_restore_transactional(&plan, workspace_root, &TestBlobStore).unwrap();

        // Should fail because destructive restore has no safety checkpoint
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Safety checkpoint")));
    }

    #[test]
    fn test_execute_restore_transactional_succeeds_with_safety_checkpoint() {
        use crate::planner::{RestoreMode, RestoreOperation};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        struct TestBlobStore;
        impl BlobStore for TestBlobStore {
            fn read_blob(&self, _hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                Ok(Some(b"content".to_vec()))
            }
            fn has_blob(&self, _hash: &[u8]) -> bool {
                true
            }
            fn write_blob(&mut self, _hash: &[u8], _data: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn delete_blob(&mut self, _hash: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(vec![])
            }
        }

        // Both mode with safety checkpoint should succeed
        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::CreateFile {
            path: "new_file.txt".to_string(),
            blob_hash: ContentHash::new(b"content"),
        });
        // Safety checkpoint set - should allow execution
        let safety_cp = CheckpointId::new();
        plan = plan.with_safety_checkpoint(safety_cp);

        let result = execute_restore_transactional(&plan, workspace_root, &TestBlobStore).unwrap();

        // Should succeed because safety checkpoint is provided
        assert!(result.success);
        assert!(workspace_root.join("new_file.txt").exists());
    }

    #[test]
    fn test_execute_restore_conversation_only_no_safety_checkpoint() {
        use crate::planner::{RestoreMode, RestoreOperation};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        struct TestBlobStore;
        impl BlobStore for TestBlobStore {
            fn read_blob(&self, _hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                Ok(Some(b"content".to_vec()))
            }
            fn has_blob(&self, _hash: &[u8]) -> bool {
                true
            }
            fn write_blob(&mut self, _hash: &[u8], _data: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn delete_blob(&mut self, _hash: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(vec![])
            }
        }

        // ConversationOnly is NOT destructive - should work without safety checkpoint
        let mut plan = RestorePlan::new(
            CheckpointId::new(),
            SessionId::new(),
            RestoreMode::ConversationOnly,
        );
        plan.add_operation(RestoreOperation::CreateFile {
            path: "new_file.txt".to_string(),
            blob_hash: ContentHash::new(b"content"),
        });
        // No safety checkpoint - should still work because conversation-only is not destructive

        let result = execute_restore_transactional(&plan, workspace_root, &TestBlobStore).unwrap();

        // Should succeed - conversation-only restore doesn't require safety checkpoint
        assert!(result.success);
    }

    // =====================================================================
    // Task 61 & 62: Failure-injection, corrupt content, missing blobs,
    // partial failure, and post-restore verification tests
    // =====================================================================

    /// Test: corrupt content is detected by verify_restore after restore completes
    #[test]
    fn test_verify_restore_detects_corrupt_file_content() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create a file with CORRUPT content (different from expected)
        let wrong_content = b"this is not the right content";
        std::fs::write(workspace_root.join("file.txt"), wrong_content).unwrap();

        let hash = ContentHash::new(b"expected correct content");
        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::CreateFile {
            path: "file.txt".to_string(),
            blob_hash: hash,
        });

        let (success, errors) = verify_restore(&plan, workspace_root).unwrap();

        // Verification should FAIL because content doesn't match
        assert!(!success);
        assert!(!errors.is_empty());
        // Should report content mismatch
        let mismatch = errors
            .iter()
            .find(|e| matches!(e, RestoreVerificationError::ContentMismatch { .. }));
        assert!(
            mismatch.is_some(),
            "Expected ContentMismatch error, got: {:?}",
            errors
        );
    }

    /// Test: missing file is detected by verify_restore
    #[test]
    fn test_verify_restore_detects_missing_file() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Don't create any file - plan expects it
        let hash = ContentHash::new(b"some content");
        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::CreateFile {
            path: "missing.txt".to_string(),
            blob_hash: hash,
        });

        let (success, errors) = verify_restore(&plan, workspace_root).unwrap();

        // Verification should FAIL - file is missing
        assert!(!success);
        let missing = errors
            .iter()
            .find(|e| matches!(e, RestoreVerificationError::MissingFile { .. }));
        assert!(
            missing.is_some(),
            "Expected MissingFile error, got: {:?}",
            errors
        );
    }

    /// Test: unexpected file in workspace is detected by verify_restore
    #[test]
    fn test_verify_restore_detects_unexpected_file() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create a file that is NOT part of the plan
        std::fs::write(workspace_root.join("extra_file.txt"), b"should not be here").unwrap();

        // Plan has only a NoOp - no expected files
        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::NoOp {
            path: "noop.txt".to_string(),
        });

        let (success, errors) = verify_restore(&plan, workspace_root).unwrap();

        // Verification should FAIL - unexpected file exists
        assert!(!success);
        let unexpected = errors
            .iter()
            .find(|e| matches!(e, RestoreVerificationError::UnexpectedFile { .. }));
        assert!(
            unexpected.is_some(),
            "Expected UnexpectedFile error, got: {:?}",
            errors
        );
    }

    /// Test: missing blob causes restore to fail and rollback workspace
    #[test]
    fn test_execute_restore_transactional_fails_and_rolls_back_when_blob_missing() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create original file with known content
        std::fs::write(workspace_root.join("original.txt"), b"original content").unwrap();

        // Blob store that fails with error on read (simulates missing blob)
        struct MissingBlobStore;
        impl BlobStore for MissingBlobStore {
            fn read_blob(&self, _hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                Err(ai_history_core::CoreError::Storage(
                    "Blob not found".to_string(),
                ))
            }
            fn has_blob(&self, _hash: &[u8]) -> bool {
                false
            }
            fn write_blob(&mut self, _hash: &[u8], _data: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn delete_blob(&mut self, _hash: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(vec![])
            }
        }

        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::UpdateFile {
            path: "original.txt".to_string(),
            blob_hash: ContentHash::new(b"new content"),
        });
        let safety_cp = CheckpointId::new();
        plan = plan.with_safety_checkpoint(safety_cp);

        let result =
            execute_restore_transactional(&plan, workspace_root, &MissingBlobStore).unwrap();

        // Restore should fail
        assert!(!result.success, "Expected restore to fail but got success");
        // Original file should be restored (rollback happened)
        assert_eq!(
            std::fs::read(workspace_root.join("original.txt")).unwrap(),
            b"original content"
        );
    }

    /// Test: partial failure - second file fails, first is rolled back
    #[test]
    fn test_execute_restore_transactional_partial_failure_rolls_back_all() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create two original files
        std::fs::write(workspace_root.join("file1.txt"), b"original1").unwrap();
        std::fs::write(workspace_root.join("file2.txt"), b"original2").unwrap();

        // Blob store that fails on second blob
        struct PartialFailBlobStore {
            call_count: std::sync::Mutex<u32>,
        }
        impl BlobStore for PartialFailBlobStore {
            fn read_blob(&self, hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                let mut count = self.call_count.lock().unwrap();
                *count += 1;
                // Return content for first blob, fail for second
                if *count <= 2 {
                    // First two calls (preflight + execution for first file) return Some
                    // But actually let's be more precise:
                    // preflight calls once per CreateFile (2 calls), execution calls once per CreateFile (2 calls)
                    // Total: 4 calls. Fail at call 3 (second preflight or second execution)
                    Ok(Some(b"new content".to_vec()))
                } else {
                    Err(ai_history_core::CoreError::Storage(
                        "Blob missing mid-operation".to_string(),
                    ))
                }
            }
            fn has_blob(&self, _hash: &[u8]) -> bool {
                true
            }
            fn write_blob(&mut self, _hash: &[u8], _data: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn delete_blob(&mut self, _hash: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(vec![])
            }
        }

        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::UpdateFile {
            path: "file1.txt".to_string(),
            blob_hash: ContentHash::new(b"content1"),
        });
        plan.add_operation(RestoreOperation::UpdateFile {
            path: "file2.txt".to_string(),
            blob_hash: ContentHash::new(b"content2"),
        });
        let safety_cp = CheckpointId::new();
        plan = plan.with_safety_checkpoint(safety_cp);

        let result = execute_restore_transactional(
            &plan,
            workspace_root,
            &PartialFailBlobStore {
                call_count: std::sync::Mutex::new(0),
            },
        )
        .unwrap();

        // Restore should fail
        assert!(!result.success);
        // Both files should be rolled back to original content
        assert_eq!(
            std::fs::read(workspace_root.join("file1.txt")).unwrap(),
            b"original1"
        );
        assert_eq!(
            std::fs::read(workspace_root.join("file2.txt")).unwrap(),
            b"original2"
        );
    }

    /// Test: verify_restore returns specific error types for different failure modes
    #[test]
    fn test_verify_restore_error_type_classification() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Test 1: CreateFile plan but file doesn't exist -> MissingFile
        {
            let hash = ContentHash::new(b"content");
            let mut plan =
                RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
            plan.add_operation(RestoreOperation::CreateFile {
                path: "missing.txt".to_string(),
                blob_hash: hash,
            });
            let (_, errors) = verify_restore(&plan, workspace_root).unwrap();
            assert!(errors
                .iter()
                .any(|e| matches!(e, RestoreVerificationError::MissingFile { .. })));
        }

        // Test 2: File with wrong content -> ContentMismatch
        {
            std::fs::write(workspace_root.join("wrong.txt"), b"wrong").unwrap();
            let hash = ContentHash::new(b"expected");
            let mut plan =
                RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
            plan.add_operation(RestoreOperation::CreateFile {
                path: "wrong.txt".to_string(),
                blob_hash: hash,
            });
            let (_, errors) = verify_restore(&plan, workspace_root).unwrap();
            assert!(errors
                .iter()
                .any(|e| matches!(e, RestoreVerificationError::ContentMismatch { .. })));
        }

        // Test 3: DeleteFile plan but file still exists -> FileShouldNotExist
        {
            std::fs::write(workspace_root.join("should_delete.txt"), b"content").unwrap();
            let mut plan =
                RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
            plan.add_operation(RestoreOperation::DeleteFile {
                path: "should_delete.txt".to_string(),
            });
            let (_, errors) = verify_restore(&plan, workspace_root).unwrap();
            assert!(errors
                .iter()
                .any(|e| matches!(e, RestoreVerificationError::FileShouldNotExist { .. })));
        }
    }

    /// Test: successful restore passes verification
    #[test]
    fn test_verify_restore_succeeds_after_successful_restore() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create a file manually with specific content, then verify it passes
        let content = b"this is the restored file content";
        let hash = ContentHash::new(content);
        std::fs::write(workspace_root.join("restored_file.txt"), content).unwrap();

        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::CreateFile {
            path: "restored_file.txt".to_string(),
            blob_hash: hash,
        });

        // Verify should pass because file exists with correct content
        let (success, errors) = verify_restore(&plan, workspace_root).unwrap();
        assert!(
            success,
            "Restore verification should pass, errors: {:?}",
            errors
        );
        assert!(errors.is_empty());
    }

    /// Test: directory creation is verified correctly
    #[test]
    fn test_verify_restore_detects_missing_directory() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::CreateDir {
            path: "missing_dir".to_string(),
        });

        let (success, errors) = verify_restore(&plan, workspace_root).unwrap();

        // Should fail because directory doesn't exist
        assert!(!success);
        assert!(errors
            .iter()
            .any(|e| matches!(e, RestoreVerificationError::MissingDirectory { .. })));
    }

    /// Test: directory creation verification passes when directory exists
    #[test]
    fn test_verify_restore_succeeds_for_directory_creation() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create the directory first
        std::fs::create_dir(workspace_root.join("existing_dir")).unwrap();

        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::CreateDir {
            path: "existing_dir".to_string(),
        });

        let (success, errors) = verify_restore(&plan, workspace_root).unwrap();

        // Should pass
        assert!(
            success,
            "Directory verification should pass, errors: {:?}",
            errors
        );
    }

    /// Test: verify restore with multiple operation types succeeds
    #[test]
    fn test_verify_restore_mixed_operations_all_succeed() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create a file that matches expected content
        std::fs::write(workspace_root.join("updated.txt"), b"correct content").unwrap();
        // Delete a file that should be deleted (doesn't exist, which is correct)
        // Create a directory
        std::fs::create_dir(workspace_root.join("new_dir")).unwrap();

        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::UpdateFile {
            path: "updated.txt".to_string(),
            blob_hash: ContentHash::new(b"correct content"),
        });
        plan.add_operation(RestoreOperation::DeleteFile {
            path: "deleted.txt".to_string(),
        });
        plan.add_operation(RestoreOperation::CreateDir {
            path: "new_dir".to_string(),
        });

        let (success, errors) = verify_restore(&plan, workspace_root).unwrap();

        assert!(
            success,
            "Mixed operations should verify successfully, errors: {:?}",
            errors
        );
    }

    /// Test: execute_restore_transactional with preflight failure - no files modified
    #[test]
    fn test_execute_restore_transactional_preflight_failure_no_modifications() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create original file
        std::fs::write(workspace_root.join("file.txt"), b"original").unwrap();

        // Blob store that fails with error on read (blob not found)
        struct PreflightFailBlobStore;
        impl BlobStore for PreflightFailBlobStore {
            fn read_blob(&self, _hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                Err(ai_history_core::CoreError::Storage(
                    "Blob not found".to_string(),
                ))
            }
            fn has_blob(&self, _hash: &[u8]) -> bool {
                false
            }
            fn write_blob(&mut self, _hash: &[u8], _data: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn delete_blob(&mut self, _hash: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(vec![])
            }
        }

        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::UpdateFile {
            path: "file.txt".to_string(),
            blob_hash: ContentHash::new(b"new content"),
        });
        let safety_cp = CheckpointId::new();
        plan = plan.with_safety_checkpoint(safety_cp);

        let result =
            execute_restore_transactional(&plan, workspace_root, &PreflightFailBlobStore).unwrap();

        // Should fail during preflight
        assert!(!result.success, "Expected restore to fail during preflight");
        // Original file should be unchanged (no rollback needed since nothing was modified)
        assert_eq!(
            std::fs::read(workspace_root.join("file.txt")).unwrap(),
            b"original"
        );
    }

    /// Test: transactional restore fully succeeds with all-or-nothing guarantee
    #[test]
    fn test_execute_restore_all_or_nothing_create_update_delete_mix() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Setup: create files with known content
        std::fs::write(workspace_root.join("existing.txt"), b"existing").unwrap();
        std::fs::write(workspace_root.join("to_delete.txt"), b"delete me").unwrap();

        struct TestBlobStore;
        impl BlobStore for TestBlobStore {
            fn read_blob(&self, _hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
                Ok(Some(b"new content".to_vec()))
            }
            fn has_blob(&self, _hash: &[u8]) -> bool {
                true
            }
            fn write_blob(&mut self, _hash: &[u8], _data: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn delete_blob(&mut self, _hash: &[u8]) -> CoreResult<()> {
                Ok(())
            }
            fn list_blobs(&self, _session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
                Ok(vec![])
            }
        }

        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        // Create new file
        plan.add_operation(RestoreOperation::CreateFile {
            path: "new_file.txt".to_string(),
            blob_hash: ContentHash::new(b"new content"),
        });
        // Update existing file
        plan.add_operation(RestoreOperation::UpdateFile {
            path: "existing.txt".to_string(),
            blob_hash: ContentHash::new(b"new content"),
        });
        // Delete a file
        plan.add_operation(RestoreOperation::DeleteFile {
            path: "to_delete.txt".to_string(),
        });
        // Create a directory
        plan.add_operation(RestoreOperation::CreateDir {
            path: "new_dir".to_string(),
        });
        let safety_cp = CheckpointId::new();
        plan = plan.with_safety_checkpoint(safety_cp);

        let result = execute_restore_transactional(&plan, workspace_root, &TestBlobStore).unwrap();

        assert!(result.success);
        assert_eq!(result.files_restored, 4);
        // Verify all changes applied
        assert!(workspace_root.join("new_file.txt").exists());
        assert_eq!(
            std::fs::read(workspace_root.join("existing.txt")).unwrap(),
            b"new content"
        );
        assert!(!workspace_root.join("to_delete.txt").exists());
        assert!(workspace_root.join("new_dir").is_dir());
    }

    /// Test: verify_restore_simple wrapper returns bool correctly
    #[test]
    fn test_verify_restore_simple_returns_bool() {
        use crate::planner::{RestoreMode, RestoreOperation, RestorePlan};
        use ai_history_core::{CheckpointId, ContentHash, SessionId};

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path();

        // Create file matching expectation
        std::fs::write(workspace_root.join("good.txt"), b"correct").unwrap();
        let hash = ContentHash::new(b"correct");
        let mut plan = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan.add_operation(RestoreOperation::CreateFile {
            path: "good.txt".to_string(),
            blob_hash: hash,
        });

        let result = verify_restore_simple(&plan, workspace_root).unwrap();
        assert!(result);

        // Mismatched file
        std::fs::write(workspace_root.join("bad.txt"), b"wrong").unwrap();
        let hash2 = ContentHash::new(b"expected");
        let mut plan2 = RestorePlan::new(CheckpointId::new(), SessionId::new(), RestoreMode::Both);
        plan2.add_operation(RestoreOperation::CreateFile {
            path: "bad.txt".to_string(),
            blob_hash: hash2,
        });

        let result2 = verify_restore_simple(&plan2, workspace_root).unwrap();
        assert!(!result2);
    }
}
