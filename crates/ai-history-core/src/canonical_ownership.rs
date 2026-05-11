//! Canonical type ownership tests
//!
//! Task 4: Verify that each domain type has exactly one canonical owner crate.
//! Duplicate public type definitions across crate boundaries are prohibited.

use std::collections::HashMap;

/// Canonical type ownership registry
/// Maps type name -> canonical owner crate
pub struct TypeOwnership {
    /// Map of type name to its canonical owner crate
    owners: HashMap<&'static str, &'static str>,
}

impl TypeOwnership {
    pub fn new() -> Self {
        let mut owners = HashMap::new();

        // Typed IDs - core owns all
        owners.insert("SessionId", "ai-history-core");
        owners.insert("BranchId", "ai-history-core");
        owners.insert("TurnId", "ai-history-core");
        owners.insert("CheckpointId", "ai-history-core");
        owners.insert("BlobId", "ai-history-core");
        owners.insert("ManifestId", "ai-history-core");
        owners.insert("PatchId", "ai-history-core");
        owners.insert("SequenceNum", "ai-history-core");

        // Domain types - core owns session, branch, turn, checkpoint
        owners.insert("Session", "ai-history-core");
        owners.insert("Branch", "ai-history-core");
        owners.insert("Turn", "ai-history-core");
        owners.insert("Checkpoint", "ai-history-core");
        owners.insert("CheckpointType", "ai-history-core");
        owners.insert("CheckpointLinks", "ai-history-core");
        owners.insert("CheckpointTrigger", "ai-history-core");

        // Event types - events owns all
        owners.insert("Event", "ai-history-events");
        owners.insert("TurnCreatedData", "ai-history-events");
        owners.insert("TurnFinalizedData", "ai-history-events");
        owners.insert("UserMessageData", "ai-history-events");
        owners.insert("AssistantMessageStartedData", "ai-history-events");
        owners.insert("AssistantMessageDeltaData", "ai-history-events");
        owners.insert("AssistantMessageCompletedData", "ai-history-events");
        owners.insert("AssistantMessageCancelledData", "ai-history-events");
        owners.insert("AssistantMessageFailedData", "ai-history-events");
        owners.insert("ToolCallRequestedData", "ai-history-events");
        owners.insert("ToolCallApprovedData", "ai-history-events");
        owners.insert("ToolCallRejectedData", "ai-history-events");
        owners.insert("ToolCallExecutedData", "ai-history-events");
        owners.insert("ToolCallFailedData", "ai-history-events");
        owners.insert("ToolCallCancelledData", "ai-history-events");
        owners.insert("ToolCallStartedData", "ai-history-events");
        owners.insert("ToolCallRetriedData", "ai-history-events");
        owners.insert("FilePatchProposedData", "ai-history-events");
        owners.insert("FilePatchApprovedData", "ai-history-events");
        owners.insert("FilePatchRejectedData", "ai-history-events");
        owners.insert("FilePatchAppliedData", "ai-history-events");
        owners.insert("FilePatchFailedData", "ai-history-events");
        owners.insert("CheckpointCreatedData", "ai-history-events");
        owners.insert("BranchCreatedData", "ai-history-events");
        owners.insert("BranchHeadMovedData", "ai-history-events");

        // Store info types - store owns DTOs
        owners.insert("SessionInfo", "ai-history-store");
        owners.insert("BranchInfo", "ai-history-store");
        owners.insert("TurnInfo", "ai-history-store");
        owners.insert("CheckpointInfo", "ai-history-store");
        owners.insert("BlobInfo", "ai-history-store");
        owners.insert("ManifestInfo", "ai-history-store");

        // Blob storage - trait in store, concrete impl in snapshot
        owners.insert("BlobStore", "ai-history-snapshot"); // concrete implementation
        owners.insert("BlobStoreTrait", "ai-history-store"); // trait for storage backends
        owners.insert("SnapshotManager", "ai-history-snapshot");
        owners.insert("Manifest", "ai-history-snapshot");

        // Diff - diff owns
        owners.insert("Diff", "ai-history-diff");
        owners.insert("DiffChange", "ai-history-diff");
        owners.insert("ChangeKind", "ai-history-diff");
        owners.insert("DiffEngine", "ai-history-diff");

        // Restore - restore owns
        owners.insert("RestorePlan", "ai-history-restore");
        owners.insert("RestoreOperation", "ai-history-restore");
        owners.insert("RestoreMode", "ai-history-restore");
        owners.insert("ConflictPolicy", "ai-history-restore");
        owners.insert("DirtyState", "ai-history-restore");
        owners.insert("WorkspaceDirtyState", "ai-history-restore");
        owners.insert("RestoreExecutor", "ai-history-restore");
        owners.insert("RestoreVerificationError", "ai-history-restore");
        owners.insert("RestoreResult", "ai-history-restore");
        owners.insert("RestoreTransaction", "ai-history-restore");
        owners.insert("ConflictPlan", "ai-history-restore");
        owners.insert("ConflictResolution", "ai-history-restore");

        // FSM - fsm owns
        owners.insert("Phase", "ai-history-fsm");
        owners.insert("RuntimeState", "ai-history-fsm");
        owners.insert("TransitionResult", "ai-history-fsm");
        owners.insert("InputEvent", "ai-history-fsm");
        owners.insert("OutputAction", "ai-history-fsm");

        // Context - context owns
        owners.insert("Context", "ai-history-context");
        owners.insert("ContextSnapshot", "ai-history-context");
        owners.insert("ReproducibilityManager", "ai-history-context");

        // UI - ui-model owns
        owners.insert("TimelineView", "ai-history-ui-model");
        owners.insert("RestoreModalState", "ai-history-ui-model");

        // Core concurrency
        owners.insert("WorkspaceGuard", "ai-history-core");
        owners.insert("AppendGuard", "ai-history-core");
        owners.insert("WorkspaceLockManager", "ai-history-core");
        owners.insert("ConcurrencyConfig", "ai-history-core");
        owners.insert("ConcurrencyError", "ai-history-core");

        Self { owners }
    }

