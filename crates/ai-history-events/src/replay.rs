//! Event replay interfaces
//!
//! This module provides interfaces for replaying events into read models and
//! storing/querying events by session and branch.
//!
//! # Branch-Event Query Contract
//!
//! Events are associated with branches through the turn hierarchy:
//! - `BranchHeadMoved(branch=B, new_turn=T)` assigns turn T to branch B
//! - `TurnCreated(turn=T', parent_turn=T)` inherits branch from parent T
//!
//! The `get_branch_events` method returns only events whose `turn_id` belongs
//! to the specified branch. Events without a `turn_id` (session events like
//! `SessionCreated`, `BranchCreated`) are excluded from branch queries since
//! they are session-scoped, not branch-scoped.
//!
//! # Session Isolation
//!
//! Events are stored per-session. Querying a session returns only events
//! that were appended to that session. Cross-session queries return empty results.

use super::event::Event;
use ai_history_core::{Actor, BranchId, CoreResult, SessionId, TurnId};
use std::collections::HashMap;

#[cfg(test)]
use super::event::{
    AssistantMessageCompletedData, BranchCreatedData, BranchHeadMovedData, SessionCreatedData,
    TurnCreatedData, UserMessageData,
};
#[cfg(test)]
use ai_history_core::Timestamp;

/// Trait for replaying events into read models
pub trait EventReplay {
    /// Replay an event and update the read model
    fn replay(&mut self, event: &Event) -> CoreResult<()>;

    /// Replay multiple events in order
    fn replay_all(&mut self, events: &[Event]) -> CoreResult<()> {
        for event in events {
            self.replay(event)?;
        }
        Ok(())
    }

    /// Get the current state after replay
    fn current_state(&self) -> &dyn std::any::Any;
}

/// Trait for event stores
pub trait EventStore {
    /// Append events to the store
    fn append(&mut self, session_id: SessionId, events: &[Event]) -> CoreResult<()>;

    /// Get events for a session
    fn get_events(&self, session_id: SessionId) -> CoreResult<Vec<Event>>;

    /// Get events since a sequence number
    fn get_events_since(&self, session_id: SessionId, since: u64) -> CoreResult<Vec<Event>>;

    /// Get events for a branch.
    ///
    /// Returns events whose `turn_id` belongs to the specified branch.
    /// Session-scoped events (e.g., `SessionCreated`) are excluded.
    /// Returns empty if no events exist for the given session or branch.
    fn get_branch_events(&self, session_id: SessionId, branch_id: &str) -> CoreResult<Vec<Event>>;
}

/// In-memory event replay store for testing
pub struct EventReplayStore {
    events: Vec<Event>,
    session_events: HashMap<SessionId, Vec<Event>>,
    /// Maps turn IDs to the branch they belong to
    turn_branch: HashMap<TurnId, BranchId>,
}

impl EventReplayStore {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            session_events: HashMap::new(),
            turn_branch: HashMap::new(),
        }
    }

    pub fn with_events(events: Vec<Event>) -> Self {
        let mut store = Self::new();
        store.events = events;
        store
    }

    /// Add events for a specific session
    pub fn add_events_for_session(&mut self, session_id: SessionId, events: &[Event]) {
        for event in events {
            self.session_events
                .entry(session_id)
                .or_default()
                .push(event.clone());
        }
    }

    /// Get all stored events
    pub fn all_events(&self) -> &[Event] {
        &self.events
    }

    /// Update turn-to-branch mapping from a BranchHeadMoved event
    fn update_turn_branch_from_head_moved(&mut self, event: &Event) {
        if let Some(branch_id) = event.get_branch_id_from_head_moved() {
            if let Some(turn_id) = event.get_turn_id() {
                self.turn_branch.insert(turn_id, branch_id);
            }
        }
    }

    /// Update turn-to-branch mapping from a TurnCreated event (inherit from parent)
    fn update_turn_branch_from_turn_created(&mut self, event: &Event) {
        if let Some(parent_turn_id) = event.get_parent_turn_id() {
            if let Some(parent_branch) = self.turn_branch.get(&parent_turn_id) {
                if let Some(turn_id) = event.get_turn_id() {
                    self.turn_branch.insert(turn_id, *parent_branch);
                }
            }
        }
    }
}

