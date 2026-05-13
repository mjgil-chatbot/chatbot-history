//! Testkit crate design tests
//!
//! Task 17: Complete `ai-history-testkit` crate design
//!
//! Verify:
//! - Builders compose with core, events, and store types
//! - Fakes implement traits correctly
//! - DeterministicClock produces consistent timestamps
//! - All fixtures can be used together across crates

#[cfg(test)]
mod test_builder_composition {
    use crate::{
        BranchBuilder, CheckpointBuilder, ConflictStateBuilder, DirtyWorkspaceBuilder,
        EventBuilder, SessionBuilder, TurnBuilder,
    };
    use ai_history_core::{Actor, ChangeType};

    /// Verify SessionBuilder produces store types
    #[test]
    fn test_session_builder_produces_session_info() {
        let session = SessionBuilder::new()
            .workspace_root("/test/workspace")
            .title("Test Session")
            .build();
        assert_eq!(session.workspace_root, "/test/workspace");
    }

    /// Verify BranchBuilder produces branch info
    #[test]
    fn test_branch_builder_produces_branch_info() {
        let branch = BranchBuilder::new().name("feature-branch").build();
        assert_eq!(branch.name, "feature-branch");
    }

    /// Verify TurnBuilder produces turn info
    #[test]
    fn test_turn_builder_produces_turn_info() {
        let turn = TurnBuilder::new().actor(Actor::Assistant).build();
        assert!(turn.actor.contains("Assistant"));
    }

    /// Verify CheckpointBuilder produces checkpoint info
    #[test]
    fn test_checkpoint_builder_produces_checkpoint_info() {
        let checkpoint = CheckpointBuilder::new().reason("test checkpoint").build();
        assert_eq!(checkpoint.reason, "test checkpoint");
    }

    /// Verify EventBuilder produces events
    #[test]
    fn test_event_builder_user_message() {
        let event = EventBuilder::new()
            .content("Hello, world!")
            .build_user_message();
        assert!(matches!(
            event,
            ai_history_events::Event::UserMessageAdded(_)
        ));
    }

    /// Verify EventBuilder produces assistant deltas
    #[test]
    fn test_event_builder_assistant_delta() {
        let event = EventBuilder::new().build_assistant_delta("Hello");
        assert!(matches!(
            event,
            ai_history_events::Event::AssistantMessageDelta(_)
        ));
    }

    /// Verify DirtyWorkspaceBuilder produces dirty workspace
    #[test]
    fn test_dirty_workspace_builder() {
        let dirty = DirtyWorkspaceBuilder::new()
            .add_file_change("src/main.rs", ChangeType::Modified)
            .build();
        assert!(dirty.is_dirty);
    }

    /// Verify ConflictStateBuilder produces conflicts
    #[test]
    fn test_conflict_state_builder() {
        let conflicts = ConflictStateBuilder::new()
            .add_modified("src/main.rs")
            .add_deleted("src/lib.rs")
            .build_conflicts();
        assert_eq!(conflicts.len(), 2);
    }

    /// Verify builders can be chained in a fluent style
    #[test]
    fn test_fluent_builder_chain() {
        let session = SessionBuilder::new()
            .title("Integration Test")
            .workspace_root("/project")
            .build();

        let branch = BranchBuilder::new()
            .session_id(session.id)
            .name("main")
            .build();

        let turn = TurnBuilder::new()
            .session_id(session.id)
            .branch_id(branch.id)
            .actor(Actor::User)
            .build();

        let event = EventBuilder::new()
            .turn_id(turn.id)
            .content("Test message")
            .build_user_message();

        assert_eq!(branch.name, "main");
        assert!(matches!(
            event,
            ai_history_events::Event::UserMessageAdded(_)
        ));
    }
}

#[cfg(test)]
mod test_fake_implementations {
    use crate::{DeterministicClock, FakeBlobStore, FakeEventStore};
    use ai_history_core::{ContentHash, SessionId};
    use ai_history_events::{Event, SessionCreatedData};
    use ai_history_store::traits::BlobStore;

    /// Verify FakeBlobStore implements BlobStore trait
    #[test]
    fn test_fake_blob_store_write_read() {
        let store = FakeBlobStore::new();
        let data = b"hello world";
        let hash = ContentHash::new(data);

        store.write(hash.as_bytes(), data).unwrap();
        assert!(store.has(hash.as_bytes()));

        let read = store.read(hash.as_bytes()).unwrap().unwrap();
        assert_eq!(read, data);
    }