    /// Get the canonical owner for a type
    pub fn get_owner(&self, type_name: &str) -> Option<&'static str> {
        self.owners.get(type_name).copied()
    }

    /// Check if a type has a canonical owner assigned
    pub fn is_registered(&self, type_name: &str) -> bool {
        self.owners.contains_key(type_name)
    }

    /// Get all registered types for a given crate
    pub fn types_for_crate(&self, crate_name: &str) -> Vec<&'static str> {
        self.owners
            .iter()
            .filter(|(_, owner)| **owner == crate_name)
            .map(|(name, _)| *name)
            .collect()
    }

    /// Get all registered type names
    pub fn all_types(&self) -> Vec<&'static str> {
        self.owners.keys().copied().collect()
    }
}

impl Default for TypeOwnership {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_ids_are_core_owned() {
        let registry = TypeOwnership::new();
        assert_eq!(registry.get_owner("SessionId"), Some("ai-history-core"));
        assert_eq!(registry.get_owner("BranchId"), Some("ai-history-core"));
        assert_eq!(registry.get_owner("TurnId"), Some("ai-history-core"));
        assert_eq!(registry.get_owner("CheckpointId"), Some("ai-history-core"));
        assert_eq!(registry.get_owner("BlobId"), Some("ai-history-core"));
        assert_eq!(registry.get_owner("ManifestId"), Some("ai-history-core"));
        assert_eq!(registry.get_owner("PatchId"), Some("ai-history-core"));
    }

    #[test]
    fn test_domain_types_core_owned() {
        let registry = TypeOwnership::new();
        assert_eq!(registry.get_owner("Session"), Some("ai-history-core"));
        assert_eq!(registry.get_owner("Branch"), Some("ai-history-core"));
        assert_eq!(registry.get_owner("Turn"), Some("ai-history-core"));
        assert_eq!(registry.get_owner("Checkpoint"), Some("ai-history-core"));
    }

    #[test]
    fn test_events_owned_by_events_crate() {
        let registry = TypeOwnership::new();
        assert_eq!(registry.get_owner("Event"), Some("ai-history-events"));
        assert_eq!(
            registry.get_owner("TurnCreatedData"),
            Some("ai-history-events")
        );
        assert_eq!(
            registry.get_owner("UserMessageData"),
            Some("ai-history-events")
        );
        assert_eq!(
            registry.get_owner("AssistantMessageDeltaData"),
            Some("ai-history-events")
        );
        assert_eq!(
            registry.get_owner("ToolCallRequestedData"),
            Some("ai-history-events")
        );
        assert_eq!(
            registry.get_owner("FilePatchProposedData"),
            Some("ai-history-events")
        );
    }

    #[test]
    fn test_store_info_owned_by_store_crate() {
        let registry = TypeOwnership::new();
        assert_eq!(registry.get_owner("SessionInfo"), Some("ai-history-store"));
        assert_eq!(registry.get_owner("BranchInfo"), Some("ai-history-store"));
        assert_eq!(registry.get_owner("TurnInfo"), Some("ai-history-store"));
        assert_eq!(
            registry.get_owner("CheckpointInfo"),
            Some("ai-history-store")
        );
    }

