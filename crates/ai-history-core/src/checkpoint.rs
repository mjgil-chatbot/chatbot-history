//! Checkpoint semantics for ai-history
//!
//! Defines checkpoint types:
//! - Prompt-level, tool-level, post-turn, named, and safety checkpoints
//! - Each checkpoint links to session, branch, turn, workspace/ context snapshots

use crate::{Actor, BranchId, CheckpointId, ContentHash, SessionId, TurnId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Checkpoint type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckpointType {
    /// Before user prompt
    PrePrompt,
    /// Before tool call execution
    PreTool,
    /// After turn completion
    PostTurn,
    /// User-named checkpoint
    Named,
    /// Safety checkpoint before destructive operations
    Safety,
    /// Automatic periodic checkpoint
    Auto,
}

impl CheckpointType {
    pub fn display_name(&self) -> &'static str {
        match self {
            CheckpointType::PrePrompt => "Pre-Prompt",
            CheckpointType::PreTool => "Pre-Tool",
            CheckpointType::PostTurn => "Post-Turn",
            CheckpointType::Named => "Named",
            CheckpointType::Safety => "Safety",
            CheckpointType::Auto => "Auto",
        }
    }

    pub fn is_user_visible(&self) -> bool {
        matches!(self, CheckpointType::Named | CheckpointType::PostTurn)
    }

    pub fn is_protected(&self) -> bool {
        matches!(self, CheckpointType::Named | CheckpointType::Safety)
    }
}

/// Checkpoint links
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointLinks {
    pub session_id: SessionId,
    pub branch_id: Option<BranchId>,
    pub turn_id: Option<TurnId>,
    pub workspace_snapshot_id: Option<ContentHash>,
    pub context_snapshot_id: Option<ContentHash>,
    pub event_sequence: u64,
}

impl CheckpointLinks {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            branch_id: None,
            turn_id: None,
            workspace_snapshot_id: None,
            context_snapshot_id: None,
            event_sequence: 0,
        }
    }

    pub fn with_branch(mut self, branch_id: BranchId) -> Self {
        self.branch_id = Some(branch_id);
        self
    }

    pub fn with_turn(mut self, turn_id: TurnId) -> Self {
        self.turn_id = Some(turn_id);
        self
    }

    pub fn with_workspace_snapshot(mut self, snapshot_id: ContentHash) -> Self {
        self.workspace_snapshot_id = Some(snapshot_id);
        self
    }

    pub fn with_context_snapshot(mut self, snapshot_id: ContentHash) -> Self {
        self.context_snapshot_id = Some(snapshot_id);
        self
    }

    pub fn with_event_sequence(mut self, seq: u64) -> Self {
        self.event_sequence = seq;
        self
    }
}

/// A checkpoint in the history system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: CheckpointId,
    pub checkpoint_type: CheckpointType,
    pub links: CheckpointLinks,
    pub name: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: Actor,
    pub metadata: HashMap<String, String>,
}

impl Checkpoint {
    pub fn new(id: CheckpointId, checkpoint_type: CheckpointType, links: CheckpointLinks) -> Self {
        Self {
            id,
            checkpoint_type,
            links,
            name: None,
            description: None,
            created_at: Utc::now(),
            created_by: Actor::User,
            metadata: HashMap::new(),
        }
    }

    /// Create a pre-prompt checkpoint
    pub fn pre_prompt(
        session_id: SessionId,
        branch_id: BranchId,
        turn_id: TurnId,
        event_seq: u64,
    ) -> Self {
        Self::new(
            CheckpointId::new(),
            CheckpointType::PrePrompt,
            CheckpointLinks::new(session_id)
                .with_branch(branch_id)
                .with_turn(turn_id)
                .with_event_sequence(event_seq),
        )
    }

    /// Create a pre-tool checkpoint
    pub fn pre_tool(
        session_id: SessionId,
        branch_id: BranchId,
        turn_id: TurnId,
        event_seq: u64,
    ) -> Self {
        Self::new(
            CheckpointId::new(),
            CheckpointType::PreTool,
            CheckpointLinks::new(session_id)
                .with_branch(branch_id)
                .with_turn(turn_id)
                .with_event_sequence(event_seq),
        )
    }

