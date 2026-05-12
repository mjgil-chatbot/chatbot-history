use ai_history_cli::OutputFormatter;
use ai_history_core::{BranchId, TurnId};
use ai_history_store::sqlite_store::{BranchInfo, SqliteStore, TurnInfo};

fn active_session_and_main_branch(
    store: &mut SqliteStore,
) -> Result<(ai_history_core::SessionId, BranchInfo), String> {
    let sessions = store.list_sessions().map_err(|e| e.to_string())?;
    let session = sessions
        .first()
        .ok_or("No active session. Run 'init' first.")?;

    let branches = store.list_branches(session.id).map_err(|e| e.to_string())?;
    let branch = branches
        .iter()
        .find(|b| b.name == "main")
        .ok_or("No main branch found")?;

    Ok((session.id, branch.clone()))
}

pub(super) fn navigate_back(count: Option<u32>, store: &mut SqliteStore) -> Result<String, String> {
    let steps = count.unwrap_or(1) as usize;
    let (session_id, branch) = active_session_and_main_branch(store)?;

    let current_cursor = store
        .get_cursor(session_id, branch.id)
        .map_err(|e| e.to_string())?;
    let current_turn = current_cursor
        .or(branch.head_turn_id)
        .ok_or("No turn to navigate back from")?;

    let mut new_cursor = current_turn;
    for _ in 0..steps {
        let turn = store
            .get_turn(new_cursor)
            .map_err(|e| e.to_string())?
            .ok_or("Turn not found")?;
        match turn.parent_turn_id {
            Some(parent) => new_cursor = parent,
            None => break,
        }
    }

    if new_cursor == branch.head_turn_id.unwrap() {
        store
            .set_cursor(session_id, branch.id, branch.head_turn_id.unwrap())
            .map_err(|e| e.to_string())?;
        Ok(format!(
            "Returned to branch head (navigated back {} turn(s))",
            steps
        ))
    } else {
        store
            .set_cursor(session_id, branch.id, new_cursor)
            .map_err(|e| e.to_string())?;
        Ok(format!(
            "Navigated back {} turn(s) to {}",
            steps, new_cursor
        ))
    }
}

pub(super) fn navigate_forward(
    count: Option<u32>,
    store: &mut SqliteStore,
) -> Result<String, String> {
    let steps = count.unwrap_or(1) as usize;
    let (session_id, branch) = active_session_and_main_branch(store)?;

    let current_cursor = store
        .get_cursor(session_id, branch.id)
        .map_err(|e| e.to_string())?;
    let current_turn = current_cursor
        .or(branch.head_turn_id)
        .ok_or("No turn to navigate forward from")?;

    let mut new_cursor = current_turn;
    for _ in 0..steps {
        let children = store
            .get_turn_forward_children(new_cursor)
            .map_err(|e| e.to_string())?;
        match children.len() {
            0 => break,
            1 => new_cursor = children[0],
            _ => {
                return Ok(format!(
                    "Ambiguous forward path at {} ({} futures). Use a specific turn ID to disambiguate.",
                    new_cursor,
                    children.len()
                ));
            }
        }
    }

    if new_cursor == current_turn {
        Ok(format!("No forward path from {}", current_turn))
    } else {
        store
            .set_cursor(session_id, branch.id, new_cursor)
            .map_err(|e| e.to_string())?;
        Ok(format!("Navigated forward to {}", new_cursor))
    }
}

pub(super) fn fork_current(name: &str, store: &mut SqliteStore) -> Result<String, String> {
    let (session_id, main_branch) = active_session_and_main_branch(store)?;
    let fork_branch_id = BranchId::new();
    let branch_info = BranchInfo {
        id: fork_branch_id,
        session_id,
        name: name.to_string(),
        parent_branch_id: Some(main_branch.id),
        created_from_turn_id: None,
        ancestry: Vec::new(),
        protected: false,
        archived: false,
        abandoned_future: false,
        head_turn_id: main_branch.head_turn_id,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store
        .create_branch(&branch_info)
        .map_err(|e| e.to_string())?;

    Ok(OutputFormatter::format_success(&format!(
        "Forked branch '{}' from main at turn {}",
        name,
        main_branch
            .head_turn_id
            .as_ref()
            .map(|turn| turn.to_string())
            .unwrap_or_else(|| "none".to_string())
    )))
}

pub(super) fn fork_from_cursor(
    turn_id: &str,
    name: &str,
    store: &mut SqliteStore,
) -> Result<String, String> {
    let cursor_turn_id: TurnId = turn_id
        .parse()
        .map_err(|_| format!("Invalid turn ID: {}", turn_id))?;
    let turn_info = store
        .get_turn(cursor_turn_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Turn not found: {}", turn_id))?;

    let fork_branch_id = BranchId::new();
    let branch_info = BranchInfo {
        id: fork_branch_id,
        session_id: turn_info.session_id,
        name: name.to_string(),
        parent_branch_id: Some(turn_info.branch_id),
        created_from_turn_id: None,
        ancestry: Vec::new(),
        protected: false,
        archived: false,
        abandoned_future: false,
        head_turn_id: Some(cursor_turn_id),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store
        .create_branch(&branch_info)
        .map_err(|e| e.to_string())?;

    Ok(OutputFormatter::format_success(&format!(
        "Forked from cursor {} (branch {}) as '{}'",
        turn_id, turn_info.branch_id, name
    )))
}

pub(super) fn continue_from_cursor(
    turn_id: &str,
    _prompt: &str,
    store: &mut SqliteStore,
) -> Result<String, String> {
    let cursor_turn_id: TurnId = turn_id
        .parse()
        .map_err(|_| format!("Invalid turn ID: {}", turn_id))?;
    let cursor_turn = store
        .get_turn(cursor_turn_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Turn not found: {}", turn_id))?;

    let cursor_suffix = turn_id.split(':').last().unwrap_or("cursor");
    let new_turn_id_for_name = TurnId::new().to_string();
    let new_id_short = new_turn_id_for_name.split(':').last().unwrap_or("new");
    let branch_name = format!("continue-{}-{}", cursor_suffix, new_id_short);

    let new_branch_id = BranchId::new();
    let new_turn_id = TurnId::new();
    let branch_info = BranchInfo {
        id: new_branch_id,
        session_id: cursor_turn.session_id,
        name: branch_name.clone(),
        parent_branch_id: Some(cursor_turn.branch_id),
        created_from_turn_id: None,
        ancestry: Vec::new(),
        protected: false,
        archived: false,
        abandoned_future: false,
        head_turn_id: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store
        .create_branch(&branch_info)
        .map_err(|e| e.to_string())?;

    let turn_info = TurnInfo {
        id: new_turn_id,
        session_id: cursor_turn.session_id,
        branch_id: new_branch_id,
        parent_turn_id: Some(cursor_turn_id),
        actor: "User".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        finalized_at: None,
    };
    store.create_turn(&turn_info).map_err(|e| e.to_string())?;
    store
        .update_branch_head(new_branch_id, new_turn_id)
        .map_err(|e| e.to_string())?;

    Ok(OutputFormatter::format_success(&format!(
        "Continued from turn {} into new branch '{}' with new turn {}",
        turn_id, branch_name, new_turn_id
    )))
}
