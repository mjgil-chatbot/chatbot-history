//! Materialized view persistence for read models
//!
//! Task 34: Implement materialized-view persistence
//!
//! ## Design
//!
//! This module provides persistence for read models (materialized views):
//! - Chat transcript view
//! - Branch timeline view
//! - Checkpoint list view
//! - File change summary view
//! - Restore history view
//! - Workspace state view
//!
//! Views can be rebuilt from events via replay, or incrementally updated
//! when new events are appended.

use ai_history_core::{BranchId, CheckpointId, ContentHash, CoreResult, SessionId, TurnId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// View type identifiers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ViewType {
    ChatTranscript,
    BranchTimeline,
    CheckpointList,
    FileChangeSummary,
    RestoreHistory,
    WorkspaceState,
}

/// Persisted view metadata
#[derive(Debug, Clone)]
pub struct ViewMetadata {
    pub view_type: ViewType,
    pub session_id: SessionId,
    pub branch_id: Option<BranchId>,
    pub updated_at: String,
    pub version: u64,
}

/// Trait for materialized view storage
pub trait ViewStore {
    /// Save a view to storage
    fn save_view(&mut self, metadata: &ViewMetadata, data: &[u8]) -> CoreResult<()>;

    /// Load a view from storage
    fn load_view(
        &self,
        view_type: ViewType,
        session_id: SessionId,
        branch_id: Option<BranchId>,
    ) -> CoreResult<Option<Vec<u8>>>;

    /// Delete old views for a session
    fn delete_views_for_session(&mut self, session_id: SessionId) -> CoreResult<()>;

    /// List all views for a session
    fn list_views(&self, session_id: SessionId) -> CoreResult<Vec<ViewMetadata>>;
}

/// Chat transcript view - assembled from events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTranscriptView {
    pub session_id: SessionId,
    pub branch_id: BranchId,
    pub turns: Vec<TurnView>,
    pub total_messages: usize,
    pub version: u64,
}

impl ChatTranscriptView {
    pub fn new(session_id: SessionId, branch_id: BranchId) -> Self {
        Self {
            session_id,
            branch_id,
            turns: Vec::new(),
            total_messages: 0,
            version: 1,
        }
    }

    pub fn with_turns(mut self, turns: Vec<TurnView>) -> Self {
        self.total_messages = turns.iter().map(|t| t.messages.len()).sum();
        self.turns = turns;
        self
    }

    /// Rebuild from event stream
    pub fn from_events(
        session_id: SessionId,
        branch_id: BranchId,
        events: &[super::Event],
    ) -> Self {
        let mut view = Self::new(session_id, branch_id);
        for event in events {
            view.apply_event(event);
        }
        view
    }

    fn apply_event(&mut self, event: &super::Event) {
        use ai_history_events::Event;

        match event {
            Event::TurnCreated(data) => {
                self.turns.push(TurnView {
                    turn_id: data.turn_id,
                    user_message: None,
                    assistant_message: None,
                    messages: Vec::new(),
                    created_at: DateTime::parse_from_rfc3339(&data.timestamp.to_string())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                });
            }
            Event::UserMessageAdded(data) => {
                if let Some(turn) = self.turns.iter_mut().find(|t| t.turn_id == data.turn_id) {
                    turn.user_message = Some(data.content.clone());
                    turn.messages.push(MessageView {
                        role: "user".to_string(),
                        content: data.content.clone(),
                        timestamp: DateTime::parse_from_rfc3339(&data.timestamp.to_string())
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    });
                }
            }
            Event::AssistantMessageCompleted(data) => {
                if let Some(turn) = self.turns.iter_mut().find(|t| t.turn_id == data.turn_id) {
                    turn.assistant_message = Some(data.content.clone());
                    turn.messages.push(MessageView {
                        role: "assistant".to_string(),
                        content: data.content.clone(),
                        timestamp: DateTime::parse_from_rfc3339(&data.timestamp.to_string())
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    });
                }
            }
            _ => {}
        }
        self.total_messages = self.turns.iter().map(|t| t.messages.len()).sum();
        self.version += 1;
    }
}

/// Turn view in transcript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnView {
    pub turn_id: TurnId,
    pub user_message: Option<String>,
    pub assistant_message: Option<String>,
    pub messages: Vec<MessageView>,
    pub created_at: DateTime<Utc>,
}