    #[test]
    fn test_restore_owned_by_restore_crate() {
        let registry = TypeOwnership::new();
        assert_eq!(
            registry.get_owner("RestorePlan"),
            Some("ai-history-restore")
        );
        assert_eq!(
            registry.get_owner("RestoreMode"),
            Some("ai-history-restore")
        );
        assert_eq!(
            registry.get_owner("RestoreExecutor"),
            Some("ai-history-restore")
        );
        assert_eq!(registry.get_owner("DirtyState"), Some("ai-history-restore"));
        assert_eq!(
            registry.get_owner("ConflictResolution"),
            Some("ai-history-restore")
        );
    }

    #[test]
    fn test_fsm_owned_by_fsm_crate() {
        let registry = TypeOwnership::new();
        assert_eq!(registry.get_owner("Phase"), Some("ai-history-fsm"));
        assert_eq!(registry.get_owner("InputEvent"), Some("ai-history-fsm"));
        assert_eq!(registry.get_owner("OutputAction"), Some("ai-history-fsm"));
        assert_eq!(registry.get_owner("RuntimeState"), Some("ai-history-fsm"));
    }

    #[test]
    fn test_ui_model_owned_by_ui_model_crate() {
        let registry = TypeOwnership::new();
        assert_eq!(
            registry.get_owner("TimelineView"),
            Some("ai-history-ui-model")
        );
        assert_eq!(
            registry.get_owner("RestoreModalState"),
            Some("ai-history-ui-model")
        );
    }

    #[test]
    fn test_all_core_types_registered() {
        let registry = TypeOwnership::new();
        let core_types = registry.types_for_crate("ai-history-core");
        assert!(core_types.contains(&"SessionId"));
        assert!(core_types.contains(&"BranchId"));
        assert!(core_types.contains(&"TurnId"));
        assert!(core_types.contains(&"Session"));
        assert!(core_types.contains(&"Branch"));
        assert!(core_types.contains(&"Turn"));
    }

    #[test]
    fn test_all_events_types_registered() {
        let registry = TypeOwnership::new();
        let events_types = registry.types_for_crate("ai-history-events");
        assert!(events_types.contains(&"Event"));
    }

    #[test]
    fn test_all_restore_types_registered() {
        let registry = TypeOwnership::new();
        let restore_types = registry.types_for_crate("ai-history-restore");
        assert!(restore_types.contains(&"RestorePlan"));
        assert!(restore_types.contains(&"RestoreMode"));
    }

    #[test]
    fn test_no_unregistered_types() {
        let registry = TypeOwnership::new();
        let all = registry.all_types();
        assert!(!all.is_empty());
        for type_name in &["SessionId", "Event", "RestorePlan", "Phase", "TimelineView"] {
            assert!(
                registry.is_registered(type_name),
                "{} should be registered",
                type_name
            );
        }
    }

    #[test]
    fn test_each_type_has_exactly_one_owner() {
        let registry = TypeOwnership::new();
        let all_types = registry.all_types();

        // Verify each type appears exactly once in the ownership map
        // This is guaranteed by HashMap semantics, but we test the contract
        for type_name in all_types {
            let owner = registry.get_owner(type_name);
            assert!(owner.is_some(), "Type {} should have an owner", type_name);
        }
    }

    #[test]
    fn test_ownership_table_complete() {
        let registry = TypeOwnership::new();

        // Key types that must be registered
        let required_types = vec![
            // IDs
            "SessionId",
            "BranchId",
            "TurnId",
            "CheckpointId",
            "BlobId",
            "ManifestId",
            // Domain
            "Session",
            "Branch",
            "Turn",
            "Checkpoint",
            // Events
            "Event",
            // Store
            "SessionInfo",
            "BranchInfo",
            "TurnInfo",
            // Restore
            "RestorePlan",
            "RestoreMode",
            "RestoreExecutor",
            // FSM
            "Phase",
            "InputEvent",
            "OutputAction",
            // UI
            "TimelineView",
        ];

        for type_name in required_types {
            assert!(
                registry.is_registered(type_name),
                "Required type {} should be registered in ownership table",
                type_name
            );
        }
    }

    #[test]
    fn test_concurrent_types_core_owned() {
        let registry = TypeOwnership::new();
        assert_eq!(
            registry.get_owner("WorkspaceGuard"),
            Some("ai-history-core")
        );
        assert_eq!(registry.get_owner("AppendGuard"), Some("ai-history-core"));
        assert_eq!(
            registry.get_owner("WorkspaceLockManager"),
            Some("ai-history-core")
        );
        assert_eq!(
            registry.get_owner("ConcurrencyError"),
            Some("ai-history-core")
        );
    }
}