    /// Create a post-turn checkpoint
    pub fn post_turn(
        session_id: SessionId,
        branch_id: BranchId,
        turn_id: TurnId,
        workspace_snapshot: ContentHash,
        context_snapshot: ContentHash,
        event_seq: u64,
    ) -> Self {
        Self::new(
            CheckpointId::new(),
            CheckpointType::PostTurn,
            CheckpointLinks::new(session_id)
                .with_branch(branch_id)
                .with_turn(turn_id)
                .with_workspace_snapshot(workspace_snapshot)
                .with_context_snapshot(context_snapshot)
                .with_event_sequence(event_seq),
        )
    }

    /// Create a named checkpoint
    pub fn named(session_id: SessionId, branch_id: BranchId, name: String) -> Self {
        Self::new(
            CheckpointId::new(),
            CheckpointType::Named,
            CheckpointLinks::new(session_id).with_branch(branch_id),
        )
        .with_name(name)
    }

    /// Create a safety checkpoint
    pub fn safety(
        session_id: SessionId,
        branch_id: BranchId,
        workspace_snapshot: ContentHash,
        event_seq: u64,
    ) -> Self {
        Self::new(
            CheckpointId::new(),
            CheckpointType::Safety,
            CheckpointLinks::new(session_id)
                .with_branch(branch_id)
                .with_workspace_snapshot(workspace_snapshot)
                .with_event_sequence(event_seq),
        )
    }

    /// Set name
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set created by
    pub fn with_created_by(mut self, actor: Actor) -> Self {
        self.created_by = actor;
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// Check if checkpoint has turn reference
    pub fn has_turn_reference(&self) -> bool {
        self.links.turn_id.is_some()
    }

    /// Check if checkpoint has workspace snapshot
    pub fn has_workspace_snapshot(&self) -> bool {
        self.links.workspace_snapshot_id.is_some()
    }

    /// Check if checkpoint is user-visible
    pub fn is_user_visible(&self) -> bool {
        self.checkpoint_type.is_user_visible()
    }

    /// Check if checkpoint is protected
    pub fn is_protected(&self) -> bool {
        self.checkpoint_type.is_protected()
    }
}

/// Checkpoint creation trigger
#[derive(Debug, Clone, Copy)]
pub enum CheckpointTrigger {
    /// Before user prompt
    PrePrompt,
    /// Before file write
    PreWrite,
    /// After patch accepted
    PostPatch,
    /// Before restore
    PreRestore,
    /// User request
    UserRequest,
    /// Automatic
    Automatic,
}

impl CheckpointTrigger {
    pub fn to_checkpoint_type(&self) -> CheckpointType {
        match self {
            CheckpointTrigger::PrePrompt => CheckpointType::PrePrompt,
            CheckpointTrigger::PreWrite => CheckpointType::PreTool,
            CheckpointTrigger::PostPatch => CheckpointType::PostTurn,
            CheckpointTrigger::PreRestore => CheckpointType::Safety,
            CheckpointTrigger::UserRequest => CheckpointType::Named,
            CheckpointTrigger::Automatic => CheckpointType::Auto,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_type_display() {
        assert_eq!(CheckpointType::PrePrompt.display_name(), "Pre-Prompt");
        assert_eq!(CheckpointType::Named.display_name(), "Named");
        assert_eq!(CheckpointType::Safety.display_name(), "Safety");
    }

    #[test]
    fn test_checkpoint_type_user_visible() {
        assert!(!CheckpointType::PrePrompt.is_user_visible());
        assert!(!CheckpointType::PreTool.is_user_visible());
        assert!(CheckpointType::PostTurn.is_user_visible());
        assert!(CheckpointType::Named.is_user_visible());
        assert!(!CheckpointType::Safety.is_user_visible());
        assert!(!CheckpointType::Auto.is_user_visible());
    }

    #[test]
    fn test_checkpoint_type_protected() {
        assert!(!CheckpointType::PrePrompt.is_protected());
        assert!(CheckpointType::Named.is_protected());
        assert!(CheckpointType::Safety.is_protected());
        assert!(!CheckpointType::PostTurn.is_protected());
    }

    #[test]
    fn test_checkpoint_links_new() {
        let session_id = SessionId::new();
        let links = CheckpointLinks::new(session_id);
        assert_eq!(links.session_id, session_id);
        assert!(links.branch_id.is_none());
        assert!(links.turn_id.is_none());
    }

    #[test]
    fn test_checkpoint_links_chain() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn_id = TurnId::new();
        let snapshot = ContentHash::new(b"test");

        let links = CheckpointLinks::new(session_id)
            .with_branch(branch_id)
            .with_turn(turn_id)
            .with_workspace_snapshot(snapshot)
            .with_event_sequence(42);

        assert_eq!(links.branch_id, Some(branch_id));
        assert_eq!(links.turn_id, Some(turn_id));
        assert_eq!(links.workspace_snapshot_id, Some(snapshot));
        assert_eq!(links.event_sequence, 42);
    }

    #[test]
    fn test_checkpoint_pre_prompt() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn_id = TurnId::new();

        let checkpoint = Checkpoint::pre_prompt(session_id, branch_id, turn_id, 1);

        assert_eq!(checkpoint.checkpoint_type, CheckpointType::PrePrompt);
        assert!(checkpoint.has_turn_reference());
        assert_eq!(checkpoint.links.event_sequence, 1);
    }

