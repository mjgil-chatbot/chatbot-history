//! Data generators for testing

use ai_history_core::*;
use ai_history_events::*;
use ai_history_fsm::Phase;
use rand::Rng;

/// Generate a random session ID
pub fn generate_session_id() -> SessionId {
    SessionId::new()
}

/// Generate a random branch ID
pub fn generate_branch_id() -> BranchId {
    BranchId::new()
}

/// Generate a random turn ID
pub fn generate_turn_id() -> TurnId {
    TurnId::new()
}

/// Generate a random checkpoint ID
pub fn generate_checkpoint_id() -> CheckpointId {
    CheckpointId::new()
}

/// Generate random content hash
pub fn generate_content_hash() -> ContentHash {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    ContentHash::from_bytes(bytes)
}

/// Generate a random manifest ID
pub fn generate_manifest_id() -> ManifestId {
    ManifestId::new()
}

/// Generate a random patch ID
pub fn generate_patch_id() -> PatchId {
    PatchId::new()
}

/// Generate a random blob ID
pub fn generate_blob_id() -> BlobId {
    BlobId::new()
}

/// Generate a sequence of test events
pub fn generate_event_sequence(count: usize, _session_id: SessionId) -> Vec<Event> {
    let mut events = Vec::new();
    let mut rng = rand::thread_rng();

    for i in 0..count {
        let turn_id = TurnId::new();
        let event = match i % 5 {
            0 => Event::TurnCreated(TurnCreatedData {
                turn_id,
                parent_turn_id: None,
                actor: Actor::User,
                timestamp: Timestamp::now(),
            }),
            1 => Event::UserMessageAdded(UserMessageData {
                turn_id,
                content: format!("Test message {}", i),
                timestamp: Timestamp::now(),
            }),
            2 => Event::AssistantMessageStarted(AssistantMessageStartedData {
                turn_id,
                timestamp: Timestamp::now(),
            }),
            3 => Event::AssistantMessageDelta(AssistantMessageDeltaData {
                turn_id,
                delta: format!("Delta {}", i),
                sequence: rng.gen(),
                timestamp: Timestamp::now(),
            }),
            _ => Event::AssistantMessageCompleted(AssistantMessageCompletedData {
                turn_id,
                content: format!("Completed response {}", i),
                timestamp: Timestamp::now(),
            }),
        };
        events.push(event);
    }

    events
}

/// Generate a random FSM state
pub fn generate_random_phase() -> Phase {
    let phases = [
        Phase::Idle,
        Phase::CreatingTurn,
        Phase::StreamingAssistant,
        Phase::AwaitingToolResult,
        Phase::ApplyingPatch,
        Phase::Checkpointing,
        Phase::Restoring,
        Phase::ConflictResolution,
        Phase::Cancelled,
        Phase::Error,
    ];
    let mut rng = rand::thread_rng();
    phases[rng.gen_range(0..phases.len())]
}

/// Generate test workspace path
pub fn generate_workspace_path() -> String {
    let mut rng = rand::thread_rng();
    format!("/tmp/test_workspace_{}", rng.gen::<u32>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ids() {
        let session_id = generate_session_id();
        let branch_id = generate_branch_id();
        let turn_id = generate_turn_id();
        let checkpoint_id = generate_checkpoint_id();

        assert_eq!(session_id.to_string().starts_with("session:"), true);
        assert_eq!(branch_id.to_string().starts_with("branch:"), true);
        assert_eq!(turn_id.to_string().starts_with("turn:"), true);
        assert_eq!(checkpoint_id.to_string().starts_with("checkpoint:"), true);
    }

    #[test]
    fn test_generate_event_sequence() {
        let session_id = SessionId::new();
        let events = generate_event_sequence(10, session_id);
        assert_eq!(events.len(), 10);
    }

    #[test]
    fn test_generate_random_phase() {
        for _ in 0..100 {
            let phase = generate_random_phase();
            assert!(matches!(
                phase,
                Phase::Idle
                    | Phase::CreatingTurn
                    | Phase::StreamingAssistant
                    | Phase::AwaitingToolResult
                    | Phase::ApplyingPatch
                    | Phase::Checkpointing
                    | Phase::Restoring
                    | Phase::ConflictResolution
                    | Phase::Cancelled
                    | Phase::Error
            ));
        }
    }
}
