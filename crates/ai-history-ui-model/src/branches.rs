//! Branch timeline views

use ai_history_core::BranchId;
use serde::{Deserialize, Serialize};

/// Branch item in the timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchItem {
    pub branch_id: BranchId,
    pub name: String,
    pub parent_branch_id: Option<BranchId>,
    pub head_turn_id: Option<String>,
    pub created_at: String,
    pub is_active: bool,
    pub turn_count: usize,
}

/// Branch timeline view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchTimelineView {
    pub session_id: String,
    pub branches: Vec<BranchItem>,
    pub active_branch_id: Option<BranchId>,
    pub total_count: usize,
}

impl BranchTimelineView {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            branches: Vec::new(),
            active_branch_id: None,
            total_count: 0,
        }
    }

    pub fn add_branch(&mut self, branch: BranchItem) {
        self.branches.push(branch);
        self.total_count += 1;
    }

    pub fn set_active(&mut self, branch_id: BranchId) {
        for b in &mut self.branches {
            b.is_active = b.branch_id == branch_id;
        }
        self.active_branch_id = Some(branch_id);
    }

    pub fn get_branch(&self, branch_id: BranchId) -> Option<&BranchItem> {
        self.branches.iter().find(|b| b.branch_id == branch_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_timeline_view_new() {
        let view = BranchTimelineView::new("session-123".to_string());
        assert_eq!(view.session_id, "session-123");
        assert!(view.active_branch_id.is_none());
    }

    #[test]
    fn test_branch_timeline_view_add_branch() {
        let mut view = BranchTimelineView::new("session-123".to_string());
        view.add_branch(BranchItem {
            branch_id: BranchId::new(),
            name: "main".to_string(),
            parent_branch_id: None,
            head_turn_id: Some("turn-1".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            is_active: true,
            turn_count: 5,
        });
        assert_eq!(view.total_count, 1);
    }

    #[test]
    fn test_branch_timeline_view_set_active() {
        let mut view = BranchTimelineView::new("session-123".to_string());
        let branch_id = BranchId::new();
        view.add_branch(BranchItem {
            branch_id,
            name: "main".to_string(),
            parent_branch_id: None,
            head_turn_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            is_active: false,
            turn_count: 5,
        });
        view.set_active(branch_id);
        let branch = view.get_branch(branch_id).unwrap();
        assert!(branch.is_active);
    }

    #[test]
    fn test_branch_item_creation() {
        let parent_id = BranchId::new();
        let item = BranchItem {
            branch_id: BranchId::new(),
            name: "feature-x".to_string(),
            parent_branch_id: Some(parent_id),
            head_turn_id: Some("turn-99".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            is_active: false,
            turn_count: 42,
        };
        assert_eq!(item.name, "feature-x");
        assert_eq!(item.turn_count, 42);
        assert!(item.parent_branch_id.is_some());
        assert!(item.head_turn_id.is_some());
    }

    #[test]
    fn test_branch_timeline_view_get_nonexistent() {
        let view = BranchTimelineView::new("session-123".to_string());
        let result = view.get_branch(BranchId::new());
        assert!(result.is_none());
    }

    #[test]
    fn test_branch_timeline_view_multiple_branches() {
        let mut view = BranchTimelineView::new("session-1".to_string());
        let main_id = BranchId::new();
        let feature_id = BranchId::new();

        view.add_branch(BranchItem {
            branch_id: main_id,
            name: "main".to_string(),
            parent_branch_id: None,
            head_turn_id: Some("turn-10".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            is_active: true,
            turn_count: 10,
        });
        view.add_branch(BranchItem {
            branch_id: feature_id,
            name: "feature-a".to_string(),
            parent_branch_id: Some(main_id),
            head_turn_id: Some("turn-5".to_string()),
            created_at: "2024-01-02T00:00:00Z".to_string(),
            is_active: false,
            turn_count: 5,
        });

        assert_eq!(view.total_count, 2);
        assert_eq!(view.branches.len(), 2);

        let main = view.get_branch(main_id).unwrap();
        let feature = view.get_branch(feature_id).unwrap();
        assert_eq!(main.name, "main");
        assert_eq!(feature.name, "feature-a");
        assert!(main.parent_branch_id.is_none());
        assert!(feature.parent_branch_id.is_some());
    }

    #[test]
    fn test_branch_item_serialization() {
        let item = BranchItem {
            branch_id: BranchId::new(),
            name: "test-branch".to_string(),
            parent_branch_id: None,
            head_turn_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            is_active: true,
            turn_count: 15,
        };

        let json = serde_json::to_string(&item).unwrap();
        let parsed: BranchItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test-branch");
        assert_eq!(parsed.turn_count, 15);
        assert!(parsed.is_active);
    }

    #[test]
    fn test_branch_set_active_updates_all_others() {
        let mut view = BranchTimelineView::new("session-1".to_string());
        let id1 = BranchId::new();
        let id2 = BranchId::new();

        view.add_branch(BranchItem {
            branch_id: id1,
            name: "branch-1".to_string(),
            parent_branch_id: None,
            head_turn_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            is_active: true,
            turn_count: 5,
        });
        view.add_branch(BranchItem {
            branch_id: id2,
            name: "branch-2".to_string(),
            parent_branch_id: None,
            head_turn_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            is_active: true,
            turn_count: 3,
        });

        view.set_active(id2);

        let b1 = view.get_branch(id1).unwrap();
        let b2 = view.get_branch(id2).unwrap();
        assert!(!b1.is_active);
        assert!(b2.is_active);
        assert_eq!(view.active_branch_id, Some(id2));
    }
}
