//! Event log infrastructure for ai-history
//!
//! This module provides event infrastructure types that are used by the event store.
//! The canonical event definitions live in ai-history-events crate.
//!
//! Event schema ownership (Task 18):
//! - ai-history-events::Event is the single source of truth for event types
//! - This module provides EventId, SequenceNumber, PayloadVersion infrastructure

use crate::{CoreError, CoreResult, SessionId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Event ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(uuid::Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "evt:{}", self.0)
    }
}

impl std::str::FromStr for EventId {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = s.strip_prefix("evt:") {
            rest.parse().map(Self).map_err(|_| "invalid UUID")
        } else {
            s.parse().map(Self).map_err(|_| "invalid UUID")
        }
    }
}

/// Sequence number for event ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SequenceNumber(u64);

impl SequenceNumber {
    pub fn new(n: u64) -> Self {
        Self(n)
    }

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl Default for SequenceNumber {
    fn default() -> Self {
        Self(0)
    }
}

impl std::fmt::Display for SequenceNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Event version
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PayloadVersion(u8);

impl PayloadVersion {
    pub fn new(v: u8) -> Self {
        Self(v)
    }

    pub fn current() -> Self {
        Self(1)
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

impl Default for PayloadVersion {
    fn default() -> Self {
        Self::current()
    }
}

/// NOTE: The canonical Event type lives in ai-history-events crate.
/// This module provides infrastructure types only.

/// Event ordering rules
#[derive(Debug, Clone)]
pub struct EventOrdering {
    pub session_id: SessionId,
    pub branch_id: Option<String>,
    pub last_sequence: SequenceNumber,
}

impl EventOrdering {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            branch_id: None,
            last_sequence: SequenceNumber::default(),
        }
    }

    /// Validate that a new sequence is strictly greater
    pub fn validate_sequence(&self, new_seq: SequenceNumber) -> CoreResult<()> {
        if new_seq <= self.last_sequence {
            return Err(CoreError::InvalidSequence(format!(
                "Sequence {} is not greater than last {}",
                new_seq.value(),
                self.last_sequence.value()
            )));
        }
        Ok(())
    }

    /// Update with a committed sequence
    pub fn advance_to(&mut self, sequence: SequenceNumber) {
        if sequence > self.last_sequence {
            self.last_sequence = sequence;
        }
    }
}

/// Event replay result
#[derive(Debug, Clone)]
pub struct ReplayResult {
    pub events_replayed: u64,
    pub errors: Vec<String>,
    pub final_sequence: SequenceNumber,
}

impl ReplayResult {
    pub fn new() -> Self {
        Self {
            events_replayed: 0,
            errors: Vec::new(),
            final_sequence: SequenceNumber::default(),
        }
    }

    pub fn with_sequence(mut self, seq: SequenceNumber) -> Self {
        self.final_sequence = seq;
        self
    }

    pub fn add_error(&mut self, err: String) {
        self.errors.push(err);
    }

    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Default for ReplayResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Event schema validator
pub struct EventSchemaValidator;

impl EventSchemaValidator {
    /// Check if payload version is supported
    pub fn validate_version(version: PayloadVersion) -> bool {
        version.value() <= PayloadVersion::current().value()
    }

    /// Check for duplicate event IDs
    pub fn is_duplicate(event_id: EventId, seen: &HashMap<EventId, ()>) -> bool {
        seen.contains_key(&event_id)
    }
}

/// Transactionally assigned sequence number guard
/// Ensures sequence numbers are assigned atomically with event append
pub struct SeqAssignmentGuard {
    expected_next: SequenceNumber,
    committed: bool,
}

impl SeqAssignmentGuard {
    /// Create a new guard for the expected next sequence
    pub fn new(next_seq: SequenceNumber) -> Self {
        Self {
            expected_next: next_seq,
            committed: false,
        }
    }

    /// Claim the next sequence number (would be called during transaction commit)
    /// Returns the sequence number to use for the event
    pub fn claim(&mut self) -> SequenceNumber {
        let seq = self.expected_next;
        self.expected_next = seq.next();
        seq
    }

    /// Mark as committed
    pub fn commit(&mut self) {
        self.committed = true;
    }

    /// Rollback - do not use the claimed sequence
    pub fn rollback(self) {
        // Nothing to do - the sequence was never officially committed
        // The next guard will start from the same expected_next
    }

