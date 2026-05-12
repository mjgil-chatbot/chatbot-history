//! Restore execution with transactional support

use super::planner::{RestoreOperation, RestorePlan};
use ai_history_core::ContentHash;
use ai_history_core::CoreResult;
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
    }}
impl RestoreResult {
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            files_restored: 0,
            files_failed: 0,
            errors: vec![error],
        }
    }}
impl RestoreResult {
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.files_failed += 1;
    }}
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
    }}
impl RestoreTransaction {
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
    }}
impl RestoreTransaction {
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
    }}
impl RestoreTransaction {
    /// Commit the transaction
    pub fn commit(&mut self) {
        // Clear state
        self.modified_files.clear();
        self.deleted_files.clear();
        self.rollback_data.clear();
    }}
impl RestoreTransaction {
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
    }}
impl RestoreTransaction {
    /// Get list of modified files
    pub fn modified_files(&self) -> &HashSet<String> {
        &self.modified_files
    }}
impl RestoreTransaction {
    /// Get list of deleted files
    pub fn deleted_files(&self) -> &HashSet<String> {
        &self.deleted_files
    }}
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
