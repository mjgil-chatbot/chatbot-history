use super::*;

#[test]
fn test_navigate_back_preserves_branch_head() {
    let mut ctx = TestContext::new();
    let session_id = ctx.create_session();
    let main_branch_id = ctx.create_main_branch(session_id);
    let t1 = ctx.create_turn(
        session_id,
        main_branch_id,
        None,
        "User",
        "2024-01-01T00:00:00Z",
    );
    let t2 = ctx.create_turn(
        session_id,
        main_branch_id,
        Some(t1),
        "Assistant",
        "2024-01-01T00:00:01Z",
    );
    let t3 = ctx.create_turn(
        session_id,
        main_branch_id,
        Some(t2),
        "User",
        "2024-01-01T00:00:02Z",
    );
    ctx.store.update_branch_head(main_branch_id, t3).unwrap();

    let result = ctx.run(&Commands::NavigateBack { count: Some(2) });
    assert!(result.is_ok(), "NavigateBack should succeed");

    let main_branch = ctx.store.get_branch(main_branch_id).unwrap().unwrap();
    assert_eq!(main_branch.head_turn_id, Some(t3));
    let cursor = ctx.store.get_cursor(session_id, main_branch_id).unwrap();
    assert_eq!(cursor, Some(t1));
}

#[test]
fn test_navigate_forward_linear_history() {
    let mut ctx = TestContext::new();
    let session_id = ctx.create_session();
    let main_branch_id = ctx.create_main_branch(session_id);
    let t1 = ctx.create_turn(
        session_id,
        main_branch_id,
        None,
        "User",
        "2024-01-01T00:00:00Z",
    );
    let t2 = ctx.create_turn(
        session_id,
        main_branch_id,
        Some(t1),
        "Assistant",
        "2024-01-01T00:00:01Z",
    );
    let t3 = ctx.create_turn(
        session_id,
        main_branch_id,
        Some(t2),
        "User",
        "2024-01-01T00:00:02Z",
    );
    ctx.store.update_branch_head(main_branch_id, t3).unwrap();
    ctx.store
        .set_cursor(session_id, main_branch_id, t1)
        .unwrap();

    let result = ctx.run(&Commands::NavigateForward { count: Some(2) });
    assert!(result.is_ok(), "NavigateForward should succeed");

    let main_branch = ctx.store.get_branch(main_branch_id).unwrap().unwrap();
    assert_eq!(main_branch.head_turn_id, Some(t3));
    let cursor = ctx.store.get_cursor(session_id, main_branch_id).unwrap();
    assert_eq!(cursor, Some(t3));
}

#[test]
fn test_navigate_forward_ambiguous_multi_future() {
    let mut ctx = TestContext::new();
    let session_id = ctx.create_session();
    let main_branch_id = ctx.create_main_branch(session_id);
    let t1 = ctx.create_turn(
        session_id,
        main_branch_id,
        None,
        "User",
        "2024-01-01T00:00:00Z",
    );
    let t2 = ctx.create_turn(
        session_id,
        main_branch_id,
        Some(t1),
        "Assistant",
        "2024-01-01T00:00:01Z",
    );
    let t3 = ctx.create_turn(
        session_id,
        main_branch_id,
        Some(t1),
        "User",
        "2024-01-01T00:00:02Z",
    );
    ctx.store.update_branch_head(main_branch_id, t2).unwrap();
    ctx.store
        .set_cursor(session_id, main_branch_id, t1)
        .unwrap();

    let result = ctx.run(&Commands::NavigateForward { count: Some(1) });
    assert!(result.is_ok(), "NavigateForward should succeed");
    let msg = result.unwrap();
    assert!(msg.contains("Ambiguous"));
    assert!(msg.contains("2 futures"));

    let main_branch = ctx.store.get_branch(main_branch_id).unwrap().unwrap();
    assert_eq!(main_branch.head_turn_id, Some(t2));
    assert_ne!(t3, t2);
}

#[test]
fn test_navigate_back_at_root_noops() {
    let mut ctx = TestContext::new();
    let session_id = ctx.create_session();
    let main_branch_id = ctx.create_main_branch(session_id);
    let t1 = ctx.create_turn(
        session_id,
        main_branch_id,
        None,
        "User",
        "2024-01-01T00:00:00Z",
    );
    ctx.store.update_branch_head(main_branch_id, t1).unwrap();

    let result = ctx.run(&Commands::NavigateBack { count: Some(1) });
    assert!(result.is_ok());

    let cursor = ctx.store.get_cursor(session_id, main_branch_id).unwrap();
    assert_eq!(cursor, Some(t1));
}

#[test]
fn test_fork_current_creates_branch() {
    let mut ctx = TestContext::new();
    let session_id = ctx.create_session();
    ctx.create_main_branch(session_id);

    let result = ctx.run(&Commands::ForkCurrent {
        name: "feature".to_string(),
    });
    assert!(
        result.is_ok(),
        "ForkCurrent should succeed: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("Forked") || msg.contains("feature"));
    let branches = ctx.store.list_branches(session_id).unwrap();
    assert_eq!(branches.len(), 2);
}

#[test]
fn test_fork_from_cursor_fails_without_turn() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::ForkFromCursor {
        turn_id: "nonexistent".to_string(),
        name: "test".to_string(),
    });
    assert!(result.is_err());
}

#[test]
fn test_continue_from_cursor_fails_without_turn() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::ContinueFromCursor {
        turn_id: "nonexistent".to_string(),
        prompt: "".to_string(),
    });
    assert!(result.is_err());
}
