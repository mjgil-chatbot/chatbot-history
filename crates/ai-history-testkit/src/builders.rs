//! Test fixture builders

use ai_history_core::{
    Actor, BranchId, ChangeOwnership, ChangeType, CheckpointId, ConflictType as CoreConflictType,
    DirtyWorkspace, FileChange, ManifestId, SessionId, Timestamp, TurnId,
};
use ai_history_events::*;
use ai_history_store::traits::{BranchInfo, CheckpointInfo, SessionInfo, TurnInfo};
use chrono::Utc;

/// Builder for creating test sessions
#[derive(Debug, Default)]
pub struct SessionBuilder {
    session_id: Option<SessionId>,
    workspace_root: Option<String>,
    title: Option<String>,
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn session_id(mut self, id: SessionId) -> Self {
        self.session_id = Some(id);
        self
    }

    pub fn workspace_root(mut self, root: &str) -> Self {
        self.workspace_root = Some(root.to_string());
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn build(self) -> SessionInfo {
        SessionInfo {
            id: self.session_id.unwrap_or_else(SessionId::new),
            title: self.title.unwrap_or_else(|| "Test Session".to_string()),
            active_branch: BranchId::new(),
            view_cursor: None,
            workspace_root: self
                .workspace_root
                .unwrap_or_else(|| "/tmp/test".to_string()),
            retention_config: "default".to_string(),
            project_identity: "test".to_string(),
            operation_state: "active".to_string(),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            archival_state: false,
        }
    }
}

/// Builder for creating test branches
#[derive(Debug, Default)]
pub struct BranchBuilder {
    branch_id: Option<BranchId>,
    session_id: Option<SessionId>,
    name: Option<String>,
    parent_branch_id: Option<BranchId>,
    head_turn_id: Option<TurnId>,
}

impl BranchBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn branch_id(mut self, id: BranchId) -> Self {
        self.branch_id = Some(id);
        self
    }

