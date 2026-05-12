use ai_history_cli::{OutputFormatter, RestoreModeCli};
use ai_history_core::{BranchId, CheckpointId, SessionId, WorkspacePath};
use ai_history_restore::{RestoreMode as RestoreModeCore, RestoreOperation, RestorePlan};
use ai_history_snapshot::{three_way_compare, Manifest};
use ai_history_store::sqlite_store::{BranchInfo, CheckpointInfo, SqliteStore};
use std::path::PathBuf;

pub(super) fn init(
    title: &Option<String>,
    workspace: &PathBuf,
    store: &mut SqliteStore,
) -> Result<String, String> {
    let session_id = SessionId::new();
    store
        .create_session(session_id, workspace.to_str().unwrap_or("."))
        .map_err(|e| e.to_string())?;

    let branch_id = BranchId::new();
    let branch_info = BranchInfo {
        id: branch_id,
        session_id,
        name: "main".to_string(),
        parent_branch_id: None,
        created_from_turn_id: None,
        head_turn_id: None,
        ancestry: Vec::new(),
        protected: false,
        archived: false,
        abandoned_future: false,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store
        .create_branch(&branch_info)
        .map_err(|e| e.to_string())?;

    let title_str = title.clone().unwrap_or_else(|| "untitled".to_string());
    Ok(OutputFormatter::format_success(&format!(
        "Initialized session {} with main branch {}",
        session_id, title_str
    )))
}

pub(super) fn checkpoint(
    name: &Option<String>,
    reason: &str,
    workspace: &PathBuf,
    store: &mut SqliteStore,
) -> Result<String, String> {
    let sessions = store.list_sessions().map_err(|e| e.to_string())?;
    let session = sessions
        .first()
        .ok_or("No active session. Run 'init' first.")?;

    let branches = store.list_branches(session.id).map_err(|e| e.to_string())?;
    let branch = branches
        .iter()
        .find(|b| b.name == "main")
        .ok_or("No main branch found")?;

    use ai_history_snapshot::scanner::{Scanner, ScannerConfig};
    let scanner = Scanner::new(ScannerConfig::default());
    let ws_path = WorkspacePath::from_string(workspace.to_str().unwrap_or("."));

    let mut blob_store_ref: Option<&mut SqliteStore> = Some(store);
    let manifest = scanner
        .scan_with_blob_store(&ws_path, &mut blob_store_ref)
        .map_err(|e| format!("Failed to scan workspace: {}", e))?;

    let manifest_id = manifest.id;
    let file_count = manifest.entries.len();
    let manifest_data = serde_json::to_string(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

    let checkpoint_id = CheckpointId::new();
    let checkpoint_info = CheckpointInfo {
        id: checkpoint_id,
        session_id: session.id,
        branch_id: branch.id,
        turn_id: branch.head_turn_id,
        manifest_id,
        reason: reason.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    store
        .create_checkpoint(&checkpoint_info)
        .map_err(|e| e.to_string())?;
    store
        .create_manifest(&ai_history_store::ManifestInfo {
            id: manifest_id,
            session_id: session.id,
            checkpoint_id,
            manifest_data,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
        .map_err(|e| e.to_string())?;

    let name_str = name.as_deref().unwrap_or("unnamed");
    Ok(OutputFormatter::format_success(&format!(
        "Created checkpoint {} ({}): {} files scanned",
        checkpoint_id, name_str, file_count
    )))
}

pub(super) fn list_checkpoints(_branch: &str, store: &mut SqliteStore) -> Result<String, String> {
    let sessions = store.list_sessions().map_err(|e| e.to_string())?;
    let session = sessions
        .first()
        .ok_or("No active session. Run 'init' first.")?;

    let checkpoints = store
        .list_checkpoints(session.id)
        .map_err(|e| e.to_string())?;

    if checkpoints.is_empty() {
        return Ok("No checkpoints found.".to_string());
    }

    let mut output = String::new();
    for checkpoint in checkpoints {
        output.push_str(&OutputFormatter::format_checkpoint(
            checkpoint.id,
            &checkpoint.reason,
            0,
            chrono::DateTime::parse_from_rfc3339(&checkpoint.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            Some(&format!(
                "{}:{}",
                checkpoint.branch_id,
                checkpoint
                    .turn_id
                    .map(|turn| turn.to_string())
                    .unwrap_or_default()
            )),
        ));
        output.push('\n');
    }
    Ok(output)
}

pub(super) fn restore(
    checkpoint_id: &str,
    mode: &RestoreModeCli,
    workspace: &PathBuf,
    store: &mut SqliteStore,
) -> Result<String, String> {
    let cp_id: CheckpointId = checkpoint_id
        .parse()
        .map_err(|_| format!("Invalid checkpoint ID: {}", checkpoint_id))?;

    let checkpoint = store
        .get_checkpoint(cp_id)
        .map_err(|e| e.to_string())?
        .ok_or("Checkpoint not found")?;

    let session = store
        .get_session(checkpoint.session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", checkpoint.session_id))?;

    let manifest_info = store
        .get_manifest(checkpoint.manifest_id)
        .map_err(|e| e.to_string())?
        .ok_or(format!(
            "Manifest {} not found for checkpoint {}",
            checkpoint.manifest_id, checkpoint_id
        ))?;

    let target_manifest: Manifest = serde_json::from_str(&manifest_info.manifest_data)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;

    let restore_mode = match mode {
        RestoreModeCli::ConversationOnly => RestoreModeCore::ConversationOnly,
        RestoreModeCli::WorkspaceOnly => RestoreModeCore::WorkspaceOnly,
        RestoreModeCli::Both => RestoreModeCore::Both,
        RestoreModeCli::Fork => RestoreModeCore::Fork,
    };

    let mut plan = RestorePlan::new(cp_id, checkpoint.session_id, restore_mode);
    for (path, entry) in &target_manifest.entries {
        plan.add_operation(RestoreOperation::UpdateFile {
            path: path.clone(),
            blob_hash: entry.blob_hash.clone(),
        });
    }

    use ai_history_snapshot::scanner::{Scanner, ScannerConfig};
    let safety_cp_id = CheckpointId::new();
    let safety_manifest_id = ai_history_core::ManifestId::new();
    let scanner = Scanner::new(ScannerConfig::default());
    let ws_path = WorkspacePath::from_string(workspace.to_str().unwrap_or("."));
    let mut blob_store_ref: Option<&mut SqliteStore> = Some(store);
    let current_manifest = scanner
        .scan_with_blob_store(&ws_path, &mut blob_store_ref)
        .map_err(|e| format!("Failed to scan workspace for safety checkpoint: {}", e))?;

    let safety_checkpoint_info = CheckpointInfo {
        id: safety_cp_id,
        session_id: session.id,
        branch_id: checkpoint.branch_id,
        turn_id: checkpoint.turn_id,
        manifest_id: safety_manifest_id,
        reason: "safety checkpoint before restore".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store
        .create_checkpoint(&safety_checkpoint_info)
        .map_err(|e| e.to_string())?;

    let safety_manifest_data = serde_json::to_string(&current_manifest)
        .map_err(|e| format!("Failed to serialize safety manifest: {}", e))?;
    store
        .create_manifest(&ai_history_store::ManifestInfo {
            id: safety_manifest_id,
            session_id: session.id,
            checkpoint_id: safety_cp_id,
            manifest_data: safety_manifest_data,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
        .map_err(|e| e.to_string())?;
    plan.safety_checkpoint_id = Some(safety_cp_id);

    let result = ai_history_restore::execute_restore_transactional(&plan, workspace, &*store)
        .map_err(|e| e.to_string())?;

    if result.success {
        Ok(OutputFormatter::format_success(&format!(
            "Restored checkpoint {} ({} files from {} manifest)",
            checkpoint_id,
            target_manifest.entries.len(),
            target_manifest.id
        )))
    } else {
        Err(format!(
            "Restore failed for checkpoint {}: {}",
            checkpoint_id,
            result.errors.join("; ")
        ))
    }
}

pub(super) fn fork(
    checkpoint_id: &str,
    name: &str,
    store: &mut SqliteStore,
) -> Result<String, String> {
    let sessions = store.list_sessions().map_err(|e| e.to_string())?;
    let session = sessions
        .first()
        .ok_or("No active session. Run 'init' first.")?;

    let cp_id: CheckpointId = checkpoint_id
        .parse()
        .map_err(|_| format!("Invalid checkpoint ID: {}", checkpoint_id))?;
    let checkpoint = store
        .get_checkpoint(cp_id)
        .map_err(|e| e.to_string())?
        .ok_or("Checkpoint not found")?;

    let branch_id = BranchId::new();
    let branch_info = BranchInfo {
        id: branch_id,
        session_id: session.id,
        name: name.to_string(),
        parent_branch_id: Some(checkpoint.branch_id),
        created_from_turn_id: None,
        ancestry: Vec::new(),
        protected: false,
        archived: false,
        abandoned_future: false,
        head_turn_id: checkpoint.turn_id,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store
        .create_branch(&branch_info)
        .map_err(|e| e.to_string())?;

    Ok(OutputFormatter::format_success(&format!(
        "Forked branch '{}' from checkpoint {} (branch {}, turn {:?})",
        name, checkpoint_id, checkpoint.branch_id, checkpoint.turn_id
    )))
}

pub(super) fn branches(store: &mut SqliteStore) -> Result<String, String> {
    let sessions = store.list_sessions().map_err(|e| e.to_string())?;
    let session = sessions
        .first()
        .ok_or("No active session. Run 'init' first.")?;
    let branches = store.list_branches(session.id).map_err(|e| e.to_string())?;

    if branches.is_empty() {
        return Ok("No branches found.".to_string());
    }

    let mut output = String::new();
    for branch in branches {
        output.push_str(&OutputFormatter::format_branch(
            branch.id,
            &branch.name,
            branch
                .head_turn_id
                .as_ref()
                .map(|turn| turn.to_string())
                .as_deref(),
            chrono::DateTime::parse_from_rfc3339(&branch.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            false,
        ));
        output.push('\n');
    }
    Ok(output)
}

pub(super) fn diff(
    checkpoint_id: &str,
    files_only: bool,
    workspace: &PathBuf,
    store: &mut SqliteStore,
) -> Result<String, String> {
    let cp_id: CheckpointId = checkpoint_id
        .parse()
        .map_err(|_| format!("Invalid checkpoint ID: {}", checkpoint_id))?;
    let checkpoint = store
        .get_checkpoint(cp_id)
        .map_err(|e| e.to_string())?
        .ok_or("Checkpoint not found")?;

    let manifest_info = store
        .get_manifest(checkpoint.manifest_id)
        .map_err(|e| e.to_string())?
        .ok_or("Manifest not found")?;
    let target_manifest: Manifest = serde_json::from_str(&manifest_info.manifest_data)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;

    use ai_history_snapshot::scanner::{Scanner, ScannerConfig};
    let scanner = Scanner::new(ScannerConfig::default());
    let ws_path = WorkspacePath::from_string(workspace.to_str().unwrap_or("."));
    let current_manifest = scanner
        .scan(&ws_path)
        .map_err(|e| format!("Failed to scan workspace: {}", e))?;

    let diff = three_way_compare(&target_manifest, &current_manifest, &target_manifest);

    if files_only {
        let mut output = String::new();
        if !diff.added.is_empty() {
            output.push_str(&format!("Added: {}\n", diff.added.join(", ")));
        }
        if !diff.modified.is_empty() {
            output.push_str(&format!("Modified: {}\n", diff.modified.join(", ")));
        }
        if !diff.deleted.is_empty() {
            output.push_str(&format!("Deleted: {}\n", diff.deleted.join(", ")));
        }
        if diff.added.is_empty() && diff.modified.is_empty() && diff.deleted.is_empty() {
            output.push_str("No changes\n");
        }
        Ok(output)
    } else {
        let mut output = format!("Diff against checkpoint {}:\n", checkpoint_id);
        output.push_str(&format!("  Added: {}\n", diff.added.len()));
        output.push_str(&format!("  Modified: {}\n", diff.modified.len()));
        output.push_str(&format!("  Deleted: {}\n", diff.deleted.len()));
        output.push_str(&format!("  Unchanged: {}\n", diff.unchanged.len()));
        if diff.has_conflicts() {
            output.push_str(&format!("  Conflicts: {}\n", diff.conflicts.len()));
        }
        Ok(output)
    }
}

pub(super) fn status(store: &mut SqliteStore) -> Result<String, String> {
    let sessions = store.list_sessions().map_err(|e| e.to_string())?;
    let session = sessions
        .first()
        .ok_or("No active session. Run 'init' first.")?;

    let branches = store.list_branches(session.id).map_err(|e| e.to_string())?;
    let checkpoints = store
        .list_checkpoints(session.id)
        .map_err(|e| e.to_string())?;

    Ok(format!(
        "Session: {}\nWorkspace: {}\nBranches: {}\nCheckpoints: {}",
        session.id,
        session.workspace_root,
        branches.len(),
        checkpoints.len()
    ))
}
