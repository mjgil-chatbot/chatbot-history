use super::*;

#[test]
fn test_init_creates_session_and_main_branch() {
    let mut ctx = TestContext::new();
    let result = ctx.run(&Commands::Init {
        title: Some("test session".to_string()),
    });
    assert!(
        result.is_ok(),
        "Init should succeed: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("Initialized") || msg.contains("session"));

    let sessions = ctx.store.list_sessions().unwrap();
    assert_eq!(sessions.len(), 1);

    let branches = ctx.store.list_branches(sessions[0].id).unwrap();
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].name, "main");
}

#[test]
fn test_init_with_default_title() {
    let mut ctx = TestContext::new();
    let result = ctx.run(&Commands::Init { title: None });
    assert!(
        result.is_ok(),
        "Init with default title should succeed: {}",
        result.unwrap_err()
    );
}

#[test]
fn test_checkpoint_creates_checkpoint() {
    let mut ctx = TestContext::new();
    let session_id = ctx.create_session();
    ctx.create_main_branch(session_id);

    let result = ctx.run(&Commands::Checkpoint {
        name: Some("test checkpoint".to_string()),
        reason: "test".to_string(),
    });
    assert!(
        result.is_ok(),
        "Checkpoint should succeed: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("Created checkpoint"));
    let checkpoints = ctx.store.list_checkpoints(session_id).unwrap();
    assert_eq!(checkpoints.len(), 1);
}

#[test]
fn test_checkpoint_fails_without_session() {
    let mut ctx = TestContext::new();
    let result = ctx.run(&Commands::Checkpoint {
        name: None,
        reason: "test".to_string(),
    });
    assert!(result.is_err());
}

#[test]
fn test_list_checkpoints_empty() {
    let mut ctx = TestContext::new();
    let session_id = ctx.create_session();
    let _ = session_id;

    let result = ctx.run(&Commands::ListCheckpoints {
        branch: "main".to_string(),
    });
    assert!(
        result.is_ok(),
        "List checkpoints should succeed: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("No checkpoints") || msg.contains("found"));
}

#[test]
fn test_list_checkpoints_shows_format() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::ListCheckpoints {
        branch: "main".to_string(),
    });
    assert!(
        result.is_ok(),
        "List checkpoints should succeed: {}",
        result.unwrap_err()
    );
}

#[test]
fn test_restore_nonexistent_checkpoint_fails() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::Restore {
        checkpoint_id: "nonexistent".to_string(),
        mode: RestoreModeCli::Both,
    });
    assert!(result.is_err());
}

#[test]
fn test_branches_lists_branches() {
    let mut ctx = TestContext::new();
    let session_id = ctx.create_session();
    ctx.create_main_branch(session_id);

    let result = ctx.run(&Commands::Branches);
    assert!(
        result.is_ok(),
        "Branches should succeed: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("main") || msg.contains("branch"));
}

#[test]
fn test_branches_empty() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::Branches);
    assert!(
        result.is_ok(),
        "Branches should succeed: {}",
        result.unwrap_err()
    );
}

#[test]
fn test_diff_nonexistent_checkpoint_fails() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::Diff {
        checkpoint_id: "nonexistent".to_string(),
        files_only: false,
    });
    assert!(result.is_err());
}

#[test]
fn test_status_shows_info() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::Status);
    assert!(
        result.is_ok(),
        "Status should succeed: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("Session") && msg.contains("Branches"));
}

#[test]
fn test_fork_creates_branch_from_checkpoint() {
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

    let cp_id = ctx.create_checkpoint(session_id, main_branch_id, Some(t1), "test");
    let result = ctx.run(&Commands::Fork {
        checkpoint_id: cp_id.to_string(),
        name: "feature-x".to_string(),
    });
    assert!(result.is_ok(), "Fork should succeed");

    let msg = result.unwrap();
    assert!(msg.contains("Forked branch 'feature-x'"));

    let branches = ctx.store.list_branches(session_id).unwrap();
    let forked = branches
        .iter()
        .find(|branch| branch.name == "feature-x")
        .unwrap();
    assert_eq!(forked.parent_branch_id, Some(main_branch_id));
    assert_eq!(forked.head_turn_id, Some(t1));
}

#[test]
fn test_checkpoint_with_real_workspace_scan() {
    let mut ctx = TestContext::new();
    let session_id = ctx.create_session();
    ctx.create_main_branch(session_id);
    ctx.write_file("test.txt", b"hello world");

    let result = ctx.run(&Commands::Checkpoint {
        name: Some("test".to_string()),
        reason: "integration test".to_string(),
    });
    assert!(
        result.is_ok(),
        "Checkpoint with real workspace should succeed: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("Created checkpoint"));
    assert!(msg.contains("files scanned") || msg.contains("1 file"));
    let checkpoints = ctx.store.list_checkpoints(session_id).unwrap();
    assert_eq!(checkpoints.len(), 1);
}

#[test]
fn test_list_checkpoints_shows_real_data() {
    let mut ctx = TestContext::new();
    let session_id = ctx.create_session();
    ctx.create_main_branch(session_id);
    ctx.write_file("test.txt", b"hello");

    ctx.run(&Commands::Checkpoint {
        name: Some("first".to_string()),
        reason: "test".to_string(),
    })
    .expect("checkpoint should succeed");

    let result = ctx.run(&Commands::ListCheckpoints {
        branch: "main".to_string(),
    });
    assert!(result.is_ok());
    let msg = result.unwrap();
    assert!(msg.contains("first") || msg.contains("checkpoint"));
}

#[test]
fn test_status_shows_real_branch_info() {
    let mut ctx = TestContext::new();
    let session_id = ctx.create_session();
    ctx.create_main_branch(session_id);

    let result = ctx.run(&Commands::Status);
    assert!(
        result.is_ok(),
        "Status should succeed: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("Session") && msg.contains("Branches"));
}

#[test]
fn test_branches_shows_main_branch() {
    let mut ctx = TestContext::new();
    let session_id = ctx.create_session();
    ctx.create_main_branch(session_id);

    let result = ctx.run(&Commands::Branches);
    assert!(
        result.is_ok(),
        "Branches should succeed: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("main"));
}

#[test]
fn test_fork_creates_new_branch_from_checkpoint() {
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

    let cp_id = CheckpointId::new();
    let manifest_id = ai_history_core::ManifestId::new();
    ctx.store
        .create_checkpoint(&CheckpointInfo {
            id: cp_id,
            session_id,
            branch_id: main_branch_id,
            turn_id: Some(t1),
            manifest_id,
            reason: "test".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        })
        .unwrap();
    ctx.store
        .create_manifest(&ai_history_store::ManifestInfo {
            id: manifest_id,
            session_id,
            checkpoint_id: cp_id,
            manifest_data: "{}".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        })
        .unwrap();

    let result = ctx.run(&Commands::Fork {
        checkpoint_id: cp_id.to_string(),
        name: "feature-branch".to_string(),
    });
    assert!(
        result.is_ok(),
        "Fork should succeed: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("Forked") && msg.contains("feature-branch"));

    let branches = ctx.store.list_branches(session_id).unwrap();
    assert_eq!(branches.len(), 2);
    let forked = branches
        .iter()
        .find(|branch| branch.name == "feature-branch")
        .unwrap();
    assert_eq!(forked.parent_branch_id, Some(main_branch_id));
}
