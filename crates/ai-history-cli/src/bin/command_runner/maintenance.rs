use ai_history_core::export::BranchExport;
use ai_history_core::export::CheckpointExport;
use ai_history_core::export::EventExport;
use ai_history_core::export::ExportPackage;
use ai_history_core::export::Exporter;
use ai_history_core::export::SessionExport;
use ai_history_core::recovery::{RecoveryExecutor, RecoveryReport, RecoveryResult};
use ai_history_core::{BranchId, CheckpointId};
use ai_history_store::sqlite_store::SqliteStore;
use ai_history_store::StoreIntegrityScanner;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
pub(super) fn doctor(fix: bool, store: &mut SqliteStore) -> Result<String, String> {
    let mut scanner = StoreIntegrityScanner::new(store);
    let report = scanner
        .scan()
        .map_err(|e| format!("Integrity scan failed: {}", e))?;

    if report.issues.is_empty() {
        Ok("Store integrity: OK. No issues found.".to_string())
    } else {
        let issues_text = format!("{} issues found", report.issues.len());
        let summary_text = format!(
            "Sessions: {}, Branches: {}, Checkpoints: {}, Turns: {}, Blobs: {}, Manifests: {}",
            report.scanned_sessions,
            report.scanned_branches,
            report.scanned_checkpoints,
            report.scanned_turns,
            report.scanned_blobs,
            report.scanned_manifests
        );
        Err(format!(
            "Store integrity problems:\n  {}\n  {}\n\n{} (auto-fix not yet implemented)",
            issues_text,
            summary_text,
            if fix { "Fix mode" } else { "Dry-run mode" }
        ))
    }
}
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
pub(super) fn verify(
    checkpoint_id: Option<&str>,
    store: &mut SqliteStore,
) -> Result<String, String> {
    let mut scanner = StoreIntegrityScanner::new(store);
    let result = scanner.scan().map_err(|e| e.to_string())?;

    match checkpoint_id {
        Some(checkpoint_id) => {
            let cp_id: CheckpointId = checkpoint_id
                .parse()
                .map_err(|_| format!("Invalid checkpoint ID: {}", checkpoint_id))?;

            let checkpoint_issues: Vec<_> = result
                .issues
                .iter()
                .filter(|issue| {
                    issue
                        .entity_id
                        .as_ref()
                        .map(|entity_id| entity_id.contains(checkpoint_id))
                        .unwrap_or(false)
                })
                .collect();

            if checkpoint_issues.is_empty() && result.passed {
                Ok(format!("Checkpoint {} verified successfully.", cp_id))
            } else if !checkpoint_issues.is_empty() {
                let message = checkpoint_issues
                    .iter()
                    .map(|issue| {
                        format!(
                            "  [{}] {}",
                            format!("{:?}", issue.check_type).to_uppercase(),
                            issue.description
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Err(format!(
                    "Checkpoint {} verification failed:\n{}",
                    cp_id, message
                ))
            } else {
                Err(format!(
                    "Checkpoint {} verification failed (store integrity issues present).",
                    cp_id
                ))
            }
        }
        None => {
            if result.passed {
                Ok(format!(
                    "Store integrity verification passed.\nScanned: {} sessions, {} branches, {} checkpoints, {} manifests",
                    result.scanned_sessions,
                    result.scanned_branches,
                    result.scanned_checkpoints,
                    result.scanned_manifests
                ))
            } else {
                let error_count = result
                    .issues
                    .iter()
                    .filter(|issue| matches!(issue.severity, ai_history_core::IssueSeverity::Error))
                    .count();
                let warning_count = result
                    .issues
                    .iter()
                    .filter(|issue| {
                        matches!(issue.severity, ai_history_core::IssueSeverity::Warning)
                    })
                    .count();
                Err(format!(
                    "Store integrity verification FAILED: {} errors, {} warnings.\nIssues:\n{}",
                    error_count,
                    warning_count,
                    result
                        .issues
                        .iter()
                        .map(|issue| format!(
                            "  [{}] {}",
                            format!("{:?}", issue.check_type).to_uppercase(),
                            issue.description
                        ))
                        .collect::<Vec<_>>()
                        .join("\n")
                ))
            }
        }
    }
}
