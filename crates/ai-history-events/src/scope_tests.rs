//! Events scope verification tests
//!
//! Task 6: Verify events is the only owner of event enums, payload versions,
//! event metadata, actor, causation/correlation IDs, sequence numbers,
//! checksums, and hash-chain fields.

#[cfg(test)]
mod tests {
    use crate::{
        AssistantMessageCancelledData, AssistantMessageCompletedData, AssistantMessageDeltaData,
        AssistantMessageFailedData, AssistantMessageResumedData, AssistantMessageStartedData,
        AssistantMessageToolInterruptionData, BranchCreatedData, BranchHeadMovedData,
        CheckpointCreatedData, ConflictDetectedData, ConflictResolvedData, ErrorOccurredData,
        Event, EventMetadata, EventSchema, FilePatchAppliedData, FilePatchApprovedData,
        FilePatchFailedData, FilePatchPostWriteSnapshotData, FilePatchPreWriteCheckpointData,
        FilePatchProposedData, FilePatchRejectedData, FilePatchValidatedData,
        FilePatchVerifiedData, RestoreCancelledData, RestoreCompletedData, RestoreFailedData,
        RestoreStartedData, SessionClosedData, SessionCreatedData, SystemMessageData,
        ToolCallApprovedData, ToolCallCancelledData, ToolCallExecutedData, ToolCallFailedData,
        ToolCallRejectedData, ToolCallRequestedData, ToolCallRetriedData, ToolCallStartedData,
        ToolMessageData, TurnCreatedData, TurnFinalizedData, UserMessageData, EVENT_SCHEMA_VERSION,
    };
    use ai_history_core::{
        Actor, BranchId, CheckpointId, ManifestId, SessionId, Timestamp, TurnId,
    };