    pub fn session_id(mut self, id: SessionId) -> Self {
        self.session_id = Some(id);
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn parent_branch_id(mut self, id: BranchId) -> Self {
        self.parent_branch_id = Some(id);
        self
    }

    pub fn head_turn_id(mut self, id: TurnId) -> Self {
        self.head_turn_id = Some(id);
        self
    }

    pub fn build(self) -> BranchInfo {
        BranchInfo {
            id: self.branch_id.unwrap_or_else(BranchId::new),
            session_id: self.session_id.unwrap_or_else(SessionId::new),
            name: self.name.unwrap_or_else(|| "main".to_string()),
            parent_branch_id: self.parent_branch_id,
            created_from_turn_id: None,
            head_turn_id: self.head_turn_id,
            ancestry: Vec::new(),
            protected: false,
            archived: false,
            abandoned_future: false,
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Builder for creating test turns
#[derive(Debug, Default)]
pub struct TurnBuilder {
    turn_id: Option<TurnId>,
    session_id: Option<SessionId>,
    branch_id: Option<BranchId>,
    parent_turn_id: Option<TurnId>,
    actor: Option<Actor>,
}

impl TurnBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn turn_id(mut self, id: TurnId) -> Self {
        self.turn_id = Some(id);
        self
    }

    pub fn session_id(mut self, id: SessionId) -> Self {
        self.session_id = Some(id);
        self
    }

    pub fn branch_id(mut self, id: BranchId) -> Self {
        self.branch_id = Some(id);
        self
    }

    pub fn parent_turn_id(mut self, id: TurnId) -> Self {
        self.parent_turn_id = Some(id);
        self
    }

    pub fn actor(mut self, actor: Actor) -> Self {
        self.actor = Some(actor);
        self
    }

    pub fn build(self) -> TurnInfo {
        TurnInfo {
            id: self.turn_id.unwrap_or_else(TurnId::new),
            session_id: self.session_id.unwrap_or_else(SessionId::new),
            branch_id: self.branch_id.unwrap_or_else(BranchId::new),
            parent_turn_id: self.parent_turn_id,
            actor: format!("{:?}", self.actor.unwrap_or(Actor::User)),
            created_at: Utc::now().to_rfc3339(),
            finalized_at: None,
        }
    }
}

/// Builder for creating test events
#[derive(Debug, Default)]
pub struct EventBuilder {
    turn_id: Option<TurnId>,
    content: Option<String>,
}

impl EventBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn turn_id(mut self, id: TurnId) -> Self {
        self.turn_id = Some(id);
        self
    }

    pub fn content(mut self, content: &str) -> Self {
        self.content = Some(content.to_string());
        self
    }

    pub fn build_user_message(self) -> Event {
        Event::UserMessageAdded(UserMessageData {
            turn_id: self.turn_id.unwrap_or_else(TurnId::new),
            content: self.content.unwrap_or_else(|| "Test message".to_string()),
            timestamp: Timestamp::now(),
        })
    }

    pub fn build_assistant_delta(self, delta: &str) -> Event {
        Event::AssistantMessageDelta(AssistantMessageDeltaData {
            turn_id: self.turn_id.unwrap_or_else(TurnId::new),
            delta: delta.to_string(),
            sequence: 0,
            timestamp: Timestamp::now(),
        })
    }
}

/// Builder for creating test checkpoints
#[derive(Debug, Default)]
pub struct CheckpointBuilder {
    checkpoint_id: Option<CheckpointId>,
    session_id: Option<SessionId>,
    branch_id: Option<BranchId>,
    turn_id: Option<TurnId>,
    manifest_id: Option<ManifestId>,
    reason: Option<String>,
}

impl CheckpointBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn checkpoint_id(mut self, id: CheckpointId) -> Self {
        self.checkpoint_id = Some(id);
        self
    }

    pub fn session_id(mut self, id: SessionId) -> Self {
        self.session_id = Some(id);
        self
    }

    pub fn branch_id(mut self, id: BranchId) -> Self {
        self.branch_id = Some(id);
        self
    }

    pub fn turn_id(mut self, id: TurnId) -> Self {
        self.turn_id = Some(id);
        self
    }

    pub fn manifest_id(mut self, id: ManifestId) -> Self {
        self.manifest_id = Some(id);
        self
    }

    pub fn reason(mut self, reason: &str) -> Self {
        self.reason = Some(reason.to_string());
        self
    }

    pub fn build(self) -> CheckpointInfo {
        CheckpointInfo {
            id: self.checkpoint_id.unwrap_or_else(CheckpointId::new),
            session_id: self.session_id.unwrap_or_else(SessionId::new),
            branch_id: self.branch_id.unwrap_or_else(BranchId::new),
            turn_id: self.turn_id,
            manifest_id: self.manifest_id.unwrap_or_else(ManifestId::new),
            reason: self.reason.unwrap_or_else(|| "test".to_string()),
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Builder for dirty workspace simulation
#[derive(Debug, Default)]
pub struct DirtyWorkspaceBuilder {
    changes: Vec<FileChange>,
}

impl DirtyWorkspaceBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file_change(mut self, path: &str, change_type: ChangeType) -> Self {
        self.changes.push(FileChange {
            path: path.to_string(),
            ownership: ChangeOwnership::Unknown,
            before_hash: None,
            after_hash: None,
            change_type,
        });
        self
    }

    pub fn build(self) -> DirtyWorkspace {
        DirtyWorkspace::from_changes(self.changes)
    }
}

/// Builder for conflict states
#[derive(Debug, Default)]
pub struct ConflictStateBuilder {
    conflicts: Vec<ConflictDetail>,
}

impl ConflictStateBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_modified(self, path: &str) -> Self {
        let mut builder = self;
        builder.conflicts.push(ConflictDetail {
            path: path.to_string(),
            conflict_type: CoreConflictType::ModifiedBoth,
            checkpoint_content: "checkpoint version".to_string(),
            current_content: "current version".to_string(),
        });
        builder
    }

    pub fn add_deleted(self, path: &str) -> Self {
        let mut builder = self;
        builder.conflicts.push(ConflictDetail {
            path: path.to_string(),
            conflict_type: CoreConflictType::DeletedInCurrent,
            checkpoint_content: "checkpoint version".to_string(),
            current_content: String::new(),
        });
        builder
    }

    pub fn build_conflicts(self) -> Vec<ConflictDetail> {
        self.conflicts
    }
}

/// Conflict detail for testing
#[derive(Debug, Clone)]
pub struct ConflictDetail {
    pub path: String,
    pub conflict_type: CoreConflictType,
    pub checkpoint_content: String,
    pub current_content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_builder() {
        let session = SessionBuilder::new()
            .workspace_root("/test/workspace")
            .title("Test Session")
            .build();
        assert_eq!(session.workspace_root, "/test/workspace");
    }

    #[test]
    fn test_branch_builder() {
        let branch = BranchBuilder::new().name("feature-branch").build();
        assert_eq!(branch.name, "feature-branch");
    }

    #[test]
    fn test_turn_builder() {
        let turn = TurnBuilder::new().actor(Actor::Assistant).build();
        assert!(turn.actor.contains("Assistant"));
    }

    #[test]
    fn test_event_builder_user_message() {
        let event = EventBuilder::new()
            .content("Hello, world!")
            .build_user_message();
        assert!(matches!(event, Event::UserMessageAdded(_)));
    }

    #[test]
    fn test_event_builder_assistant_delta() {
        let event = EventBuilder::new()
            .turn_id(TurnId::new())
            .build_assistant_delta("Hello");
        assert!(matches!(event, Event::AssistantMessageDelta(_)));
    }

    #[test]
    fn test_checkpoint_builder() {
        let checkpoint = CheckpointBuilder::new().reason("test checkpoint").build();
        assert_eq!(checkpoint.reason, "test checkpoint");
    }

    #[test]
    fn test_dirty_workspace_builder() {
        let dirty = DirtyWorkspaceBuilder::new()
            .add_file_change("src/main.rs", ChangeType::Modified)
            .build();
        assert!(dirty.is_dirty);
    }

    #[test]
    fn test_conflict_state_builder() {
        let conflicts = ConflictStateBuilder::new()
            .add_modified("src/main.rs")
            .add_deleted("src/lib.rs")
            .build_conflicts();
        assert_eq!(conflicts.len(), 2);
    }
}
