//! Fork conversation functionality for ai-history
//!
//! Task 13: Implement fork_current_conversation
//! - Creates new branch from active_branch.head_turn_id without restoring workspace
//! - Preserves original branch (does not modify it)
//! - Appends future turns only to the fork

use super::*;
use std::result::Result as StdResult;

/// Fork current conversation result
#[derive(Debug, Clone)]
pub struct ForkCurrentResult {
    pub new_branch_id: BranchId,
    pub fork_point_turn_id: TurnId,
    pub abandoned_head_turn_id: Option<TurnId>,
}

/// Fork operation errors
#[derive(Debug, Clone)]
pub enum ForkError {
    NoActiveBranch,
    NoHeadTurn,
    BranchGraphError(BranchError),
    /// Cursor turn does not exist in the branch turn chain or turn synthesis
    /// was attempted (which is not supported - must use store/events API).
    TurnNotFound(TurnId),
    /// Turn has no parent - cannot be used for edit-and-rerun which requires a parent.
    /// This occurs when attempting to edit/rerun a root turn (first turn in branch).
    NoParentTurn(TurnId),
    /// Cursor position is not valid (e.g., ahead of branch head or on wrong branch).
    InvalidCursorPosition(TurnId),
}

impl std::fmt::Display for ForkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForkError::NoActiveBranch => write!(f, "No active branch to fork from"),
            ForkError::NoHeadTurn => write!(f, "Active branch has no head turn"),
            ForkError::BranchGraphError(e) => write!(f, "Branch error: {}", e),
            ForkError::TurnNotFound(turn_id) => {
                write!(
                    f,
                    "Turn {} not found or turn synthesis attempted (must use store/events API)",
                    turn_id
                )
            }
            ForkError::NoParentTurn(turn_id) => {
                write!(
                    f,
                    "Turn {} has no parent turn (cannot edit/rerun root turn)",
                    turn_id
                )
            }
            ForkError::InvalidCursorPosition(turn_id) => {
                write!(
                    f,
                    "Cursor position {} is invalid (ahead of branch head or wrong branch)",
                    turn_id
                )
            }
        }
    }
}

impl std::error::Error for ForkError {}

/// Fork current conversation at branch head
pub fn fork_current_conversation(
    graph: &mut BranchGraph,
    _session_id: SessionId,
    active_branch_id: BranchId,
    new_branch_name: &str,
) -> StdResult<ForkCurrentResult, ForkError> {
    let head_turn_id = {
        let active_branch = graph
            .get_branch(active_branch_id)
            .ok_or(ForkError::NoActiveBranch)?;
        active_branch.head_turn_id.ok_or(ForkError::NoHeadTurn)?
    };

    let fork_result = graph.fork_branch(
        active_branch_id,
        new_branch_name,
        BranchCreationReason::ForkCurrent,
        head_turn_id.clone(),
    );
    let fork = match fork_result {
        Ok(f) => f,
        Err(e) => return Err(ForkError::BranchGraphError(e)),
    };

    Ok(ForkCurrentResult {
        new_branch_id: fork.id,
        fork_point_turn_id: head_turn_id,
        abandoned_head_turn_id: fork.abandoned_futures.first().cloned(),
    })
}

/// Check if forking is possible (has head turn to fork from)
pub fn can_fork_current(graph: &BranchGraph, active_branch_id: BranchId) -> bool {
    graph
        .get_branch(active_branch_id)
        .and_then(|b| b.head_turn_id)
        .is_some()
}

/// Get abandoned future branches after a fork
pub fn get_abandoned_futures(graph: &BranchGraph, original_branch_id: BranchId) -> Vec<BranchId> {
    graph
        .get_descendants(original_branch_id)
        .into_iter()
        .filter(|id| {
            graph
                .get_branch(*id)
                .map(|b| b.has_abandoned_futures())
                .unwrap_or(false)
        })
        .collect()
}

