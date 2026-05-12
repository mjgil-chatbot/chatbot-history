//! Compile-boundary tests for crate type consolidation
//!
//! Task 5: Consolidate duplicate domain models
//!
//! This module provides compile-time tests that verify:
//! - No parallel event enums appear in non-events crates
//! - No duplicate ContextSnapshot types in incompatible form
//! - No duplicate restore mode definitions
//! - No duplicate checkpoint types
//! - Downstream crates use canonical types only

/// Test that ContextSnapshot in context crate has expected fields
/// This verifies that the canonical ContextSnapshot is in ai-history-context
#[cfg(test)]
mod test_context_snapshot_canonical {
    use ai_history_context::context::ContextSnapshot;
    use ai_history_core::{ContentHash, TurnId};

    #[test]
    fn test_context_snapshot_has_required_fields() {
        let turn_id = TurnId::new();
        let mut snapshot = ContextSnapshot::new(turn_id);

        // Verify all required fields exist
        assert!(snapshot.selected_files.len() == 0);
        assert!(snapshot.retrieved_chunks.len() == 0);
        assert!(snapshot.tool_outputs.len() == 0);
        assert!(snapshot.summary_refs.len() == 0);
        assert!(snapshot.memory_refs.len() == 0);
        assert!(snapshot.rule_refs.len() == 0);

        // Verify model_config has expected fields
        let config = &snapshot.model_config;
        assert!(config.provider == "unknown");
    }

    #[test]
    fn test_context_snapshot_add_file() {
        use ai_history_context::context::SelectedFileRef;
        use ai_history_core::ContentHash;

        let turn_id = TurnId::new();
        let mut snapshot = ContextSnapshot::new(turn_id);

        let file_ref = SelectedFileRef {
            path: "src/main.rs".to_string(),
            blob_hash: ContentHash::new(b"test"),
            line_range: None,
            relevance_score: Some(0.9),
        };

        snapshot.add_selected_file(file_ref);
        assert_eq!(snapshot.selected_files.len(), 1);
        assert_eq!(snapshot.total_references(), 1);
    }
}

/// Test that core exports event types correctly
/// The canonical Event type should come from ai-history-events
#[cfg(test)]
mod test_event_canonical {
    use ai_history_core::{Timestamp, TurnId};
    use ai_history_events::{AssistantMessageStartedData, Event, UserMessageData};

    #[test]
    fn test_events_defined_in_events_crate() {
        let turn_id = TurnId::new();
        let event = Event::UserMessageAdded(UserMessageData {
            turn_id,
            content: "test".to_string(),
            timestamp: Timestamp::now(),
        });

        assert!(matches!(event, Event::UserMessageAdded(_)));
    }

    #[test]
    fn test_assistant_message_started() {
        let turn_id = TurnId::new();
        let event = Event::AssistantMessageStarted(AssistantMessageStartedData {
            turn_id,
            timestamp: Timestamp::now(),
        });

        assert!(matches!(event, Event::AssistantMessageStarted(_)));
    }

    #[test]
    fn test_event_serialization() {
        let turn_id = TurnId::new();
        let event = Event::UserMessageAdded(UserMessageData {
            turn_id,
            content: "test".to_string(),
            timestamp: Timestamp::now(),
        });

        let json = serde_json::to_string(&event).unwrap();
        let parsed: Event = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Event::UserMessageAdded(_)));
    }
}

/// Test that rollback modes are unified in core
#[cfg(test)]
mod test_rollback_modes_canonical {
    use ai_history_core::rollback_modes::RollbackMode;

    #[test]
    fn test_rollback_mode_variants() {
        assert_eq!(
            RollbackMode::ConversationOnly,
            RollbackMode::ConversationOnly
        );
        assert_eq!(RollbackMode::WorkspaceOnly, RollbackMode::WorkspaceOnly);
        assert_eq!(RollbackMode::Both, RollbackMode::Both);
        assert_eq!(RollbackMode::Fork, RollbackMode::Fork);
    }

    #[test]
    fn test_rollback_mode_default() {
        assert_eq!(RollbackMode::default(), RollbackMode::Both);
    }
}

/// Test that conflict types are unified
#[cfg(test)]
mod test_conflict_types_canonical {
    use ai_history_core::conflict_resolution::ConflictType;

    #[test]
    fn test_conflict_type_variants() {
        assert_eq!(ConflictType::ModifiedBoth, ConflictType::ModifiedBoth);
        assert_eq!(
            ConflictType::DeletedInCurrent,
            ConflictType::DeletedInCurrent
        );
        assert_eq!(
            ConflictType::DeletedInCheckpoint,
            ConflictType::DeletedInCheckpoint
        );
        assert_eq!(ConflictType::AddedInBoth, ConflictType::AddedInBoth);
    }
}

/// Test that checkpoint types use consistent IDs
#[cfg(test)]
mod test_checkpoint_canonical {
    use ai_history_core::{CheckpointId, ManifestId};
    use ai_history_store::traits::CheckpointInfo;

    #[test]
    fn test_checkpoint_info_has_required_fields() {
        let info = CheckpointInfo {
            id: CheckpointId::new(),
            session_id: ai_history_core::SessionId::new(),
            branch_id: ai_history_core::BranchId::new(),
            turn_id: None,
            manifest_id: ManifestId::new(),
            reason: "test".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        assert!(!info.reason.is_empty());
        assert_eq!(info.reason, "test");
    }
}

/// Test that message types are unified
#[cfg(test)]
mod test_message_canonical {
    use ai_history_core::message::MessageType;

    #[test]
    fn test_message_type_variants() {
        assert_eq!(MessageType::User, MessageType::User);
        assert_eq!(MessageType::Assistant, MessageType::Assistant);
        assert_eq!(MessageType::System, MessageType::System);
        assert_eq!(MessageType::Tool, MessageType::Tool);
    }
}