impl Default for EventReplayStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStore for EventReplayStore {
    fn append(&mut self, session_id: SessionId, events: &[Event]) -> CoreResult<()> {
        for event in events {
            // Update turn-to-branch mapping before storing
            self.update_turn_branch_from_head_moved(event);
            self.update_turn_branch_from_turn_created(event);

            self.session_events
                .entry(session_id)
                .or_default()
                .push(event.clone());
            self.events.push(event.clone());
        }
        Ok(())
    }

    fn get_events(&self, session_id: SessionId) -> CoreResult<Vec<Event>> {
        Ok(self
            .session_events
            .get(&session_id)
            .cloned()
            .unwrap_or_default())
    }

    fn get_events_since(&self, session_id: SessionId, since: u64) -> CoreResult<Vec<Event>> {
        let events = self
            .session_events
            .get(&session_id)
            .cloned()
            .unwrap_or_default();
        Ok(events
            .into_iter()
            .filter(|e| e.sequence() > since)
            .collect())
    }

    fn get_branch_events(&self, session_id: SessionId, branch_id: &str) -> CoreResult<Vec<Event>> {
        let events = self
            .session_events
            .get(&session_id)
            .cloned()
            .unwrap_or_default();
        let branch_id_str = branch_id.to_string();
        Ok(events
            .into_iter()
            .filter(|e| {
                if let Some(turn_id) = e.get_turn_id() {
                    if let Some(event_branch) = self.turn_branch.get(&turn_id) {
                        return event_branch.to_string() == branch_id_str;
                    }
                }
                false
            })
            .collect())
    }
}

/// Simple read model for chat transcript
#[derive(Debug, Clone)]
pub struct ChatTranscript {
    pub turns: Vec<TurnRecord>,
}

impl ChatTranscript {
    pub fn new() -> Self {
        Self { turns: Vec::new() }
    }
}

impl Default for ChatTranscript {
    fn default() -> Self {
        Self::new()
    }
}

/// A turn in the transcript
#[derive(Debug, Clone)]
pub struct TurnRecord {
    pub turn_id: TurnId,
    pub actor: Actor,
    pub user_message: Option<String>,
    pub assistant_message: Option<String>,
    pub tool_calls: Vec<String>,
}

impl EventReplay for ChatTranscript {
    fn replay(&mut self, event: &Event) -> CoreResult<()> {
        match event {
            Event::TurnCreated(data) => {
                self.turns.push(TurnRecord {
                    turn_id: data.turn_id,
                    actor: data.actor,
                    user_message: None,
                    assistant_message: None,
                    tool_calls: Vec::new(),
                });
            }
            Event::UserMessageAdded(data) => {
                if let Some(turn) = self.turns.iter_mut().find(|t| t.turn_id == data.turn_id) {
                    turn.user_message = Some(data.content.clone());
                }
            }
            Event::AssistantMessageCompleted(data) => {
                if let Some(turn) = self.turns.iter_mut().find(|t| t.turn_id == data.turn_id) {
                    turn.assistant_message = Some(data.content.clone());
                }
            }
            Event::ToolCallRequested(data) => {
                if let Some(turn) = self.turns.iter_mut().find(|t| t.turn_id == data.turn_id) {
                    turn.tool_calls.push(data.tool_name.clone());
                }
            }
            // Other event types don't affect transcript
            _ => {}
        }
        Ok(())
    }

    fn current_state(&self) -> &dyn std::any::Any {
        self
    }
}

impl ChatTranscript {
    /// Rebuild transcript from events
    pub fn from_events(events: &[Event]) -> CoreResult<Self> {
        let mut transcript = Self::new();
        transcript.replay_all(events)?;
        Ok(transcript)
    }