/// Message view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageView {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Branch timeline view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchTimelineView {
    pub session_id: SessionId,
    pub branches: Vec<BranchView>,
    pub active_branch_id: Option<BranchId>,
    pub version: u64,
}

impl BranchTimelineView {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            branches: Vec::new(),
            active_branch_id: None,
            version: 1,
        }
    }

    pub fn from_events(session_id: SessionId, events: &[super::Event]) -> Self {
        let mut view = Self::new(session_id);
        for event in events {
            view.apply_event(event);
        }
        view
    }

    fn apply_event(&mut self, event: &super::Event) {
        use ai_history_events::Event;

        match event {
            Event::BranchCreated(data) => {
                self.branches.push(BranchView {
                    branch_id: data.branch_id,
                    name: data.name.clone(),
                    parent_branch_id: data.parent_branch_id,
                    head_turn_id: None,
                    created_at: DateTime::parse_from_rfc3339(&data.timestamp.to_string())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    is_active: false,
                });
            }
            Event::BranchHeadMoved(data) => {
                if let Some(branch) = self
                    .branches
                    .iter_mut()
                    .find(|b| b.branch_id == data.branch_id)
                {
                    branch.head_turn_id = Some(data.new_turn_id);
                }
            }
            _ => {}
        }
        self.version += 1;
    }
}

/// Branch view in timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchView {
    pub branch_id: BranchId,
    pub name: String,
    pub parent_branch_id: Option<BranchId>,
    pub head_turn_id: Option<TurnId>,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

/// Checkpoint list view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointListView {
    pub session_id: SessionId,
    pub checkpoints: Vec<CheckpointView>,
    pub version: u64,
}

impl CheckpointListView {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            checkpoints: Vec::new(),
            version: 1,
        }
    }

    pub fn from_events(session_id: SessionId, events: &[super::Event]) -> Self {
        let mut view = Self::new(session_id);
        for event in events {
            view.apply_event(event);
        }
        view
    }

    fn apply_event(&mut self, event: &super::Event) {
        use ai_history_events::Event;

        match event {
            Event::CheckpointCreated(data) => {
                self.checkpoints.push(CheckpointView {
                    checkpoint_id: data.checkpoint_id,
                    name: None,
                    checkpoint_type: data.reason.clone(),
                    turn_id: data.turn_id,
                    created_at: DateTime::parse_from_rfc3339(&data.timestamp.to_string())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    is_protected: false,
                });
            }
            _ => {}
        }
        self.version += 1;
    }
}

/// Checkpoint view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointView {
    pub checkpoint_id: CheckpointId,
    pub name: Option<String>,
    pub checkpoint_type: String,
    pub turn_id: Option<TurnId>,
    pub created_at: DateTime<Utc>,
    pub is_protected: bool,
}

/// File change summary view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeSummaryView {
    pub session_id: SessionId,
    pub branch_id: BranchId,
    pub changes: Vec<FileChangeView>,
    pub version: u64,
}

impl FileChangeSummaryView {
    pub fn new(session_id: SessionId, branch_id: BranchId) -> Self {
        Self {
            session_id,
            branch_id,
            changes: Vec::new(),
            version: 1,
        }
    }
}

/// File change view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeView {
    pub path: String,
    pub change_type: String,
    pub hash: Option<ContentHash>,
}

/// Restore history view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreHistoryView {
    pub session_id: SessionId,
    pub restores: Vec<RestoreView>,
    pub version: u64,
}

impl RestoreHistoryView {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            restores: Vec::new(),
            version: 1,
        }
    }

    pub fn from_events(session_id: SessionId, events: &[super::Event]) -> Self {
        let mut view = Self::new(session_id);
        for event in events {
            view.apply_event(event);
        }
        view
    }

    fn apply_event(&mut self, event: &super::Event) {
        use ai_history_events::Event;

        match event {
            Event::RestoreStarted(data) => {
                self.restores.push(RestoreView {
                    checkpoint_id: data.checkpoint_id,
                    mode: data.mode.clone(),
                    executed_at: DateTime::parse_from_rfc3339(&data.timestamp.to_string())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    success: false,
                    files_restored: 0,
                });
            }
            Event::RestoreCompleted(data) => {
                if let Some(restore) = self
                    .restores
                    .iter_mut()
                    .find(|r| r.checkpoint_id == data.checkpoint_id)
                {
                    restore.success = true;
                }
            }
            Event::RestoreFailed(data) => {
                if let Some(restore) = self
                    .restores
                    .iter_mut()
                    .find(|r| r.checkpoint_id == data.checkpoint_id)
                {
                    restore.success = false;
                }
            }
            _ => {}
        }
        self.version += 1;
    }
}