    #[test]
    fn test_checkpoint_safety() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let snapshot = ContentHash::new(b"workspace");

        let checkpoint = Checkpoint::safety(session_id, branch_id, snapshot, 10);

        assert_eq!(checkpoint.checkpoint_type, CheckpointType::Safety);
        assert!(checkpoint.is_protected());
        assert_eq!(checkpoint.links.workspace_snapshot_id, Some(snapshot));
    }

    #[test]
    fn test_checkpoint_named() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        let checkpoint = Checkpoint::named(session_id, branch_id, "My checkpoint".to_string())
            .with_description("Important state".to_string());

        assert_eq!(checkpoint.checkpoint_type, CheckpointType::Named);
        assert_eq!(checkpoint.name, Some("My checkpoint".to_string()));
        assert_eq!(checkpoint.description, Some("Important state".to_string()));
        assert!(checkpoint.is_user_visible());
        assert!(checkpoint.is_protected());
    }

    #[test]
    fn test_checkpoint_post_turn() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn_id = TurnId::new();
        let workspace = ContentHash::new(b"workspace");
        let context = ContentHash::new(b"context");

        let checkpoint =
            Checkpoint::post_turn(session_id, branch_id, turn_id, workspace, context, 5);

        assert_eq!(checkpoint.checkpoint_type, CheckpointType::PostTurn);
        assert!(checkpoint.has_workspace_snapshot());
        assert_eq!(checkpoint.links.context_snapshot_id, Some(context));
        assert_eq!(checkpoint.links.event_sequence, 5);
    }

    #[test]
    fn test_checkpoint_trigger_to_type() {
        assert_eq!(
            CheckpointTrigger::PrePrompt.to_checkpoint_type(),
            CheckpointType::PrePrompt
        );
        assert_eq!(
            CheckpointTrigger::PreWrite.to_checkpoint_type(),
            CheckpointType::PreTool
        );
        assert_eq!(
            CheckpointTrigger::PostPatch.to_checkpoint_type(),
            CheckpointType::PostTurn
        );
        assert_eq!(
            CheckpointTrigger::PreRestore.to_checkpoint_type(),
            CheckpointType::Safety
        );
        assert_eq!(
            CheckpointTrigger::UserRequest.to_checkpoint_type(),
            CheckpointType::Named
        );
        assert_eq!(
            CheckpointTrigger::Automatic.to_checkpoint_type(),
            CheckpointType::Auto
        );
    }

    #[test]
    fn test_checkpoint_with_metadata() {
        let session_id = SessionId::new();
        let checkpoint = Checkpoint::new(
            CheckpointId::new(),
            CheckpointType::Auto,
            CheckpointLinks::new(session_id),
        )
        .with_metadata("key1", "value1")
        .with_metadata("key2", "value2");

        assert_eq!(checkpoint.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(checkpoint.metadata.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_checkpoint_created_at_is_set() {
        let session_id = SessionId::new();
        let checkpoint = Checkpoint::new(
            CheckpointId::new(),
            CheckpointType::PostTurn,
            CheckpointLinks::new(session_id),
        );

        assert!(checkpoint.created_at <= chrono::Utc::now());
    }
}