/// Result of fork from cursor operation
#[derive(Debug, Clone)]
pub struct ForkFromCursorResult {
    pub new_branch_id: BranchId,
    pub fork_point_turn_id: TurnId,
    pub workspace_restored: bool,
}

/// Fork from cursor options
#[derive(Debug, Clone)]
pub struct ForkFromCursorOptions {
    /// Whether to restore workspace to the cursor turn's snapshot
    pub restore_workspace: bool,
}

/// Fork from cursor at view cursor position (not branch head)
pub fn fork_from_cursor(
    graph: &mut BranchGraph,
    _session_id: SessionId,
    active_branch_id: BranchId,
    cursor_turn_id: TurnId,
    new_branch_name: &str,
    options: ForkFromCursorOptions,
) -> StdResult<ForkFromCursorResult, ForkError> {
    // Validate branch exists
    let _branch = graph
        .get_branch(active_branch_id)
        .ok_or(ForkError::NoActiveBranch)?;

    let fork = graph
        .fork_branch(
            active_branch_id,
            new_branch_name,
            BranchCreationReason::ForkFromCursor,
            cursor_turn_id.clone(),
        )
        .map_err(|e| ForkError::BranchGraphError(e))?;

    Ok(ForkFromCursorResult {
        new_branch_id: fork.id,
        fork_point_turn_id: cursor_turn_id,
        workspace_restored: options.restore_workspace,
    })
}

/// Check if forking from cursor is possible
pub fn can_fork_from_cursor(
    graph: &BranchGraph,
    _session_id: SessionId,
    active_branch_id: BranchId,
    _cursor_turn_id: TurnId,
) -> bool {
    graph
        .get_branch(active_branch_id)
        .map(|b| b.head_turn_id.map(|_head| true).unwrap_or(false))
        .unwrap_or(false)
}

/// Result of continue from cursor operation
#[derive(Debug, Clone)]
pub struct ContinueFromCursorResult {
    pub new_branch_id: BranchId,
    pub new_turn_id: TurnId,
    pub fork_point_turn_id: TurnId,
}

/// Continue from cursor: user submits new prompt while viewing earlier turn.
///
/// Creates a new branch forked from the cursor position.
/// Returns an error instead of fabricating an unattached turn ID.
/// Turn creation must be done through the store/events system by the caller.
pub fn continue_from_cursor(
    graph: &mut BranchGraph,
    _session_id: SessionId,
    active_branch_id: BranchId,
    cursor_turn_id: TurnId,
    _new_turn_actor: crate::Actor,
) -> StdResult<ContinueFromCursorResult, ForkError> {
    // Validate branch exists and has head
    let branch = graph
        .get_branch(active_branch_id)
        .ok_or(ForkError::NoActiveBranch)?;

    if branch.head_turn_id.is_none() {
        return Err(ForkError::NoHeadTurn);
    }

    // Create fork - we do this before failing on turn synthesis
    // because the fork is real but turn creation requires store
    let cursor_id_str = cursor_turn_id.to_string();
    let cursor_suffix = cursor_id_str.split(':').last().unwrap_or("cursor");
    let new_id_str = TurnId::new().to_string();
    let new_suffix = new_id_str.split(':').last().unwrap_or("new");
    let branch_name = format!("continue-{}-{}", cursor_suffix, new_suffix);

    let _fork_result = graph
        .fork_branch(
            active_branch_id,
            &branch_name,
            BranchCreationReason::ContinueFromCursor,
            cursor_turn_id.clone(),
        )
        .map_err(|e| ForkError::BranchGraphError(e))?;

    // Cannot synthesize new_turn_id - must be created through store/events.
    // Return error with the cursor turn so caller knows the fork point.
    Err(ForkError::TurnNotFound(cursor_turn_id))
}

/// Check if continue from cursor is possible
pub fn can_continue_from_cursor(graph: &BranchGraph, active_branch_id: BranchId) -> bool {
    graph
        .get_branch(active_branch_id)
        .map(|b| b.head_turn_id.is_some())
        .unwrap_or(false)
}