/// Restore view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreView {
    pub checkpoint_id: CheckpointId,
    pub mode: String,
    pub executed_at: DateTime<Utc>,
    pub success: bool,
    pub files_restored: usize,
}

/// Workspace state view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStateView {
    pub session_id: SessionId,
    pub branch_id: BranchId,
    pub current_head_turn_id: Option<TurnId>,
    pub manifest_hash: Option<ContentHash>,
    pub file_count: usize,
    pub last_modified: DateTime<Utc>,
    pub version: u64,
}

impl WorkspaceStateView {
    pub fn new(session_id: SessionId, branch_id: BranchId) -> Self {
        Self {
            session_id,
            branch_id,
            current_head_turn_id: None,
            manifest_hash: None,
            file_count: 0,
            last_modified: Utc::now(),
            version: 1,
        }
    }
}

/// View store implementation using SQLite
pub struct SqliteViewStore<'a> {
    conn: &'a rusqlite::Connection,
}

impl<'a> SqliteViewStore<'a> {
    pub fn new(conn: &'a rusqlite::Connection) -> Self {
        Self { conn }
    }

    fn view_key(
        view_type: &ViewType,
        session_id: SessionId,
        branch_id: Option<BranchId>,
    ) -> String {
        let branch_str = branch_id
            .map(|b| b.to_string())
            .unwrap_or_else(|| "none".to_string());
        format!("{:?}:{}:{}", view_type, session_id, branch_str)
    }
}

impl<'a> ViewStore for SqliteViewStore<'a> {
    fn save_view(&mut self, metadata: &ViewMetadata, data: &[u8]) -> CoreResult<()> {
        let key = Self::view_key(&metadata.view_type, metadata.session_id, metadata.branch_id);
        let view_type_str = format!("{:?}", metadata.view_type);

        self.conn
            .execute(
                "INSERT OR REPLACE INTO materialized_views (view_key, view_type, session_id, branch_id, data, updated_at, version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    key,
                    view_type_str,
                    metadata.session_id.to_string(),
                    metadata.branch_id.map(|b| b.to_string()),
                    data,
                    metadata.updated_at,
                    metadata.version as i64,
                ],
            )
            .map_err(|e| ai_history_core::CoreError::Storage(e.to_string()))?;

