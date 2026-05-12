//! Checkpoint list views

use ai_history_core::CheckpointId;
use serde::{Deserialize, Serialize};

/// A checkpoint list item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointItem {
    pub checkpoint_id: CheckpointId,
    pub name: Option<String>,
    pub reason: String,
    pub file_count: usize,
    pub created_at: String,
    pub turn_id: Option<String>,
    pub is_safety: bool,
    pub is_named: bool,
}

/// Checkpoint list view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointListView {
    pub session_id: String,
    pub branch_id: String,
    pub checkpoints: Vec<CheckpointItem>,
    pub total_count: usize,
}

impl CheckpointListView {
    pub fn new(session_id: String, branch_id: String) -> Self {
        Self {
            session_id,
            branch_id,
            checkpoints: Vec::new(),
            total_count: 0,
        }
    }

    pub fn add_checkpoint(&mut self, checkpoint: CheckpointItem) {
        self.checkpoints.push(checkpoint);
        self.total_count += 1;
    }

    pub fn safety_checkpoints(&self) -> Vec<&CheckpointItem> {
        self.checkpoints.iter().filter(|c| c.is_safety).collect()
    }

    pub fn named_checkpoints(&self) -> Vec<&CheckpointItem> {
        self.checkpoints.iter().filter(|c| c.is_named).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_list_view_new() {
        let view = CheckpointListView::new("session-1".to_string(), "branch-1".to_string());
        assert_eq!(view.total_count, 0);
        assert!(view.checkpoints.is_empty());
    }

    #[test]
    fn test_checkpoint_filtering() {
        let mut view = CheckpointListView::new("s1".to_string(), "b1".to_string());
        view.add_checkpoint(CheckpointItem {
            checkpoint_id: CheckpointId::new(),
            name: None,
            reason: "safety".to_string(),
            file_count: 10,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            turn_id: None,
            is_safety: true,
            is_named: false,
        });
        view.add_checkpoint(CheckpointItem {
            checkpoint_id: CheckpointId::new(),
            name: Some("release-1".to_string()),
            reason: "named".to_string(),
            file_count: 20,
            created_at: "2024-01-02T00:00:00Z".to_string(),
            turn_id: None,
            is_safety: false,
            is_named: true,
        });
        assert_eq!(view.safety_checkpoints().len(), 1);
        assert_eq!(view.named_checkpoints().len(), 1);
    }

    #[test]
    fn test_checkpoint_item_creation() {
        let item = CheckpointItem {
            checkpoint_id: CheckpointId::new(),
            name: Some("test-checkpoint".to_string()),
            reason: "manual".to_string(),
            file_count: 5,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            turn_id: Some("turn-42".to_string()),
            is_safety: false,
            is_named: true,
        };
        assert_eq!(item.file_count, 5);
        assert!(item.name.is_some());
        assert!(item.turn_id.is_some());
    }

    #[test]
    fn test_checkpoint_list_view_add_multiple() {
        let mut view = CheckpointListView::new("session-1".to_string(), "branch-1".to_string());
        for i in 1..=5 {
            view.add_checkpoint(CheckpointItem {
                checkpoint_id: CheckpointId::new(),
                name: None,
                reason: format!("reason-{}", i),
                file_count: i * 10,
                created_at: format!("2024-01-0{}T00:00:00Z", i),
                turn_id: None,
                is_safety: i == 1,
                is_named: false,
            });
        }
        assert_eq!(view.total_count, 5);
        assert_eq!(view.checkpoints.len(), 5);
    }

    #[test]
    fn test_checkpoint_item_serialization() {
        let item = CheckpointItem {
            checkpoint_id: CheckpointId::new(),
            name: Some("serialization-test".to_string()),
            reason: "test".to_string(),
            file_count: 3,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            turn_id: Some("turn-1".to_string()),
            is_safety: true,
            is_named: true,
        };

        let json = serde_json::to_string(&item).unwrap();
        let parsed: CheckpointItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.reason, "test");
        assert_eq!(parsed.file_count, 3);
        assert!(parsed.is_safety);
    }

    #[test]
    fn test_checkpoint_list_view_empty_filtering() {
        let view = CheckpointListView::new("s1".to_string(), "b1".to_string());
        assert!(view.safety_checkpoints().is_empty());
        assert!(view.named_checkpoints().is_empty());
    }

    #[test]
    fn test_checkpoint_item_no_turn_id() {
        let item = CheckpointItem {
            checkpoint_id: CheckpointId::new(),
            name: None,
            reason: "safety".to_string(),
            file_count: 10,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            turn_id: None,
            is_safety: true,
            is_named: false,
        };
        assert!(item.turn_id.is_none());
        assert!(item.name.is_none());
    }
}
