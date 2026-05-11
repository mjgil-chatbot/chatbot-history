//! Materialized views for ai-history
//!
//! Defines read models for:
//! - Current chat transcript, branch timeline, checkpoint list
//! - File-change summary, restore history, active workspace state

use crate::{BranchId, CheckpointId, ContentHash, SessionId, TurnId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Chat transcript view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTranscriptView {
    pub session_id: SessionId,
    pub branch_id: BranchId,
    pub turns: Vec<TurnView>,
    pub total_messages: usize,
}

impl ChatTranscriptView {
    pub fn new(session_id: SessionId, branch_id: BranchId) -> Self {
        Self {
            session_id,
            branch_id,
            turns: Vec::new(),
            total_messages: 0,
        }
    }

    pub fn with_turns(mut self, turns: Vec<TurnView>) -> Self {
        self.total_messages = turns.iter().map(|t| t.messages.len()).sum();
        self.turns = turns;
        self
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

impl TurnView {
    pub fn new(turn_id: TurnId) -> Self {
        Self {
            turn_id,
            user_message: None,
            assistant_message: None,
            messages: Vec::new(),
            created_at: Utc::now(),
        }
    }
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
}

impl BranchTimelineView {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            branches: Vec::new(),
        }
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
}

impl CheckpointListView {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            checkpoints: Vec::new(),
        }
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
}

impl FileChangeSummaryView {
    pub fn new(session_id: SessionId, branch_id: BranchId) -> Self {
        Self {
            session_id,
            branch_id,
            changes: Vec::new(),
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
}

impl RestoreHistoryView {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            restores: Vec::new(),
        }
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

/// Active workspace state view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStateView {
    pub session_id: SessionId,
    pub branch_id: BranchId,
    pub current_head_turn_id: TurnId,
    pub manifest_hash: ContentHash,
    pub file_count: usize,
    pub last_modified: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_transcript_view() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let view = ChatTranscriptView::new(session_id, branch_id);
        assert_eq!(view.session_id, session_id);
        assert_eq!(view.branch_id, branch_id);
    }

    #[test]
    fn test_turn_view_new() {
        let turn_id = TurnId::new();
        let view = TurnView::new(turn_id);
        assert_eq!(view.turn_id, turn_id);
        assert!(view.user_message.is_none());
    }

    #[test]
    fn test_checkpoint_list_view() {
        let session_id = SessionId::new();
        let view = CheckpointListView::new(session_id);
        assert!(view.checkpoints.is_empty());
    }

    #[test]
    fn test_restore_history_view() {
        let session_id = SessionId::new();
        let view = RestoreHistoryView::new(session_id);
        assert!(view.restores.is_empty());
    }

    #[test]
    fn test_file_change_summary_view() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let view = FileChangeSummaryView::new(session_id, branch_id);
        assert_eq!(view.session_id, session_id);
        assert_eq!(view.branch_id, branch_id);
    }
}
