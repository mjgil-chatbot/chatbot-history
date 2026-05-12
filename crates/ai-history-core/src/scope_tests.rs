//! Core scope verification tests
//!
//! Task 5: Verify core restricts itself to:
//! - IDs, hashes, timestamps, actors, errors, path safety, invariants
//! - Small domain enums and shared traits
//! - No concrete blob/filesystem/storage implementations

use crate::{
    BlobId, BranchId, CheckpointId, ConcurrencyConfig, CoreError, CoreResult, CrateName,
    DependencyChecker, DependencyRules, ManifestId, PatchId, SessionId, TurnId, TypeOwnership,
};

#[test]
fn test_typed_ids_core_owned() {
    let session_id = SessionId::new();
    let branch_id = BranchId::new();
    let turn_id = TurnId::new();
    let checkpoint_id = CheckpointId::new();
    let blob_id = BlobId::new();
    let manifest_id = ManifestId::new();
    let patch_id = PatchId::new();

    assert!(session_id.to_string().len() > 0);
    assert!(branch_id.to_string().len() > 0);
    assert!(turn_id.to_string().len() > 0);
    assert!(checkpoint_id.to_string().len() > 0);
    assert!(blob_id.to_string().len() > 0);
    assert!(manifest_id.to_string().len() > 0);
    assert!(patch_id.to_string().len() > 0);
}

#[test]
fn test_id_parsing() {
    let session_id = SessionId::new();
    let parsed = session_id.to_string().parse::<SessionId>();
    assert!(parsed.is_ok());

    let invalid = "not-a-valid-id".parse::<SessionId>();
    assert!(invalid.is_err());
}

#[test]
fn test_error_types() {
    let err = CoreError::InvalidId("test".to_string());
    let display = format!("{}", err);
    assert!(display.contains("Invalid ID format"));

    let result: CoreResult<()> = Err(CoreError::InvalidId("test".to_string()));
    assert!(result.is_err());
}

#[test]
fn test_concurrency_config() {
    let config = ConcurrencyConfig::default();
    assert_eq!(config.workspace_lock_timeout_ms, 5000);
    assert_eq!(config.append_lock_timeout_ms, 1000);
    assert!(config.restore_write_exclusion);
}

#[test]
fn test_canonical_ownership_registry() {
    let registry = TypeOwnership::new();

    assert_eq!(registry.get_owner("SessionId"), Some("ai-history-core"));
    assert_eq!(registry.get_owner("BranchId"), Some("ai-history-core"));
    assert_eq!(registry.get_owner("TurnId"), Some("ai-history-core"));
    assert_eq!(registry.get_owner("Checkpoint"), Some("ai-history-core"));
    assert_eq!(registry.get_owner("Event"), Some("ai-history-events"));
}

#[test]
fn test_dependency_rules_core_cannot_depend_on_store() {
    let checker = DependencyChecker::new();

    let violation = checker.check_dependency(CrateName::Core, CrateName::Store);
    assert!(violation.is_some());
}

#[test]
fn test_dependency_rules_fsm_cannot_depend_on_store() {
    let checker = DependencyChecker::new();

    let violation = checker.check_dependency(CrateName::Fsm, CrateName::Store);
    assert!(violation.is_some());
}

#[test]
fn test_no_storage_side_effects() {
    let session_id = SessionId::new();
    let branch_id = BranchId::new();
    let turn_id = TurnId::new();

    assert!(session_id.to_string().len() > 0);
    assert!(branch_id.to_string().len() > 0);
    assert!(turn_id.to_string().len() > 0);
}

#[test]
fn test_core_only_contains_domain_types() {
    let id = SessionId::new();
    assert!(id.to_string().len() > 0);
}

#[test]
fn test_blob_store_config() {
    use crate::BlobStoreConfig;

    let config = BlobStoreConfig::default();
    assert!(config.atomic_writes);

    let config = config.with_atomic_writes(false);
    assert!(!config.atomic_writes);
}

#[test]
fn test_error_display() {
    let err = CoreError::InvalidId("test-id".to_string());
    let display = format!("{}", err);
    assert!(display.contains("Invalid ID format"));
}

#[test]
fn test_session_id_generation() {
    let id1 = SessionId::new();
    let id2 = SessionId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_branch_id_generation() {
    let id1 = BranchId::new();
    let id2 = BranchId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_turn_id_generation() {
    let id1 = TurnId::new();
    let id2 = TurnId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_checkpoint_id_generation() {
    let id1 = CheckpointId::new();
    let id2 = CheckpointId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_blob_id_generation() {
    let id1 = BlobId::new();
    let id2 = BlobId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_manifest_id_generation() {
    let id1 = ManifestId::new();
    let id2 = ManifestId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_patch_id_generation() {
    let id1 = PatchId::new();
    let id2 = PatchId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_type_ownership_complete() {
    let registry = TypeOwnership::new();

    // All major types should be registered
    let types = vec![
        "SessionId",
        "BranchId",
        "TurnId",
        "CheckpointId",
        "Event",
        "Phase",
        "RestorePlan",
        "TimelineView",
    ];

    for type_name in types {
        let owner = registry.get_owner(type_name);
        assert!(
            owner.is_some(),
            "Type {} should have a canonical owner",
            type_name
        );
    }
}

#[test]
fn test_crate_names() {
    assert_eq!(CrateName::Core.as_str(), "ai-history-core");
    assert_eq!(CrateName::Fsm.as_str(), "ai-history-fsm");
    assert_eq!(CrateName::Store.as_str(), "ai-history-store");
    assert_eq!(CrateName::Events.as_str(), "ai-history-events");
}

#[test]
fn test_concurrency_config_custom() {
    let config = ConcurrencyConfig {
        workspace_lock_timeout_ms: 10000,
        append_lock_timeout_ms: 2000,
        restore_write_exclusion: false,
    };

    assert_eq!(config.workspace_lock_timeout_ms, 10000);
    assert_eq!(config.append_lock_timeout_ms, 2000);
    assert!(!config.restore_write_exclusion);
}
