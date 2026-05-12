//! Event lifecycle and persistence tests
//!
//! Task 26-35: Events persistence and lifecycle
//!
//! Verify:
//! - All message types exist (user, assistant, system, tool)
//! - Assistant streaming lifecycle events exist
//! - Tool-call lifecycle events exist
//! - Patch lifecycle events exist
//! - Event checksums are canonical and deterministic
//! - Hash-chain linkage works correctly

#[cfg(test)]
mod test_message_events {
    use ai_history_core::{Actor, Timestamp, TurnId};
    use ai_history_events::{
        AssistantMessageCancelledData, AssistantMessageCompletedData, AssistantMessageDeltaData,
        AssistantMessageFailedData, AssistantMessageResumedData, AssistantMessageStartedData,
        AssistantMessageToolInterruptionData, Event, SystemMessageData, ToolMessageData,
        UserMessageData,
    };

    /// Verify UserMessageData exists
    #[test]
    fn test_user_message_event() {
        let event = Event::UserMessageAdded(UserMessageData {
            turn_id: TurnId::new(),
            content: "Hello".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "UserMessageAdded");
    }

    /// Verify SystemMessageData exists
    #[test]
    fn test_system_message_event() {
        let event = Event::SystemMessageAdded(SystemMessageData {
            turn_id: TurnId::new(),
            content: "System info".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "SystemMessageAdded");
    }

    /// Verify ToolMessageData exists
    #[test]
    fn test_tool_message_event() {
        let event = Event::ToolMessageAdded(ToolMessageData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            content: "Tool result".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "ToolMessageAdded");
    }
}

#[cfg(test)]
mod test_assistant_streaming_lifecycle {
    use ai_history_core::{Timestamp, TurnId};
    use ai_history_events::{
        AssistantMessageCancelledData, AssistantMessageCompletedData, AssistantMessageDeltaData,
        AssistantMessageFailedData, AssistantMessageResumedData, AssistantMessageStartedData,
        AssistantMessageToolInterruptionData, Event,
    };

    /// Verify AssistantMessageStarted event
    #[test]
    fn test_assistant_started_event() {
        let event = Event::AssistantMessageStarted(AssistantMessageStartedData {
            turn_id: TurnId::new(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "AssistantMessageStarted");
    }

    /// Verify AssistantMessageDelta event with sequence
    #[test]
    fn test_assistant_delta_event() {
        let event = Event::AssistantMessageDelta(AssistantMessageDeltaData {
            turn_id: TurnId::new(),
            delta: "Hello".to_string(),
            sequence: 0,
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "AssistantMessageDelta");
    }

    /// Verify AssistantMessageCompleted event
    #[test]
    fn test_assistant_completed_event() {
        let event = Event::AssistantMessageCompleted(AssistantMessageCompletedData {
            turn_id: TurnId::new(),
            content: "Full response".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "AssistantMessageCompleted");
    }

    /// Verify AssistantMessageCancelled event
    #[test]
    fn test_assistant_cancelled_event() {
        let event = Event::AssistantMessageCancelled(AssistantMessageCancelledData {
            turn_id: TurnId::new(),
            partial_content: "Partial...".to_string(),
            reason: "User cancelled".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "AssistantMessageCancelled");
    }

    /// Verify AssistantMessageFailed event
    #[test]
    fn test_assistant_failed_event() {
        let event = Event::AssistantMessageFailed(AssistantMessageFailedData {
            turn_id: TurnId::new(),
            error: "Rate limit".to_string(),
            partial_content: Some("Partial".to_string()),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "AssistantMessageFailed");
    }

    /// Verify AssistantMessageToolInterruption event
    #[test]
    fn test_assistant_tool_interruption_event() {
        let event = Event::AssistantMessageToolInterruption(AssistantMessageToolInterruptionData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            partial_content: "Reading file...".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "AssistantMessageToolInterruption");
    }

    /// Verify AssistantMessageResumed event
    #[test]
    fn test_assistant_resumed_event() {
        let event = Event::AssistantMessageResumed(AssistantMessageResumedData {
            turn_id: TurnId::new(),
            resumed_from_sequence: 5,
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "AssistantMessageResumed");
    }
}

#[cfg(test)]
mod test_tool_call_lifecycle {
    use ai_history_core::{Timestamp, TurnId};
    use ai_history_events::{
        Event, ToolCallApprovedData, ToolCallCancelledData, ToolCallExecutedData,
        ToolCallFailedData, ToolCallRejectedData, ToolCallRequestedData, ToolCallRetriedData,
        ToolCallStartedData,
    };

    /// Verify ToolCallRequested event
    #[test]
    fn test_tool_call_requested_event() {
        let event = Event::ToolCallRequested(ToolCallRequestedData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            tool_name: "bash".to_string(),
            args: serde_json::json!({}),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "ToolCallRequested");
    }

    /// Verify ToolCallApproved event
    #[test]
    fn test_tool_call_approved_event() {
        let event = Event::ToolCallApproved(ToolCallApprovedData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "ToolCallApproved");
    }

    /// Verify ToolCallRejected event
    #[test]
    fn test_tool_call_rejected_event() {
        let event = Event::ToolCallRejected(ToolCallRejectedData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            reason: "Dangerous".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "ToolCallRejected");
    }

    /// Verify ToolCallExecuted event
    #[test]
    fn test_tool_call_executed_event() {
        let event = Event::ToolCallExecuted(ToolCallExecutedData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            result: serde_json::json!({"output": "result"}),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "ToolCallExecuted");
    }

    /// Verify ToolCallFailed event
    #[test]
    fn test_tool_call_failed_event() {
        let event = Event::ToolCallFailed(ToolCallFailedData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            error: "Not found".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "ToolCallFailed");
    }

    /// Verify ToolCallCancelled event
    #[test]
    fn test_tool_call_cancelled_event() {
        let event = Event::ToolCallCancelled(ToolCallCancelledData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "ToolCallCancelled");
    }

    /// Verify ToolCallStarted event
    #[test]
    fn test_tool_call_started_event() {
        let event = Event::ToolCallStarted(ToolCallStartedData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            tool_name: "read_file".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "ToolCallStarted");
    }

    /// Verify ToolCallRetried event
    #[test]
    fn test_tool_call_retried_event() {
        let event = Event::ToolCallRetried(ToolCallRetriedData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            attempt_number: 2,
            previous_error: Some("Timeout".to_string()),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "ToolCallRetried");
    }
}

#[cfg(test)]
mod test_patch_lifecycle {
    use ai_history_core::{CheckpointId, ManifestId, Timestamp, TurnId};
    use ai_history_events::{
        Event, FilePatchAppliedData, FilePatchApprovedData, FilePatchFailedData,
        FilePatchPostWriteSnapshotData, FilePatchPreWriteCheckpointData, FilePatchProposedData,
        FilePatchRejectedData, FilePatchValidatedData, FilePatchVerifiedData,
    };

    /// Verify FilePatchProposed event
    #[test]
    fn test_patch_proposed_event() {
        let event = Event::FilePatchProposed(FilePatchProposedData {
            turn_id: TurnId::new(),
            patch_id: "patch_1".to_string(),
            file_path: "src/main.rs".to_string(),
            patch: "--- a\n+++ b".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "FilePatchProposed");
    }

    /// Verify FilePatchApproved event
    #[test]
    fn test_patch_approved_event() {
        let event = Event::FilePatchApproved(FilePatchApprovedData {
            turn_id: TurnId::new(),
            patch_id: "patch_1".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "FilePatchApproved");
    }

    /// Verify FilePatchRejected event
    #[test]
    fn test_patch_rejected_event() {
        let event = Event::FilePatchRejected(FilePatchRejectedData {
            turn_id: TurnId::new(),
            patch_id: "patch_1".to_string(),
            reason: "Syntax error".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "FilePatchRejected");
    }

    /// Verify FilePatchValidated event
    #[test]
    fn test_patch_validated_event() {
        let event = Event::FilePatchValidated(FilePatchValidatedData {
            turn_id: TurnId::new(),
            patch_id: "patch_1".to_string(),
            file_path: "src/main.rs".to_string(),
            hunks_count: 3,
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "FilePatchValidated");
    }

    /// Verify FilePatchPreWriteCheckpoint event
    #[test]
    fn test_patch_pre_write_checkpoint_event() {
        let event = Event::FilePatchPreWriteCheckpoint(FilePatchPreWriteCheckpointData {
            turn_id: TurnId::new(),
            patch_id: "patch_1".to_string(),
            checkpoint_id: CheckpointId::new(),
            file_path: "src/main.rs".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "FilePatchPreWriteCheckpoint");
    }

    /// Verify FilePatchApplied event
    #[test]
    fn test_patch_applied_event() {
        let event = Event::FilePatchApplied(FilePatchAppliedData {
            turn_id: TurnId::new(),
            patch_id: "patch_1".to_string(),
            file_path: "src/main.rs".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "FilePatchApplied");
    }

    /// Verify FilePatchVerified event
    #[test]
    fn test_patch_verified_event() {
        let event = Event::FilePatchVerified(FilePatchVerifiedData {
            turn_id: TurnId::new(),
            patch_id: "patch_1".to_string(),
            file_path: "src/main.rs".to_string(),
            verification_passed: true,
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "FilePatchVerified");
    }

    /// Verify FilePatchPostWriteSnapshot event
    #[test]
    fn test_patch_post_write_snapshot_event() {
        let event = Event::FilePatchPostWriteSnapshot(FilePatchPostWriteSnapshotData {
            turn_id: TurnId::new(),
            patch_id: "patch_1".to_string(),
            snapshot_id: CheckpointId::new(),
            file_path: "src/main.rs".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "FilePatchPostWriteSnapshot");
    }

    /// Verify FilePatchFailed event
    #[test]
    fn test_patch_failed_event() {
        let event = Event::FilePatchFailed(FilePatchFailedData {
            turn_id: TurnId::new(),
            patch_id: "patch_1".to_string(),
            error: "Permission denied".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_eq!(event.event_type(), "FilePatchFailed");
    }
}

#[cfg(test)]
mod test_event_checksums {
    use ai_history_core::{SessionId, Timestamp, TurnId};
    use ai_history_events::{Event, SessionCreatedData, UserMessageData};

    /// Verify event checksum is deterministic
    #[test]
    fn test_checksum_deterministic() {
        let event = Event::SessionCreated(SessionCreatedData {
            session_id: SessionId::new(),
            workspace_root: "/workspace".to_string(),
            timestamp: Timestamp::now(),
        });
        let c1 = event.checksum();
        let c2 = event.checksum();
        assert_eq!(c1, c2);
    }

    /// Verify different content produces different checksum
    #[test]
    fn test_different_content_different_checksum() {
        let event1 = Event::UserMessageAdded(UserMessageData {
            turn_id: TurnId::new(),
            content: "Hello".to_string(),
            timestamp: Timestamp::now(),
        });
        let event2 = Event::UserMessageAdded(UserMessageData {
            turn_id: TurnId::new(),
            content: "World".to_string(),
            timestamp: Timestamp::now(),
        });
        assert_ne!(event1.checksum(), event2.checksum());
    }

    /// Verify hash chain linkage modifies checksum
    #[test]
    fn test_hash_chain_linkage() {
        let event = Event::TurnCreated(ai_history_events::TurnCreatedData {
            turn_id: TurnId::new(),
            parent_turn_id: None,
            actor: ai_history_core::Actor::User,
            timestamp: Timestamp::now(),
        });
        let standalone = event.checksum();
        let chained = event.checksum_chain(Some("previous_hash"));
        assert_ne!(standalone, chained);
    }

    /// Verify checksum is valid SHA-256 (64 hex chars)
    #[test]
    fn test_checksum_format() {
        let event = Event::SessionCreated(SessionCreatedData {
            session_id: SessionId::new(),
            workspace_root: "/test".to_string(),
            timestamp: Timestamp::now(),
        });
        let checksum = event.checksum();
        assert_eq!(checksum.len(), 64);
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// Verify verify_checksum returns true for valid event
    #[test]
    fn test_verify_checksum() {
        let event = Event::SessionCreated(SessionCreatedData {
            session_id: SessionId::new(),
            workspace_root: "/test".to_string(),
            timestamp: Timestamp::now(),
        });
        assert!(event.verify_checksum());
    }
}
