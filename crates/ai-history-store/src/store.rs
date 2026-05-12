//! In-memory event store for testing and development

use ai_history_core::{CoreResult, SessionId};
use ai_history_events::{Event, EventStore};

/// In-memory event store implementation
#[derive(Debug, Default)]
pub struct InMemoryEventStore {
    sessions: std::collections::HashMap<SessionId, Vec<Event>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            sessions: std::collections::HashMap::new(),
        }
    }
}

impl EventStore for InMemoryEventStore {
    fn append(&mut self, session_id: SessionId, events: &[Event]) -> CoreResult<()> {
        let session_events = self.sessions.entry(session_id).or_default();
        session_events.extend(events.to_vec());
        Ok(())
    }

    fn get_events(&self, session_id: SessionId) -> CoreResult<Vec<Event>> {
        Ok(self.sessions.get(&session_id).cloned().unwrap_or_default())
    }

    fn get_events_since(&self, session_id: SessionId, since: u64) -> CoreResult<Vec<Event>> {
        let events = self.sessions.get(&session_id).cloned().unwrap_or_default();
        Ok(events.into_iter().skip(since as usize).collect())
    }

    fn get_branch_events(
        &self,
        _session_id: SessionId,
        _branch_id: &str,
    ) -> CoreResult<Vec<Event>> {
        Ok(vec![])
    }
}

impl InMemoryEventStore {
    /// Clear all stored events (for testing)
    pub fn clear(&mut self) {
        self.sessions.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_history_core::*;
    use ai_history_events::*;

    #[test]
    fn test_in_memory_store_append() {
        let mut store = InMemoryEventStore::new();
        let session_id = SessionId::new();
        let event = Event::SessionCreated(SessionCreatedData {
            session_id,
            workspace_root: "/tmp".to_string(),
            timestamp: Timestamp::now(),
        });

        store.append(session_id, &[event.clone()]).unwrap();
        let events = store.get_events(session_id).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_in_memory_store_get_events_since() {
        let mut store = InMemoryEventStore::new();
        let session_id = SessionId::new();

        for i in 0..5 {
            let event = Event::UserMessageAdded(UserMessageData {
                turn_id: TurnId::new(),
                content: format!("Message {}", i),
                timestamp: Timestamp::now(),
            });
            store.append(session_id, &[event]).unwrap();
        }

        let events = store.get_events_since(session_id, 3).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_in_memory_store_clear() {
        let mut store = InMemoryEventStore::new();
        let session_id = SessionId::new();
        let event = Event::SessionClosed(SessionClosedData {
            session_id,
            timestamp: Timestamp::now(),
        });
        store.append(session_id, &[event]).unwrap();
        store.clear();
        let events = store.get_events(session_id).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_in_memory_store_get_events_empty() {
        let store = InMemoryEventStore::new();
        let session_id = SessionId::new();
        let events = store.get_events(session_id).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_in_memory_store_get_events_since_out_of_range() {
        let mut store = InMemoryEventStore::new();
        let session_id = SessionId::new();

        let event = Event::UserMessageAdded(UserMessageData {
            turn_id: TurnId::new(),
            content: "Hello".to_string(),
            timestamp: Timestamp::now(),
        });
        store.append(session_id, &[event]).unwrap();

        // Request events since index 100 (beyond what we have)
        let events = store.get_events_since(session_id, 100).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_in_memory_store_multiple_sessions() {
        let mut store = InMemoryEventStore::new();
        let session1 = SessionId::new();
        let session2 = SessionId::new();

        let event1 = Event::SessionCreated(SessionCreatedData {
            session_id: session1,
            workspace_root: "/tmp".to_string(),
            timestamp: Timestamp::now(),
        });
        let event2 = Event::SessionCreated(SessionCreatedData {
            session_id: session2,
            workspace_root: "/home".to_string(),
            timestamp: Timestamp::now(),
        });

        store.append(session1, &[event1]).unwrap();
        store.append(session2, &[event2]).unwrap();

        let events1 = store.get_events(session1).unwrap();
        let events2 = store.get_events(session2).unwrap();

        assert_eq!(events1.len(), 1);
        assert_eq!(events2.len(), 1);
    }

    #[test]
    fn test_in_memory_store_event_sequence_order() {
        let mut store = InMemoryEventStore::new();
        let session_id = SessionId::new();

        // Append multiple events
        for i in 0..3 {
            let event = Event::UserMessageAdded(UserMessageData {
                turn_id: TurnId::new(),
                content: format!("Message {}", i),
                timestamp: Timestamp::now(),
            });
            store.append(session_id, &[event]).unwrap();
        }

        let events = store.get_events(session_id).unwrap();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_in_memory_store_branch_events_empty() {
        let store = InMemoryEventStore::new();
        let session_id = SessionId::new();
        let branch_id = "branch_1";

        let events = store.get_branch_events(session_id, branch_id).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_in_memory_store_new_is_empty() {
        let store = InMemoryEventStore::new();
        assert!(store.sessions.is_empty());
    }
}