    /// Verify FakeBlobStore works with BlobStore trait
    #[test]
    fn test_fake_blob_store_trait_impl() {
        let mut store = FakeBlobStore::new();
        let data = b"test content";
        let hash = ContentHash::new(data);

        // Use the trait methods
        store.write_blob(hash.as_bytes(), data).unwrap();
        assert!(store.has_blob(hash.as_bytes()));

        let read = store.read_blob(hash.as_bytes()).unwrap().unwrap();
        assert_eq!(read, data);
    }

    /// Verify FakeBlobStore clear works
    #[test]
    fn test_fake_blob_store_clear() {
        let store = FakeBlobStore::new();
        let hash = ContentHash::new(b"data");
        store.write(hash.as_bytes(), b"data").unwrap();
        assert!(store.has(hash.as_bytes()));

        store.clear();
        assert!(!store.has(hash.as_bytes()));
    }

    /// Verify FakeEventStore works
    #[test]
    fn test_fake_event_store() {
        let store = FakeEventStore::new();
        let session_id = SessionId::new();

        let event = Event::SessionCreated(SessionCreatedData {
            session_id,
            workspace_root: "/tmp".to_string(),
            timestamp: ai_history_core::Timestamp::now(),
        });

        store.append(session_id, event.clone());
        let events = store.get_all(session_id);
        assert_eq!(events.len(), 1);
    }

    /// Verify FakeEventStore clear works
    #[test]
    fn test_fake_event_store_clear() {
        let store = FakeEventStore::new();
        let session_id = SessionId::new();

        store.append(
            session_id,
            Event::SessionCreated(SessionCreatedData {
                session_id,
                workspace_root: "/tmp".to_string(),
                timestamp: ai_history_core::Timestamp::now(),
            }),
        );
        assert_eq!(store.get_all(session_id).len(), 1);

        store.clear();
        assert!(store.get_all(session_id).is_empty());
    }

    /// Verify DeterministicClock produces timestamps
    #[test]
    fn test_deterministic_clock() {
        let clock = DeterministicClock::new(1000);
        let ts1 = clock.now();
        clock.advance(3600);
        let ts2 = clock.now();
        assert_ne!(ts1.to_string(), ts2.to_string());
    }

    /// Verify DeterministicClock now_utc produces consistent values
    #[test]
    fn test_deterministic_clock_consistency() {
        let clock = DeterministicClock::new(1704067200); // 2024-01-01 00:00:00 UTC
        let ts = clock.now();
        assert!(ts.to_string().contains("2024"));
    }
}

#[cfg(test)]
mod test_fixture_cross_crate_composition {
    use crate::{
        BranchBuilder, CheckpointBuilder, DeterministicClock, EventBuilder, FakeBlobStore,
        FakeEventStore, SessionBuilder, TurnBuilder,
    };
    use ai_history_core::{ChangeOwnership, ChangeType, ContentHash, FileChange};
    use ai_history_events::Event;

    /// Verify fixtures from different modules compose together
    #[test]
    fn test_full_session_lifecycle_with_fixtures() {
        // Use DeterministicClock for consistent timestamps
        let _clock = DeterministicClock::new(1704067200);

        // Build a session
        let session = SessionBuilder::new().workspace_root("/test").build();

        // Build a branch
        let branch = BranchBuilder::new()
            .session_id(session.id)
            .name("main")
            .build();

        // Build a turn
        let turn = TurnBuilder::new()
            .session_id(session.id)
            .branch_id(branch.id)
            .build();

        // Use FakeEventStore to store events
        let event_store = FakeEventStore::new();
        let event = EventBuilder::new()
            .turn_id(turn.id)
            .content("Hello")
            .build_user_message();
        event_store.append(session.id, event);

        // Use FakeBlobStore for content
        let blob_store = FakeBlobStore::new();
        let content = b"file content";
        let hash = ContentHash::new(content);
        blob_store.write(hash.as_bytes(), content).unwrap();

        // Verify everything works together
        assert_eq!(branch.name, "main");
        assert_eq!(event_store.get_all(session.id).len(), 1);
        assert!(blob_store.has(hash.as_bytes()));
    }

    /// Verify checkpoint creation with all fixtures
    #[test]
    fn test_checkpoint_with_blobs() {
        let blob_store = FakeBlobStore::new();

        // Write some content
        let content1 = b"original content";
        let hash1 = ContentHash::new(content1);
        blob_store.write(hash1.as_bytes(), content1).unwrap();

        // Create checkpoint info
        let checkpoint = CheckpointBuilder::new().reason("before change").build();

        // Verify blob is accessible
        assert!(blob_store.has(hash1.as_bytes()));
        assert_eq!(checkpoint.reason, "before change");
    }
}
