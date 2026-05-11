//! Turn model for ai-history
//!
//! Task 16: Make turns truly immutable
//! - Replace mutating setters on finalized turns with builder/transaction APIs
//! - After commit, only explicitly allowed metadata may change

use super::{Actor, BranchId, CheckpointId, SessionId, TurnId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Error type for invalid turn operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnError {
    NotFinalized,
    AlreadyFinalized,
    ContentNotEditable,
}

impl TurnError {
    fn message(&self) -> &'static str {
        match self {
            TurnError::NotFinalized => "not finalized",
            TurnError::AlreadyFinalized => "already finalized",
            TurnError::ContentNotEditable => "content cannot be modified",
        }
    }
}

impl std::fmt::Display for TurnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for TurnError {}

/// Turn builder for constructing turns before finalization
pub struct TurnBuilder {
    session_id: SessionId,
    branch_id: BranchId,
    actor: Actor,
    parent_turn_id: Option<TurnId>,
    user_message: Option<String>,
    assistant_content: Option<String>,
    model_config: HashMap<String, String>,
    before_snapshot_id: Option<CheckpointId>,
    after_snapshot_id: Option<CheckpointId>,
    before_context_id: Option<String>,
    after_context_id: Option<String>,
}

impl TurnBuilder {
    pub fn new(session_id: SessionId, branch_id: BranchId, actor: Actor) -> Self {
        Self {
            session_id,
            branch_id,
            actor,
            parent_turn_id: None,
            user_message: None,
            assistant_content: None,
            model_config: HashMap::new(),
            before_snapshot_id: None,
            after_snapshot_id: None,
            before_context_id: None,
            after_context_id: None,
        }
    }

    pub fn parent_turn_id(mut self, id: TurnId) -> Self {
        self.parent_turn_id = Some(id);
        self
    }

    pub fn user_message(mut self, message: &str) -> Self {
        self.user_message = Some(message.to_string());
        self
    }

    pub fn assistant_content(mut self, content: &str) -> Self {
        self.assistant_content = Some(content.to_string());
        self
    }

    pub fn append_assistant_content(mut self, delta: &str) -> Self {
        if let Some(ref mut content) = self.assistant_content {
            content.push_str(delta);
        } else {
            self.assistant_content = Some(delta.to_string());
        }
        self
    }

    pub fn model_config(mut self, key: &str, value: &str) -> Self {
        self.model_config.insert(key.to_string(), value.to_string());
        self
    }

    pub fn before_snapshot(mut self, id: CheckpointId) -> Self {
        self.before_snapshot_id = Some(id);
        self
    }

    pub fn after_snapshot(mut self, id: CheckpointId) -> Self {
        self.after_snapshot_id = Some(id);
        self
    }

    pub fn before_context(mut self, id: &str) -> Self {
        self.before_context_id = Some(id.to_string());
        self
    }

    pub fn after_context(mut self, id: &str) -> Self {
        self.after_context_id = Some(id.to_string());
        self
    }

    pub fn build(self) -> Turn {
        Turn {
            id: TurnId::new(),
            session_id: self.session_id,
            branch_id: self.branch_id,
            parent_turn_id: self.parent_turn_id,
            user_message: self.user_message,
            assistant_content: self.assistant_content,
            model_config: self.model_config,
            before_snapshot_id: self.before_snapshot_id,
            after_snapshot_id: self.after_snapshot_id,
            before_context_id: self.before_context_id,
            after_context_id: self.after_context_id,
            actor: self.actor,
            created_at: Utc::now(),
            finalized_at: None,
        }
    }

    pub fn build_and_finalize(self) -> Turn {
        let mut turn = self.build();
        turn.finalize();
        turn
    }
}

/// Turn information - immutable after finalization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    id: TurnId,
    session_id: SessionId,
    branch_id: BranchId,
    parent_turn_id: Option<TurnId>,
    user_message: Option<String>,
    assistant_content: Option<String>,
    model_config: HashMap<String, String>,
    before_snapshot_id: Option<CheckpointId>,
    after_snapshot_id: Option<CheckpointId>,
    before_context_id: Option<String>,
    after_context_id: Option<String>,
    actor: Actor,
    created_at: DateTime<Utc>,
    finalized_at: Option<DateTime<Utc>>,
}

impl Turn {
    pub fn new(session_id: SessionId, branch_id: BranchId, actor: Actor) -> Self {
        Self {
            id: TurnId::new(),
            session_id,
            branch_id,
            parent_turn_id: None,
            user_message: None,
            assistant_content: None,
            model_config: HashMap::new(),
            before_snapshot_id: None,
            after_snapshot_id: None,
            before_context_id: None,
            after_context_id: None,
            actor,
            created_at: Utc::now(),
            finalized_at: None,
        }
    }

    pub fn builder(session_id: SessionId, branch_id: BranchId, actor: Actor) -> TurnBuilder {
        TurnBuilder::new(session_id, branch_id, actor)
    }