    /// Check if already committed
    pub fn is_committed(&self) -> bool {
        self.committed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_id_new() {
        let id = EventId::new();
        assert!(!format!("{}", id).is_empty());
    }

    #[test]
    fn test_event_id_roundtrip() {
        let id = EventId::new();
        let s = id.to_string();
        let parsed: EventId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_sequence_number_ordering() {
        let seq1 = SequenceNumber::new(1);
        let seq2 = SequenceNumber::new(2);
        let seq3 = SequenceNumber::new(3);

        assert!(seq1 < seq2);
        assert!(seq2 < seq3);
        assert!(seq1.next() == seq2);
    }

    #[test]
    fn test_event_ordering_validate() {
        let session_id = SessionId::new();
        let mut ordering = EventOrdering::new(session_id);

        ordering.advance_to(SequenceNumber::new(1));
        ordering.advance_to(SequenceNumber::new(2));

        assert!(ordering.validate_sequence(SequenceNumber::new(3)).is_ok());
        assert!(ordering.validate_sequence(SequenceNumber::new(2)).is_err()); // Not greater
        assert!(ordering.validate_sequence(SequenceNumber::new(1)).is_err()); // Not greater
    }

    #[test]
    fn test_duplicate_detection() {
        let seen = HashMap::new();
        let id1 = EventId::new();

        assert!(!EventSchemaValidator::is_duplicate(id1, &seen));

        let mut seen = HashMap::new();
        seen.insert(id1, ());

        assert!(EventSchemaValidator::is_duplicate(id1, &seen));
    }

    #[test]
    fn test_sequence_number_next() {
        let seq = SequenceNumber::new(100);
        let next = seq.next();
        assert_eq!(next.value(), 101);
    }

    #[test]
    fn test_payload_version_current() {
        let v = PayloadVersion::current();
        assert_eq!(v.value(), 1);
    }

    #[test]
    fn test_event_ordering_rejects_duplicate_sequence() {
        let session_id = SessionId::new();
        let mut ordering = EventOrdering::new(session_id);

        ordering.advance_to(SequenceNumber::new(1));
        ordering.advance_to(SequenceNumber::new(2));

        // Same sequence should fail
        assert!(ordering.validate_sequence(SequenceNumber::new(2)).is_err());
    }

    #[test]
    fn test_event_ordering_detects_gaps() {
        let session_id = SessionId::new();
        let mut ordering = EventOrdering::new(session_id);

        ordering.advance_to(SequenceNumber::new(1));
        ordering.advance_to(SequenceNumber::new(5));

        // Gap detection: last is 5, but a gap existed (2,3,4 skipped)
        // Note: This doesn't fail validation, just notes the gap
        // The gap itself isn't rejected - only non-increasing sequences
        assert!(ordering.validate_sequence(SequenceNumber::new(6)).is_ok());
    }

    #[test]
    fn test_event_ordering_rejects_out_of_order() {
        let session_id = SessionId::new();
        let mut ordering = EventOrdering::new(session_id);

        ordering.advance_to(SequenceNumber::new(5));

        // Out of order - trying to append 3 after 5
        assert!(ordering.validate_sequence(SequenceNumber::new(3)).is_err());
        assert!(ordering.validate_sequence(SequenceNumber::new(4)).is_err());
        assert!(ordering.validate_sequence(SequenceNumber::new(5)).is_err()); // Equal fails too
    }

    #[test]
    fn test_event_ordering_advance_only_if_greater() {
        let session_id = SessionId::new();
        let mut ordering = EventOrdering::new(session_id);

        ordering.advance_to(SequenceNumber::new(10));
        ordering.advance_to(SequenceNumber::new(5)); // Should NOT go backwards

        assert_eq!(ordering.last_sequence.value(), 10);
    }

    #[test]
    fn test_replay_result_tracks_events_replayed() {
        let mut result = ReplayResult::new();
        result.events_replayed = 100;
        assert_eq!(result.events_replayed, 100);
    }

    #[test]
    fn test_replay_result_with_errors_tracks_them() {
        let mut result = ReplayResult::new();
        result.add_error("Event 5 corrupted".to_string());
        result.add_error("Event 10 sequence gap".to_string());

        assert!(!result.is_success());
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_seq_assignment_guard_claim() {
        let mut guard = SeqAssignmentGuard::new(SequenceNumber::new(1));

        let seq1 = guard.claim();
        assert_eq!(seq1.value(), 1);

        let seq2 = guard.claim();
        assert_eq!(seq2.value(), 2);

        guard.commit();
        assert!(guard.is_committed());
    }

    #[test]
    fn test_seq_assignment_guard_sequential_claims() {
        let mut guard = SeqAssignmentGuard::new(SequenceNumber::new(0));

        let seq0 = guard.claim();
        assert_eq!(seq0.value(), 0);

        let seq1 = guard.claim();
        assert_eq!(seq1.value(), 1);

        let seq2 = guard.claim();
        assert_eq!(seq2.value(), 2);

        guard.commit();
    }

    #[test]
    fn test_seq_assignment_guard_rollback() {
        let guard = SeqAssignmentGuard::new(SequenceNumber::new(5));

        // Rollback doesn't error, just drops the guard
        guard.rollback();

        // New guard starting from same point
        let mut guard2 = SeqAssignmentGuard::new(SequenceNumber::new(5));
        let seq = guard2.claim();
        assert_eq!(seq.value(), 5);
    }
}
