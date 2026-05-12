//! Restore modal views

use ai_history_core::CheckpointId;
use serde::{Deserialize, Serialize};

/// Available restore modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreModeUi {
    ConversationOnly,
    WorkspaceOnly,
    Both,
    Fork,
}

impl Default for RestoreModeUi {
    fn default() -> Self {
        Self::Both
    }
}

/// Restore modal state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreModalState {
    pub checkpoint_id: CheckpointId,
    pub checkpoint_name: Option<String>,
    pub checkpoint_reason: String,
    pub selected_mode: RestoreModeUi,
    pub has_conflicts: bool,
    pub affected_files_count: usize,
    pub is_executing: bool,
    pub show_confirm: bool,
}

impl RestoreModalState {
    pub fn new(checkpoint_id: CheckpointId, reason: String) -> Self {
        Self {
            checkpoint_id,
            checkpoint_name: None,
            checkpoint_reason: reason,
            selected_mode: RestoreModeUi::default(),
            has_conflicts: false,
            affected_files_count: 0,
            is_executing: false,
            show_confirm: false,
        }
    }

    pub fn set_mode(&mut self, mode: RestoreModeUi) {
        self.selected_mode = mode;
    }

    pub fn set_executing(&mut self, executing: bool) {
        self.is_executing = executing;
    }

    pub fn request_confirmation(&mut self) {
        self.show_confirm = true;
    }
}

/// Restore history item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreHistoryItem {
    pub checkpoint_id: CheckpointId,
    pub mode: RestoreModeUi,
    pub executed_at: String,
    pub success: bool,
    pub files_restored: usize,
}

/// Restore history view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreHistoryView {
    pub session_id: String,
    pub items: Vec<RestoreHistoryItem>,
}

impl RestoreHistoryView {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            items: Vec::new(),
        }
    }

    pub fn add_restore(&mut self, item: RestoreHistoryItem) {
        self.items.push(item);
    }

    pub fn success_count(&self) -> usize {
        self.items.iter().filter(|i| i.success).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restore_modal_state_new() {
        let state = RestoreModalState::new(CheckpointId::new(), "before_user_prompt".to_string());
        assert_eq!(state.checkpoint_reason, "before_user_prompt");
        assert_eq!(state.selected_mode, RestoreModeUi::Both);
        assert!(!state.is_executing);
    }

    #[test]
    fn test_restore_history_view_success_count() {
        let mut view = RestoreHistoryView::new("session-1".to_string());
        view.add_restore(RestoreHistoryItem {
            checkpoint_id: CheckpointId::new(),
            mode: RestoreModeUi::Both,
            executed_at: "2024-01-01T00:00:00Z".to_string(),
            success: true,
            files_restored: 5,
        });
        view.add_restore(RestoreHistoryItem {
            checkpoint_id: CheckpointId::new(),
            mode: RestoreModeUi::WorkspaceOnly,
            executed_at: "2024-01-02T00:00:00Z".to_string(),
            success: false,
            files_restored: 0,
        });
        assert_eq!(view.success_count(), 1);
    }

    #[test]
    fn test_restore_modal_state_set_mode() {
        let mut state = RestoreModalState::new(CheckpointId::new(), "test".to_string());
        state.set_mode(RestoreModeUi::WorkspaceOnly);
        assert_eq!(state.selected_mode, RestoreModeUi::WorkspaceOnly);

        state.set_mode(RestoreModeUi::Fork);
        assert_eq!(state.selected_mode, RestoreModeUi::Fork);
    }

    #[test]
    fn test_restore_modal_state_set_executing() {
        let mut state = RestoreModalState::new(CheckpointId::new(), "test".to_string());
        assert!(!state.is_executing);

        state.set_executing(true);
        assert!(state.is_executing);

        state.set_executing(false);
        assert!(!state.is_executing);
    }

    #[test]
    fn test_restore_modal_state_request_confirmation() {
        let mut state = RestoreModalState::new(CheckpointId::new(), "test".to_string());
        assert!(!state.show_confirm);

        state.request_confirmation();
        assert!(state.show_confirm);
    }

    #[test]
    fn test_restore_history_item_creation() {
        let item = RestoreHistoryItem {
            checkpoint_id: CheckpointId::new(),
            mode: RestoreModeUi::ConversationOnly,
            executed_at: "2024-01-01T00:00:00Z".to_string(),
            success: true,
            files_restored: 10,
        };
        assert_eq!(item.mode, RestoreModeUi::ConversationOnly);
        assert!(item.success);
        assert_eq!(item.files_restored, 10);
    }

    #[test]
    fn test_restore_history_view_empty() {
        let view = RestoreHistoryView::new("session-1".to_string());
        assert_eq!(view.success_count(), 0);
        assert!(view.items.is_empty());
    }

    #[test]
    fn test_restore_history_view_multiple_items() {
        let mut view = RestoreHistoryView::new("session-1".to_string());
        for i in 1..=5 {
            view.add_restore(RestoreHistoryItem {
                checkpoint_id: CheckpointId::new(),
                mode: RestoreModeUi::Both,
                executed_at: format!("2024-01-0{}T00:00:00Z", i),
                success: i % 2 == 0,
                files_restored: i * 2,
            });
        }
        assert_eq!(view.items.len(), 5);
        assert_eq!(view.success_count(), 2);
    }

    #[test]
    fn test_restore_mode_ui_variants() {
        assert_eq!(
            RestoreModeUi::ConversationOnly,
            RestoreModeUi::ConversationOnly
        );
        assert_eq!(RestoreModeUi::WorkspaceOnly, RestoreModeUi::WorkspaceOnly);
        assert_eq!(RestoreModeUi::Both, RestoreModeUi::Both);
        assert_eq!(RestoreModeUi::Fork, RestoreModeUi::Fork);
        assert_ne!(
            RestoreModeUi::ConversationOnly,
            RestoreModeUi::WorkspaceOnly
        );
    }

    #[test]
    fn test_restore_modal_state_with_all_fields() {
        let checkpoint_id = CheckpointId::new();
        let mut state = RestoreModalState::new(checkpoint_id.clone(), "test reason".to_string());
        state.checkpoint_name = Some("my checkpoint".to_string());
        state.has_conflicts = true;
        state.affected_files_count = 7;

        assert_eq!(state.checkpoint_id, checkpoint_id);
        assert_eq!(state.checkpoint_name, Some("my checkpoint".to_string()));
        assert!(state.has_conflicts);
        assert_eq!(state.affected_files_count, 7);
    }

    #[test]
    fn test_restore_history_item_serialization() {
        let item = RestoreHistoryItem {
            checkpoint_id: CheckpointId::new(),
            mode: RestoreModeUi::Fork,
            executed_at: "2024-01-01T00:00:00Z".to_string(),
            success: true,
            files_restored: 5,
        };

        let json = serde_json::to_string(&item).unwrap();
        let parsed: RestoreHistoryItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.mode, RestoreModeUi::Fork);
        assert!(parsed.success);
    }
}