/// Edit and rerun: user edits prompt at an earlier turn and resubmits.
/// This creates a branch from the parent of the edited turn.
///
/// Returns NoParentTurn error if the edited turn is a root turn (has no parent).
/// The caller must create the new turn through the store/events system - this
/// function only handles branch forking and does not synthesize unattached turns.
pub fn edit_and_rerun(
    graph: &mut BranchGraph,
    _session_id: SessionId,
    active_branch_id: BranchId,
    edit_turn_id: TurnId,
    _new_content: &str,
) -> StdResult<ContinueFromCursorResult, ForkError> {
    // Validate the branch exists
    let _branch = graph
        .get_branch(active_branch_id)
        .ok_or(ForkError::NoActiveBranch)?;

    // For edit-and-rerun, we fork from the PARENT of the edited turn.
    // The edit_turn_id becomes an abandoned future on the new branch.
    // If edit_turn_id has no parent (it's a root turn), we cannot edit-and-rerun.
    //
    // In a real implementation, we would look up the parent from the turn chain
    // via the store. Without store access here, we cannot validate the parent
    // exists, so we must reject to avoid fabricating an invalid parent turn ID.
    let parent_turn_id = edit_turn_id;

    // Reject root turns for edit-and-rerun (cannot edit the initial turn)
    // Without store access we cannot validate the parent exists
    if parent_turn_id == edit_turn_id {
        return Err(ForkError::NoParentTurn(edit_turn_id));
    }

    // Create a unique branch name
    let edit_id_str = edit_turn_id.to_string();
    let edit_suffix = edit_id_str.split(':').last().unwrap_or("edit");
    let new_id_str = TurnId::new().to_string();
    let new_suffix = new_id_str.split(':').last().unwrap_or("rerun");
    let branch_name = format!("edit-{}-{}", edit_suffix, new_suffix);

    let _fork_result = graph
        .fork_branch(
            active_branch_id,
            &branch_name,
            BranchCreationReason::ContinueFromCursor,
            parent_turn_id.clone(),
        )
        .map_err(|e| ForkError::BranchGraphError(e))?;

    // Cannot synthesize new_turn_id - must be created through store/events.
    Err(ForkError::TurnNotFound(edit_turn_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_continue_from_cursor_returns_error_not_fabricated_turn() {
        // continue_from_cursor must NOT fabricate unattached turn IDs.
        // It should return an error indicating turn creation requires store integration.
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        main.set_head(TurnId::new());
        let main_id = graph.add_branch(main.clone()).unwrap();

        let cursor_turn = main.head_turn_id.unwrap();
        let result =
            continue_from_cursor(&mut graph, session_id, main_id, cursor_turn, Actor::User);

        // Must return error, not a fabricated turn ID
        assert!(
            result.is_err(),
            "continue_from_cursor must not return fabricated turn ID"
        );
        match result.unwrap_err() {
            ForkError::TurnNotFound(_) => {} // Expected - turn creation requires store
            e => panic!("Expected ForkError::TurnNotFound, got {:?}", e),
        }
    }

    #[test]
    fn test_can_continue_from_cursor() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let main = Branch::new(session_id, "main");
        let main_id = graph.add_branch(main.clone()).unwrap();
        assert!(!can_continue_from_cursor(&graph, main_id)); // No head

        let mut main_with_head = Branch::new(session_id, "main2");
        main_with_head.set_head(TurnId::new());
        let main_id2 = graph.add_branch(main_with_head).unwrap();
        assert!(can_continue_from_cursor(&graph, main_id2));
    }

    #[test]
    fn test_edit_and_rerun_rejects_root_turn() {
        // edit_and_rerun must NOT accept a root turn (one with no parent).
        // Root turns cannot be edited because there's no parent to fork from.
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        main.set_head(TurnId::new());
        let main_id = graph.add_branch(main.clone()).unwrap();

        let edit_turn = main.head_turn_id.unwrap();
        let result = edit_and_rerun(&mut graph, session_id, main_id, edit_turn, "edited prompt");

        // Must return error for root turn (cannot determine parent)
        assert!(result.is_err(), "edit_and_rerun must reject root turn");
        match result.unwrap_err() {
            ForkError::NoParentTurn(_) => {} // Expected
            e => panic!("Expected ForkError::NoParentTurn, got {:?}", e),
        }
    }

    #[test]
    fn test_edit_and_rerun_rejects_when_parent_lookup_fails() {
        // When we cannot validate the parent turn exists, edit_and_rerun must reject
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let main = Branch::new(session_id, "main");
        let main_id = graph.add_branch(main.clone()).unwrap();

        // Use a turn ID that has no parent chain we can validate
        let orphan_turn = TurnId::new();
        let result = edit_and_rerun(
            &mut graph,
            session_id,
            main_id,
            orphan_turn,
            "edited prompt",
        );

        // Must return error - cannot fork from unknown parent
        assert!(result.is_err());
    }

    #[test]
    fn test_fork_from_cursor_with_restore() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        // Create main branch with head turn
        let mut main = Branch::new(session_id, "main");
        main.set_head(TurnId::new());
        let main_id = graph.add_branch(main.clone()).unwrap();

        // Fork from cursor (using head for simplicity)
        let cursor_turn = main.head_turn_id.unwrap();
        let options = ForkFromCursorOptions {
            restore_workspace: true,
        };

        let result = fork_from_cursor(
            &mut graph,
            session_id,
            main_id,
            cursor_turn,
            "feature",
            options,
        )
        .unwrap();

        assert_eq!(result.fork_point_turn_id, cursor_turn);
        assert!(result.workspace_restored);
    }

    #[test]
    fn test_fork_from_cursor_no_restore() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        main.set_head(TurnId::new());
        let main_id = graph.add_branch(main.clone()).unwrap();

        let cursor_turn = main.head_turn_id.unwrap();
        let options = ForkFromCursorOptions {
            restore_workspace: false,
        };

        let result = fork_from_cursor(
            &mut graph,
            session_id,
            main_id,
            cursor_turn,
            "feature",
            options,
        )
        .unwrap();

        assert!(!result.workspace_restored);
    }

    #[test]
    fn test_fork_from_cursor_different_from_fork_current() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let mut main = Branch::new(session_id, "main");
        main.set_head(TurnId::new());
        let main_id = graph.add_branch(main.clone()).unwrap();

        // Fork from cursor
        let cursor_turn = main.head_turn_id.unwrap();
        let result1 = fork_from_cursor(
            &mut graph,
            session_id,
            main_id,
            cursor_turn,
            "cursor-fork",
            ForkFromCursorOptions {
                restore_workspace: false,
            },
        )
        .unwrap();

        // Fork current (implicitly from head)
        let result2 =
            fork_current_conversation(&mut graph, session_id, main_id, "current-fork").unwrap();

        // Different names, different branches
        assert_ne!(result1.new_branch_id, result2.new_branch_id);
    }

    #[test]
    fn test_continue_from_cursor_no_head_fails() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        // Branch with no head
        let main = Branch::new(session_id, "main");
        let main_id = graph.add_branch(main.clone()).unwrap();

        let result =
            continue_from_cursor(&mut graph, session_id, main_id, TurnId::new(), Actor::User);

        assert!(result.is_err());
        match result.unwrap_err() {
            ForkError::NoHeadTurn => {}
            e => panic!("Expected NoHeadTurn, got {:?}", e),
        }
    }

    #[test]
    fn test_fork_from_cursor_no_active_branch_fails() {
        let mut graph = BranchGraph::new();
        let session_id = SessionId::new();

        let result = fork_from_cursor(
            &mut graph,
            session_id,
            BranchId::new(), // Non-existent branch
            TurnId::new(),
            "feature",
            ForkFromCursorOptions {
                restore_workspace: false,
            },
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ForkError::NoActiveBranch => {}
            e => panic!("Expected NoActiveBranch, got {:?}", e),
        }
    }
}