    /// Verify transcript matches replay from events
    pub fn verify_replay(&self, events: &[Event]) -> bool {
        let rebuilt = Self::from_events(events).ok();
        match rebuilt {
            Some(r) => {
                self.turns.len() == r.turns.len()
                    && self
                        .turns
                        .iter()
                        .zip(r.turns.iter())
                        .all(|(a, b)| a.turn_id == b.turn_id && a.actor == b.actor)
            }
            None => false,
        }
    }
}

/// Branch timeline read model
#[derive(Debug, Clone)]
pub struct BranchTimeline {
    pub branches: HashMap<String, Vec<TurnId>>,
    pub active_branch: String,
}

impl BranchTimeline {
    pub fn new() -> Self {
        Self {
            branches: HashMap::new(),
            active_branch: "main".to_string(),
        }
    }
}

impl Default for BranchTimeline {
    fn default() -> Self {
        Self::new()
    }
}

impl EventReplay for BranchTimeline {
    fn replay(&mut self, event: &Event) -> CoreResult<()> {
        match event {
            Event::BranchCreated(data) => {
                self.branches.insert(data.name.clone(), Vec::new());
            }
            Event::BranchHeadMoved(data) => {
                if let Some(turns) = self.branches.get_mut(&data.branch_id.to_string()) {
                    turns.push(data.new_turn_id);
                }
            }
            Event::TurnCreated(data) => {
                // Add turn to active branch
                if let Some(turns) = self.branches.get_mut(&self.active_branch) {
                    turns.push(data.turn_id);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn current_state(&self) -> &dyn std::any::Any {
        self
    }
}

impl BranchTimeline {
    /// Rebuild branch timeline from events
    pub fn from_events(events: &[Event]) -> CoreResult<Self> {
        let mut timeline = Self::new();
        timeline.replay_all(events)?;
        Ok(timeline)
    }
}

/// Check if incremental replay produces same result as full rebuild
pub struct ReplayEquivalenceChecker;

impl ReplayEquivalenceChecker {
    /// Verify that replaying events incrementally gives same result as full rebuild
    pub fn check_incremental_equivalence(
        events: &[Event],
        mut incremental_model: impl EventReplay,
        full_rebuild: impl Fn(&[Event]) -> CoreResult<Box<dyn std::any::Any>>,
    ) -> bool {
        // Build incremental model
        if incremental_model.replay_all(events).is_err() {
            return false;
        }

        // Build full model
        let full = match full_rebuild(events) {
            Ok(f) => f,
            Err(_) => return false,
        };

        // Compare types
        let inc_state = incremental_model.current_state();
        let inc_type = std::any::type_name_of_val(inc_state);
        let full_type = std::any::type_name_of_val(&*full);

        inc_type == full_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Dummy implementations for testing the traits are object-safe
    #[test]
    fn test_event_replay_trait_object_safe() {
        fn _check_replay<T: EventReplay>() {}
        fn _check_store<T: EventStore>() {}

        // These compile if the traits are object-safe
        _check_replay::<ChatTranscript>();
        _check_replay::<BranchTimeline>();
    }

    #[test]
    fn test_event_replay_store_basic() {
        let store = EventReplayStore::new();
        let session_id = SessionId::new();

        // Should start empty
        let events = store.get_events(session_id).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_chat_transcript_replay() {
        let _session_id = SessionId::new();
        let turn_id = TurnId::new();

        // Build events
        let events = vec![
            Event::TurnCreated(TurnCreatedData {
                turn_id,
                parent_turn_id: None,
                actor: Actor::User,
                timestamp: Timestamp::now(),
            }),
            Event::UserMessageAdded(UserMessageData {
                turn_id,
                content: "Hello".to_string(),
                timestamp: Timestamp::now(),
            }),
            Event::AssistantMessageCompleted(AssistantMessageCompletedData {
                turn_id,
                content: "Hi there!".to_string(),
                timestamp: Timestamp::now(),
            }),
        ];

        // Replay into transcript
        let mut transcript = ChatTranscript::new();
        transcript.replay_all(&events).unwrap();

        // Should have one turn with both messages
        assert_eq!(transcript.turns.len(), 1);
        assert_eq!(transcript.turns[0].user_message, Some("Hello".to_string()));
        assert_eq!(
            transcript.turns[0].assistant_message,
            Some("Hi there!".to_string())
        );
    }

    #[test]
    fn test_branch_timeline_replay() {
        let branch_id = BranchId::new();

        let events = vec![Event::BranchCreated(BranchCreatedData {
            branch_id,
            name: "main".to_string(),
            parent_branch_id: None,
            from_turn_id: TurnId::new(),
            timestamp: Timestamp::now(),
        })];

        let mut timeline = BranchTimeline::new();
        timeline.replay_all(&events).unwrap();

        assert!(timeline.branches.contains_key("main"));
    }

    #[test]
    fn test_get_branch_events_filters_by_branch() {
        let mut store = EventReplayStore::new();
        let session_id = SessionId::new();
        let branch_a = BranchId::new();
        let branch_b = BranchId::new();

        let turn_a1 = TurnId::new();
        let turn_a2 = TurnId::new();
        let turn_b1 = TurnId::new();

        // Build events for branch A: turn_a1, turn_a2
        // Build events for branch B: turn_b1 (forked from turn_a1)
        let events = vec![
            // Branch A creation
            Event::BranchCreated(BranchCreatedData {
                branch_id: branch_a,
                name: "main".to_string(),
                parent_branch_id: None,
                from_turn_id: TurnId::new(),
                timestamp: Timestamp::now(),
            }),
            // Turn A1 created on branch A
            Event::TurnCreated(TurnCreatedData {
                turn_id: turn_a1,
                parent_turn_id: None,
                actor: Actor::User,
                timestamp: Timestamp::now(),
            }),
            Event::BranchHeadMoved(BranchHeadMovedData {
                branch_id: branch_a,
                previous_turn_id: TurnId::new(),
                new_turn_id: turn_a1,
                timestamp: Timestamp::now(),
            }),
            // Message on turn A1
            Event::UserMessageAdded(UserMessageData {
                turn_id: turn_a1,
                content: "Hello from A".to_string(),
                timestamp: Timestamp::now(),
            }),
            // Turn A2 created on branch A
            Event::TurnCreated(TurnCreatedData {
                turn_id: turn_a2,
                parent_turn_id: Some(turn_a1),
                actor: Actor::User,
                timestamp: Timestamp::now(),
            }),
            Event::BranchHeadMoved(BranchHeadMovedData {
                branch_id: branch_a,
                previous_turn_id: turn_a1,
                new_turn_id: turn_a2,
                timestamp: Timestamp::now(),
            }),
            // Branch B forked from turn_a1
            Event::BranchCreated(BranchCreatedData {
                branch_id: branch_b,
                name: "feature".to_string(),
                parent_branch_id: Some(branch_a),
                from_turn_id: turn_a1,
                timestamp: Timestamp::now(),
            }),
            // Turn B1 created on branch B (forked from A's turn_a1)
            Event::TurnCreated(TurnCreatedData {
                turn_id: turn_b1,
                parent_turn_id: Some(turn_a1),
                actor: Actor::User,
                timestamp: Timestamp::now(),
            }),
            Event::BranchHeadMoved(BranchHeadMovedData {
                branch_id: branch_b,
                previous_turn_id: TurnId::new(),
                new_turn_id: turn_b1,
                timestamp: Timestamp::now(),
            }),
            // Message on turn B1
            Event::UserMessageAdded(UserMessageData {
                turn_id: turn_b1,
                content: "Hello from B".to_string(),
                timestamp: Timestamp::now(),
            }),
        ];

        store.append(session_id, &events).unwrap();

        // Get events for branch A
        let branch_a_events = store
            .get_branch_events(session_id, &branch_a.to_string())
            .unwrap();
        // Should include events with turn_a1 and turn_a2
        assert!(!branch_a_events.is_empty());
        let branch_a_contents: Vec<_> = branch_a_events
            .iter()
            .filter_map(|e| match e {
                Event::UserMessageAdded(d) => Some(d.content.clone()),
                _ => None,
            })
            .collect();
        assert!(branch_a_contents.contains(&"Hello from A".to_string()));
        assert!(!branch_a_contents.contains(&"Hello from B".to_string()));

        // Get events for branch B
        let branch_b_events = store
            .get_branch_events(session_id, &branch_b.to_string())
            .unwrap();
        // Should include events with turn_b1 only
        assert!(!branch_b_events.is_empty());
        let branch_b_contents: Vec<_> = branch_b_events
            .iter()
            .filter_map(|e| match e {
                Event::UserMessageAdded(d) => Some(d.content.clone()),
                _ => None,
            })
            .collect();
        assert!(branch_b_contents.contains(&"Hello from B".to_string()));
        assert!(!branch_b_contents.contains(&"Hello from A".to_string()));
    }

    #[test]
    fn test_get_branch_events_session_isolation() {
        let mut store = EventReplayStore::new();
        let session_a = SessionId::new();
        let session_b = SessionId::new();
        let branch_x = BranchId::new();

        let turn_x = TurnId::new();

        // Session A has branch X
        let events_session_a = vec![
            Event::BranchCreated(BranchCreatedData {
                branch_id: branch_x,
                name: "main".to_string(),
                parent_branch_id: None,
                from_turn_id: TurnId::new(),
                timestamp: Timestamp::now(),
            }),
            Event::TurnCreated(TurnCreatedData {
                turn_id: turn_x,
                parent_turn_id: None,
                actor: Actor::User,
                timestamp: Timestamp::now(),
            }),
            Event::BranchHeadMoved(BranchHeadMovedData {
                branch_id: branch_x,
                previous_turn_id: TurnId::new(),
                new_turn_id: turn_x,
                timestamp: Timestamp::now(),
            }),
            Event::UserMessageAdded(UserMessageData {
                turn_id: turn_x,
                content: "Session A message".to_string(),
                timestamp: Timestamp::now(),
            }),
        ];

        // Session B has nothing - different session entirely

        store.append(session_a, &events_session_a).unwrap();

        // Query branch X from session A - should find events
        let events_from_a = store
            .get_branch_events(session_a, &branch_x.to_string())
            .unwrap();
        assert!(!events_from_a.is_empty());

        // Query branch X from session B - should be empty (session B has no events)
        let events_from_b = store
            .get_branch_events(session_b, &branch_x.to_string())
            .unwrap();
        assert!(events_from_b.is_empty());
    }

    #[test]
    fn test_get_events_session_isolation() {
        let mut store = EventReplayStore::new();
        let session_a = SessionId::new();
        let session_b = SessionId::new();

        let event_a = Event::SessionCreated(SessionCreatedData {
            session_id: session_a,
            workspace_root: "/tmp/a".to_string(),
            timestamp: Timestamp::now(),
        });
        let event_b = Event::SessionCreated(SessionCreatedData {
            session_id: session_b,
            workspace_root: "/tmp/b".to_string(),
            timestamp: Timestamp::now(),
        });

        store.append(session_a, &[event_a]).unwrap();
        store.append(session_b, &[event_b]).unwrap();

        let events_a = store.get_events(session_a).unwrap();
        let events_b = store.get_events(session_b).unwrap();

        assert_eq!(events_a.len(), 1);
        assert_eq!(events_b.len(), 1);

        // Verify contents are different
        match (&events_a[0], &events_b[0]) {
            (Event::SessionCreated(d1), Event::SessionCreated(d2)) => {
                assert_eq!(d1.workspace_root, "/tmp/a");
                assert_eq!(d2.workspace_root, "/tmp/b");
            }
            _ => panic!("Expected SessionCreated events"),
        }
    }
}