        Ok(())
    }

    fn load_view(
        &self,
        view_type: ViewType,
        session_id: SessionId,
        branch_id: Option<BranchId>,
    ) -> CoreResult<Option<Vec<u8>>> {
        let key = Self::view_key(&view_type, session_id, branch_id);

        let mut stmt = self
            .conn
            .prepare("SELECT data FROM materialized_views WHERE view_key = ?1")
            .map_err(|e| ai_history_core::CoreError::Storage(e.to_string()))?;

        let result = stmt.query_row(rusqlite::params![key], |row| Ok(row.get::<_, Vec<u8>>(0)?));

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ai_history_core::CoreError::Storage(e.to_string())),
        }
    }

    fn delete_views_for_session(&mut self, session_id: SessionId) -> CoreResult<()> {
        self.conn
            .execute(
                "DELETE FROM materialized_views WHERE session_id = ?1",
                rusqlite::params![session_id.to_string()],
            )
            .map_err(|e| ai_history_core::CoreError::Storage(e.to_string()))?;

        Ok(())
    }

    fn list_views(&self, session_id: SessionId) -> CoreResult<Vec<ViewMetadata>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT view_key, view_type, branch_id, updated_at, version
                 FROM materialized_views WHERE session_id = ?1",
            )
            .map_err(|e| ai_history_core::CoreError::Storage(e.to_string()))?;

        let views = stmt
            .query_map(rusqlite::params![session_id.to_string()], |row| {
                let _view_key: String = row.get(0)?;
                let view_type_str: String = row.get(1)?;
                let branch_id_str: Option<String> = row.get(2)?;
                let updated_at: String = row.get(3)?;
                let version = row.get::<_, i64>(4)? as u64;

                let view_type = match view_type_str.as_str() {
                    "ChatTranscript" => ViewType::ChatTranscript,
                    "BranchTimeline" => ViewType::BranchTimeline,
                    "CheckpointList" => ViewType::CheckpointList,
                    "FileChangeSummary" => ViewType::FileChangeSummary,
                    "RestoreHistory" => ViewType::RestoreHistory,
                    "WorkspaceState" => ViewType::WorkspaceState,
                    _ => ViewType::ChatTranscript,
                };

                let branch_id = branch_id_str.map(|s| s.parse().ok()).flatten();

                Ok(ViewMetadata {
                    view_type,
                    session_id,
                    branch_id,
                    updated_at,
                    version,
                })
            })
            .map_err(|e| ai_history_core::CoreError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ai_history_core::CoreError::Storage(e.to_string()))?;

        Ok(views)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_history_core::{Actor, Timestamp};
    use ai_history_events::{
        AssistantMessageCompletedData, Event, TurnCreatedData, UserMessageData,
    };
    use tempfile::tempdir;

    fn create_test_store() -> super::super::SqliteStore {
        super::super::SqliteStore::in_memory().unwrap()
    }

    #[test]
    fn test_chat_transcript_from_events() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn_id = TurnId::new();

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

        let view = ChatTranscriptView::from_events(session_id, branch_id, &events);
        assert_eq!(view.turns.len(), 1);
        assert_eq!(view.turns[0].user_message, Some("Hello".to_string()));
        assert_eq!(
            view.turns[0].assistant_message,
            Some("Hi there!".to_string())
        );
    }

    #[test]
    fn test_checkpoint_list_from_events() {
        use ai_history_core::ManifestId;
        use ai_history_events::CheckpointCreatedData;

        let session_id = SessionId::new();
        let checkpoint_id = CheckpointId::new();
        let manifest_id = ManifestId::new();

        let events = vec![Event::CheckpointCreated(CheckpointCreatedData {
            checkpoint_id,
            reason: "test checkpoint".to_string(),
            manifest_id,
            turn_id: None,
            timestamp: Timestamp::now(),
        })];

        let view = CheckpointListView::from_events(session_id, &events);
        assert_eq!(view.checkpoints.len(), 1);
        assert_eq!(view.checkpoints[0].checkpoint_id, checkpoint_id);
    }

    #[test]
    fn test_branch_timeline_from_events() {
        use ai_history_events::{BranchCreatedData, BranchHeadMovedData};

        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn_id = TurnId::new();

        let events = vec![
            Event::BranchCreated(BranchCreatedData {
                branch_id,
                name: "main".to_string(),
                parent_branch_id: None,
                from_turn_id: turn_id,
                timestamp: Timestamp::now(),
            }),
            Event::BranchHeadMoved(BranchHeadMovedData {
                branch_id,
                previous_turn_id: turn_id,
                new_turn_id: TurnId::new(),
                timestamp: Timestamp::now(),
            }),
        ];

        let view = BranchTimelineView::from_events(session_id, &events);
        assert_eq!(view.branches.len(), 1);
        assert_eq!(view.branches[0].name, "main");
    }

    #[test]
    fn test_restore_history_from_events() {
        use ai_history_events::{RestoreCompletedData, RestoreStartedData};

        let session_id = SessionId::new();
        let checkpoint_id = CheckpointId::new();

        let events = vec![
            Event::RestoreStarted(RestoreStartedData {
                checkpoint_id,
                mode: "Both".to_string(),
                timestamp: Timestamp::now(),
            }),
            Event::RestoreCompleted(RestoreCompletedData {
                checkpoint_id,
                timestamp: Timestamp::now(),
            }),
        ];

        let view = RestoreHistoryView::from_events(session_id, &events);
        assert_eq!(view.restores.len(), 1);
        assert!(view.restores[0].success);
    }

    #[test]
    fn test_view_serialization_roundtrip() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        let view = ChatTranscriptView::new(session_id, branch_id);
        let data = serde_json::to_vec(&view).unwrap();
        let parsed: ChatTranscriptView = serde_json::from_slice(&data).unwrap();
        assert_eq!(parsed.session_id, session_id);
        assert_eq!(parsed.branch_id, branch_id);
    }

    #[test]
    fn test_view_metadata() {
        let metadata = ViewMetadata {
            view_type: ViewType::ChatTranscript,
            session_id: SessionId::new(),
            branch_id: Some(BranchId::new()),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            version: 1,
        };
        assert_eq!(metadata.version, 1);
    }
}
