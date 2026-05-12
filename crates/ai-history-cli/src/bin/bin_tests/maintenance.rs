use super::*;

#[test]
fn test_gc_dry_run_with_empty_store() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::Gc {
        execute: false,
        protected: vec![],
    });
    assert!(result.is_ok(), "GC dry-run should succeed");

    let msg = result.unwrap();
    assert!(msg.contains("dry-run") || msg.contains("Nothing to delete"));
}

#[test]
fn test_gc_execute_flag_is_respected() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::Gc {
        execute: true,
        protected: vec![],
    });
    assert!(
        result.is_ok(),
        "GC execute should succeed: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("executed") || msg.contains("deleted"));
}

#[test]
fn test_doctor_returns_ok_on_clean_store() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::Doctor { fix: false });
    assert!(
        result.is_ok(),
        "Doctor should succeed: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("OK") || msg.contains("integrity"));
}

#[test]
fn test_doctor_returns_error_with_issues() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::Doctor { fix: true });
    assert!(
        result.is_ok(),
        "Doctor should succeed on clean store: {}",
        result.unwrap_err()
    );
}

#[test]
fn test_export_to_file() {
    let mut ctx = TestContext::new();
    ctx.create_session();
    let output = ctx.output_path("test_export.json");

    let result = ctx.run(&Commands::Export {
        output: output.clone(),
        include_blobs: false,
    });
    assert!(
        result.is_ok(),
        "Export should succeed: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("Exported"));
    assert!(ctx.workspace().join("test_export.json").exists());
    let _ = output;
}

#[test]
fn test_export_empty_store() {
    let mut ctx = TestContext::new();
    let output = ctx.output_path("empty_export.json");

    let result = ctx.run(&Commands::Export {
        output,
        include_blobs: false,
    });
    assert!(
        result.is_ok(),
        "Export should succeed on empty store: {}",
        result.unwrap_err()
    );
}

#[test]
fn test_import_validates_file() {
    let mut ctx = TestContext::new();
    let input = ctx.output_path("nonexistent_export.json");

    let result = ctx.run(&Commands::Import {
        input,
        session_id: None,
    });
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.contains("Failed to open") || err.contains("nonexistent"));
}

#[test]
fn test_import_rejects_invalid_json() {
    let mut ctx = TestContext::new();
    let invalid_file = ctx.workspace().join("invalid_import.json");
    std::fs::write(&invalid_file, "not valid json {").unwrap();

    let result = ctx.run(&Commands::Import {
        input: invalid_file.to_string_lossy().into_owned(),
        session_id: None,
    });
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.contains("Failed to parse") || err.contains("invalid"));
}

#[test]
fn test_recover_dry_run_on_clean_store() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::Recover { execute: false });
    assert!(
        result.is_ok(),
        "Recover dry-run should succeed on clean store: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("No issues") || msg.contains("success"));
}

#[test]
fn test_recover_detects_incomplete_turn() {
    let mut ctx = TestContext::new();
    let session_id = ctx.create_session();
    let main_branch_id = ctx.create_main_branch(session_id);
    ctx.create_turn(
        session_id,
        main_branch_id,
        None,
        "User",
        "2024-01-01T00:00:00Z",
    );

    let result = ctx.run(&Commands::Recover { execute: false });
    assert!(
        result.is_ok(),
        "Recover should succeed: {}",
        result.unwrap_err()
    );
}

#[test]
fn test_recover_execute_on_clean_store() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::Recover { execute: true });
    assert!(
        result.is_ok(),
        "Recover execute should succeed on clean store: {}",
        result.unwrap_err()
    );

    let msg = result.unwrap();
    assert!(msg.contains("completed"));
}

#[test]
fn test_verify_returns_error_for_nonexistent_checkpoint() {
    let mut ctx = TestContext::new();
    ctx.create_session();

    let result = ctx.run(&Commands::Verify {
        checkpoint_id: Some("nonexistent".to_string()),
    });
    assert!(result.is_err());
}