    #[test]
    fn test_event_type_name_turn_created() {
        let event = Event::TurnCreated(TurnCreatedData {
            turn_id: TurnId::new(),
            parent_turn_id: None,
            actor: Actor::User,
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "TurnCreated");
    }

    #[test]
    fn test_event_type_name_turn_finalized() {
        let event = Event::TurnFinalized(TurnFinalizedData {
            turn_id: TurnId::new(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "TurnFinalized");
    }

    #[test]
    fn test_event_type_name_user_message() {
        let event = Event::UserMessageAdded(UserMessageData {
            turn_id: TurnId::new(),
            content: "test".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "UserMessageAdded");
    }

    #[test]
    fn test_event_type_name_assistant_message() {
        let event = Event::AssistantMessageStarted(AssistantMessageStartedData {
            turn_id: TurnId::new(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "AssistantMessageStarted");
    }

    #[test]
    fn test_event_type_name_assistant_message_delta() {
        let event = Event::AssistantMessageDelta(AssistantMessageDeltaData {
            turn_id: TurnId::new(),
            delta: "hello".to_string(),
            sequence: 1,
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "AssistantMessageDelta");
    }

    #[test]
    fn test_event_checksum_deterministic() {
        let event = Event::TurnCreated(TurnCreatedData {
            turn_id: TurnId::new(),
            parent_turn_id: None,
            actor: Actor::User,
            timestamp: Timestamp::now(),
        });
        let checksum1 = event.checksum();
        let checksum2 = event.checksum();
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_event_checksum_chain_with_previous() {
        let event = Event::TurnCreated(TurnCreatedData {
            turn_id: TurnId::new(),
            parent_turn_id: None,
            actor: Actor::User,
            timestamp: Timestamp::now(),
        });
        let chain1 = event.checksum_chain(Some("previous_hash"));
        let chain2 = event.checksum_chain(Some("previous_hash"));
        assert_eq!(chain1, chain2);
    }

    #[test]
    fn test_event_checksum_chain_different_with_different_previous() {
        let event = Event::TurnCreated(TurnCreatedData {
            turn_id: TurnId::new(),
            parent_turn_id: None,
            actor: Actor::User,
            timestamp: Timestamp::now(),
        });
        let chain1 = event.checksum_chain(Some("hash_a"));
        let chain2 = event.checksum_chain(Some("hash_b"));
        assert_ne!(chain1, chain2);
    }

    #[test]
    fn test_event_verify_checksum() {
        let event = Event::TurnCreated(TurnCreatedData {
            turn_id: TurnId::new(),
            parent_turn_id: None,
            actor: Actor::User,
            timestamp: Timestamp::now(),
        });
        assert!(event.verify_checksum());
    }

    #[test]
    fn test_event_metadata_default() {
        let metadata = EventMetadata::default();
        assert!(metadata.causation_id.is_none());
        assert!(metadata.correlation_id.is_none());
        assert_eq!(metadata.actor, Actor::System);
        assert!(metadata.sequence_num.is_none());
    }

    #[test]
    fn test_event_metadata_with_values() {
        let metadata = EventMetadata {
            causation_id: Some("cause_123".to_string()),
            correlation_id: Some("corr_456".to_string()),
            actor: Actor::User,
            sequence_num: Some(42),
        };
        assert_eq!(metadata.causation_id, Some("cause_123".to_string()));
        assert_eq!(metadata.correlation_id, Some("corr_456".to_string()));
        assert_eq!(metadata.actor, Actor::User);
        assert_eq!(metadata.sequence_num, Some(42));
    }

    #[test]
    fn test_schema_version_constant() {
        assert_eq!(EVENT_SCHEMA_VERSION, "1.0");
    }

    #[test]
    fn test_schema_validate_valid_user_message() {
        let event = Event::UserMessageAdded(UserMessageData {
            turn_id: TurnId::new(),
            content: "Hello".to_string(),
            timestamp: Timestamp::now(),
        });
        assert!(EventSchema::validate(&event).is_ok());
    }

    #[test]
    fn test_schema_validate_empty_user_message_fails() {
        let event = Event::UserMessageAdded(UserMessageData {
            turn_id: TurnId::new(),
            content: "".to_string(),
            timestamp: Timestamp::now(),
        });
        assert!(EventSchema::validate(&event).is_err());
    }

    #[test]
    fn test_schema_validate_valid_tool_call() {
        let event = Event::ToolCallRequested(ToolCallRequestedData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            tool_name: "bash".to_string(),
            args: serde_json::json!({"command": "ls"}),
            timestamp: Timestamp::now(),
        });
        assert!(EventSchema::validate(&event).is_ok());
    }

    #[test]
    fn test_schema_validate_empty_tool_name_fails() {
        let event = Event::ToolCallRequested(ToolCallRequestedData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            tool_name: "".to_string(),
            args: serde_json::json!({}),
            timestamp: Timestamp::now(),
        });
        assert!(EventSchema::validate(&event).is_err());
    }

    #[test]
    fn test_event_type_coverage() {
        let all_events: Vec<Event> = vec![
            Event::TurnCreated(TurnCreatedData {
                turn_id: TurnId::new(),
                parent_turn_id: None,
                actor: Actor::User,
                timestamp: Timestamp::now(),
            }),
            Event::TurnFinalized(TurnFinalizedData {
                turn_id: TurnId::new(),
                timestamp: Timestamp::now(),
            }),
            Event::UserMessageAdded(UserMessageData {
                turn_id: TurnId::new(),
                content: "test".to_string(),
                timestamp: Timestamp::now(),
            }),
            Event::AssistantMessageStarted(AssistantMessageStartedData {
                turn_id: TurnId::new(),
                timestamp: Timestamp::now(),
            }),
            Event::AssistantMessageDelta(AssistantMessageDeltaData {
                turn_id: TurnId::new(),
                delta: "test".to_string(),
                sequence: 1,
                timestamp: Timestamp::now(),
            }),
            Event::AssistantMessageCompleted(AssistantMessageCompletedData {
                turn_id: TurnId::new(),
                content: "result".to_string(),
                timestamp: Timestamp::now(),
            }),
            Event::CheckpointCreated(CheckpointCreatedData {
                checkpoint_id: CheckpointId::new(),
                reason: "test".to_string(),
                manifest_id: ManifestId::new(),
                turn_id: Some(TurnId::new()),
                timestamp: Timestamp::now(),
            }),
            Event::BranchCreated(BranchCreatedData {
                branch_id: BranchId::new(),
                name: "test".to_string(),
                parent_branch_id: None,
                from_turn_id: TurnId::new(),
                timestamp: Timestamp::now(),
            }),
            Event::SessionCreated(SessionCreatedData {
                session_id: SessionId::new(),
                workspace_root: "/tmp".to_string(),
                timestamp: Timestamp::now(),
            }),
        ];

        for event in all_events {
            assert!(!event.event_type().is_empty());
            assert!(event.verify_checksum());
        }
    }

    #[test]
    fn test_event_serialization_round_trip() {
        let event = Event::TurnCreated(TurnCreatedData {
            turn_id: TurnId::new(),
            parent_turn_id: None,
            actor: Actor::User,
            timestamp: Timestamp::now(),
        });
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&serialized).unwrap();
        assert_eq!(event.event_type(), deserialized.event_type());
    }

    #[test]
    fn test_assistant_message_cancelled() {
        let event = Event::AssistantMessageCancelled(AssistantMessageCancelledData {
            turn_id: TurnId::new(),
            partial_content: "partial".to_string(),
            reason: "user cancelled".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "AssistantMessageCancelled");
    }

    #[test]
    fn test_assistant_message_failed() {
        let event = Event::AssistantMessageFailed(AssistantMessageFailedData {
            turn_id: TurnId::new(),
            error: "connection timeout".to_string(),
            partial_content: None,
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "AssistantMessageFailed");
    }

    #[test]
    fn test_tool_call_events() {
        let requested = Event::ToolCallRequested(ToolCallRequestedData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            tool_name: "bash".to_string(),
            args: serde_json::json!({}),
            timestamp: Timestamp::now(),
        });
        assert_eq!(requested.event_type(), "ToolCallRequested");

        let executed = Event::ToolCallExecuted(ToolCallExecutedData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            result: serde_json::json!("result"),
            timestamp: Timestamp::now(),
        });
        assert_eq!(executed.event_type(), "ToolCallExecuted");
    }

    #[test]
    fn test_file_patch_events() {
        let proposed = Event::FilePatchProposed(FilePatchProposedData {
            turn_id: TurnId::new(),
            patch_id: "patch_1".to_string(),
            file_path: "/tmp/test.txt".to_string(),
            patch: "diff content".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(proposed.event_type(), "FilePatchProposed");

        let applied = Event::FilePatchApplied(FilePatchAppliedData {
            turn_id: TurnId::new(),
            patch_id: "patch_1".to_string(),
            file_path: "/tmp/test.txt".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(applied.event_type(), "FilePatchApplied");
    }

    #[test]
    fn test_restore_events() {
        let started = Event::RestoreStarted(RestoreStartedData {
            checkpoint_id: CheckpointId::new(),
            mode: "full".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(started.event_type(), "RestoreStarted");

        let completed = Event::RestoreCompleted(RestoreCompletedData {
            checkpoint_id: CheckpointId::new(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(completed.event_type(), "RestoreCompleted");
    }

    #[test]
    fn test_conflict_events() {
        let detected = Event::ConflictDetected(ConflictDetectedData {
            files: vec!["/tmp/a.txt".to_string()],
            checkpoint_id: CheckpointId::new(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(detected.event_type(), "ConflictDetected");

        let resolved = Event::ConflictResolved(ConflictResolvedData {
            resolution: "keep_current".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(resolved.event_type(), "ConflictResolved");
    }
}
