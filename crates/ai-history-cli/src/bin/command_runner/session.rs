use ai_history_cli::OutputFormatter;
use ai_history_core::BranchId;
use ai_history_core::SessionId;
use ai_history_store::sqlite_store::BranchInfo;
use ai_history_store::sqlite_store::SqliteStore;
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