    pub fn id(&self) -> TurnId {
        self.id
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn branch_id(&self) -> BranchId {
        self.branch_id
    }

    pub fn parent_turn_id(&self) -> Option<TurnId> {
        self.parent_turn_id
    }

    pub fn user_message(&self) -> Option<&str> {
        self.user_message.as_deref()
    }

    pub fn assistant_content(&self) -> Option<&str> {
        self.assistant_content.as_deref()
    }

    pub fn model_config(&self) -> &HashMap<String, String> {
        &self.model_config
    }

    pub fn before_snapshot_id(&self) -> Option<CheckpointId> {
        self.before_snapshot_id
    }

    pub fn after_snapshot_id(&self) -> Option<CheckpointId> {
        self.after_snapshot_id
    }

    pub fn actor(&self) -> Actor {
        self.actor
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn finalized_at(&self) -> Option<DateTime<Utc>> {
        self.finalized_at
    }

    /// Finalize the turn - makes it immutable
    pub fn finalize(&mut self) {
        self.finalized_at = Some(Utc::now());
    }

    pub fn is_finalized(&self) -> bool {
        self.finalized_at.is_some()
    }

    pub fn is_immutable(&self) -> bool {
        self.finalized_at.is_some()
    }

    /// Check if content can be edited (only before finalization)
    pub fn can_edit_content(&self) -> bool {
        !self.is_finalized()
    }

    /// Attempt to set user message - only works before finalization
    pub fn set_user_message(&mut self, message: &str) -> Result<(), TurnError> {
        if self.is_finalized() {
            return Err(TurnError::AlreadyFinalized);
        }
        self.user_message = Some(message.to_string());
        Ok(())
    }

    /// Attempt to append assistant content - only works before finalization
    pub fn append_assistant_content(&mut self, delta: &str) -> Result<(), TurnError> {
        if self.is_finalized() {
            return Err(TurnError::AlreadyFinalized);
        }
        if let Some(ref mut content) = self.assistant_content {
            content.push_str(delta);
        } else {
            self.assistant_content = Some(delta.to_string());
        }
        Ok(())
    }

    /// After finalization, only metadata like model_config can be updated
    /// This is the "explicitly allowed metadata" exception
    pub fn update_model_config(&mut self, key: &str, value: &str) -> Result<(), TurnError> {
        if !self.is_finalized() {
            return Err(TurnError::NotFinalized);
        }
        self.model_config.insert(key.to_string(), value.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_new() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn = Turn::new(session_id, branch_id, Actor::User);
        assert!(turn.user_message().is_none());
        assert!(turn.finalized_at().is_none());
        assert!(!turn.is_finalized());
    }

    #[test]
    fn test_turn_builder() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn = Turn::builder(session_id, branch_id, Actor::User)
            .user_message("Hello")
            .assistant_content("Hi there!")
            .build();

        assert_eq!(turn.user_message(), Some("Hello"));
        assert_eq!(turn.assistant_content(), Some("Hi there!"));
    }

    #[test]
    fn test_turn_builder_and_finalize() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn = Turn::builder(session_id, branch_id, Actor::User)
            .user_message("Hello")
            .build_and_finalize();

        assert!(turn.is_finalized());
        assert!(turn.is_immutable());
    }

    #[test]
    fn test_turn_finalize() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut turn = Turn::new(session_id, branch_id, Actor::User);
        turn.finalize();
        assert!(turn.is_finalized());
        assert!(turn.is_immutable());
    }

    #[test]
    fn test_turn_mutation_before_finalize() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut turn = Turn::new(session_id, branch_id, Actor::User);

        turn.set_user_message("Hello").unwrap();
        assert_eq!(turn.user_message(), Some("Hello"));

        turn.append_assistant_content("Hi").unwrap();
        assert_eq!(turn.assistant_content(), Some("Hi"));
    }

    #[test]
    fn test_turn_mutation_after_finalize_fails() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut turn = Turn::new(session_id, branch_id, Actor::User);
        turn.finalize();

        let result = turn.set_user_message("Hello");
        assert!(matches!(result, Err(TurnError::AlreadyFinalized)));

        let result = turn.append_assistant_content("Hi");
        assert!(matches!(result, Err(TurnError::AlreadyFinalized)));
    }

    #[test]
    fn test_turn_can_edit_content() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut turn = Turn::new(session_id, branch_id, Actor::User);

        assert!(turn.can_edit_content());

        turn.finalize();
        assert!(!turn.can_edit_content());
    }

    #[test]
    fn test_turn_with_parent() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let parent_id = TurnId::new();
        let turn = Turn::builder(session_id, branch_id, Actor::User)
            .parent_turn_id(parent_id)
            .build();

        assert_eq!(turn.parent_turn_id(), Some(parent_id));
    }

    #[test]
    fn test_turn_with_before_after_snapshot() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let before = CheckpointId::new();
        let after = CheckpointId::new();

        let turn = Turn::builder(session_id, branch_id, Actor::User)
            .before_snapshot(before)
            .after_snapshot(after)
            .build();

        assert_eq!(turn.before_snapshot_id(), Some(before));
        assert_eq!(turn.after_snapshot_id(), Some(after));
    }

    #[test]
    fn test_turn_metadata_update_after_finalize() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut turn = Turn::new(session_id, branch_id, Actor::User);
        turn.finalize();

        // Only metadata updates are allowed after finalization
        let result = turn.update_model_config("model", "gpt-4");
        assert!(result.is_ok());
        assert_eq!(turn.model_config().get("model"), Some(&"gpt-4".to_string()));
    }

    #[test]
    fn test_turn_metadata_update_before_finalize_fails() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut turn = Turn::new(session_id, branch_id, Actor::User);

        // Metadata can only be updated after finalization
        let result = turn.update_model_config("model", "gpt-4");
        assert!(matches!(result, Err(TurnError::NotFinalized)));
    }

    #[test]
    fn test_turn_id_accessors() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn = Turn::new(session_id, branch_id, Actor::User);

        assert_eq!(turn.session_id(), session_id);
        assert_eq!(turn.branch_id(), branch_id);
        assert_eq!(turn.actor(), Actor::User);
    }

    #[test]
    fn test_turn_error_display() {
        assert!(TurnError::NotFinalized
            .to_string()
            .contains("not finalized"));
        assert!(TurnError::AlreadyFinalized
            .to_string()
            .contains("already finalized"));
        assert!(TurnError::ContentNotEditable
            .to_string()
            .contains("cannot be modified"));
    }
}
