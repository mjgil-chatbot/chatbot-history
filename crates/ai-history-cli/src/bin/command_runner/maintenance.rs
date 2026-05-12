use ai_history_core::export::BranchExport;
use ai_history_core::export::CheckpointExport;
use ai_history_core::export::EventExport;
use ai_history_core::export::ExportPackage;
use ai_history_core::export::Exporter;
use ai_history_core::export::SessionExport;
use ai_history_core::recovery::{RecoveryExecutor, RecoveryReport, RecoveryResult};
use ai_history_core::BranchId;
use ai_history_store::sqlite_store::SqliteStore;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
pub(super) fn recover(execute: bool, store: &mut SqliteStore) -> Result<String, String> {
    let sessions = store.list_sessions().map_err(|e| e.to_string())?;
    if sessions.is_empty() {
        return Ok("No sessions found. Nothing to recover.".to_string());
    }

    let mut report = RecoveryReport::new();
    for session in &sessions {
        let states = store
            .detect_recovery_state(session.id)
            .map_err(|e| format!("Recovery detection failed: {}", e))?;

        for state in states {
            let result = if execute {
                RecoveryExecutor::execute(&state)
            } else {
                match state.recovery_action() {
                    ai_history_core::recovery::RecoveryAction::UserConfirmation => {
                        RecoveryResult::RequiresConfirmation("dry-run".to_string())
                    }
                    _ => RecoveryResult::Success,
                }
            };
            report.add_result(state, result);
        }
    }

    report.finalize();
    if report.all_successful {
        if execute {
            Ok("Recovery completed successfully.".to_string())
        } else {
            Ok("Recovery would succeed. No issues found.".to_string())
        }
    } else {
        let confirmation_count = report.confirmation_count();
        if confirmation_count > 0 {
            Err(format!(
                "Recovery requires {} confirmation(s). Run with --execute to proceed.",
                confirmation_count
            ))
        } else {
            Err("Recovery failed. Run with --execute to attempt recovery.".to_string())
        }
    }
}
pub(super) fn export(
    output: &str,
    _include_blobs: bool,
    store: &mut SqliteStore,
) -> Result<String, String> {
    let sessions = store
        .list_sessions()
        .map_err(|e| format!("Failed to list sessions: {}", e))?;

    let mut branches = Vec::new();
    let mut checkpoints = Vec::new();
    let manifests = Vec::new();
    let mut events = Vec::new();
    let blobs = Vec::new();

    for session in &sessions {
        let session_branches = store
            .list_branches(session.id)
            .map_err(|e| format!("Failed to list branches: {}", e))?;
        for branch in session_branches {
            branches.push(BranchExport {
                id: branch.id,
                session_id: branch.session_id,
                name: branch.name,
                parent_branch_id: branch.parent_branch_id,
                head_turn_id: branch.head_turn_id,
                created_at: branch.created_at,
            });
        }

        let session_checkpoints = store
            .list_checkpoints(session.id)
            .map_err(|e| format!("Failed to list checkpoints: {}", e))?;
        for checkpoint in session_checkpoints {
            checkpoints.push(CheckpointExport {
                id: checkpoint.id,
                session_id: checkpoint.session_id,
                branch_id: checkpoint.branch_id,
                turn_id: checkpoint.turn_id,
                manifest_id: checkpoint.manifest_id,
                reason: checkpoint.reason,
                created_at: checkpoint.created_at,
            });
        }

        let session_events = store
            .get_events_since(session.id, 0)
            .map_err(|e| format!("Failed to get events: {}", e))?;
        for (index, event) in session_events.iter().enumerate() {
            events.push(EventExport {
                id: index,
                session_id: session.id,
                branch_id: BranchId::default(),
                turn_id: event.get_turn_id(),
                sequence_num: event.sequence(),
                event_type: format!("{:?}", event.event_type()),
                event_data: serde_json::to_string(event).unwrap_or_default(),
                causation_id: None,
                correlation_id: None,
                actor: "unknown".to_string(),
                checksum: event.checksum(),
                created_at: chrono::Utc::now().to_rfc3339(),
            });
        }
    }

    let exporter = Exporter::new();
    let metadata = exporter.create_metadata(
        sessions
            .first()
            .map(|session| session.id)
            .unwrap_or_default(),
        sessions.len(),
        events.len(),
        branches.len(),
        checkpoints.len(),
        manifests.len(),
        blobs.len(),
    );

    let package = ExportPackage {
        metadata,
        sessions: sessions
            .iter()
            .map(|session| SessionExport {
                id: session.id,
                workspace_root: session.workspace_root.clone(),
                created_at: session.created_at.clone(),
                updated_at: session.updated_at.clone(),
            })
            .collect(),
        branches,
        checkpoints,
        manifests,
        events,
        blobs,
    };

    let json = serde_json::to_string_pretty(&package)
        .map_err(|e| format!("Failed to serialize export: {}", e))?;
    let mut file =
        File::create(output).map_err(|e| format!("Failed to create output file: {}", e))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write export file: {}", e))?;

    Ok(format!(
        "Exported {} sessions, {} branches, {} checkpoints, {} events to {}",
        package.sessions.len(),
        package.branches.len(),
        package.checkpoints.len(),
        package.events.len(),
        output
    ))
}
