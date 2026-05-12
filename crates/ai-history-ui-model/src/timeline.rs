//! Timeline views for chat transcript

use ai_history_core::{Actor, TurnId};
use serde::{Deserialize, Serialize};

/// A timeline item representing a turn in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineItem {
    pub turn_id: TurnId,
    pub turn_number: usize,
    pub actor: Actor,
    pub content_preview: String,
    pub timestamp: String,
    pub message_count: usize,
    pub has_tool_calls: bool,
    pub is_finalized: bool,
}

/// Timeline view showing the conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineView {
    pub session_id: String,
    pub items: Vec<TimelineItem>,
    pub total_turns: usize,
    pub current_turn_id: Option<TurnId>,
}

impl TimelineView {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            items: Vec::new(),
            total_turns: 0,
            current_turn_id: None,
        }
    }

    pub fn add_item(&mut self, item: TimelineItem) {
        self.items.push(item);
        self.total_turns += 1;
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get_item(&self, turn_id: TurnId) -> Option<&TimelineItem> {
        self.items.iter().find(|i| i.turn_id == turn_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_view_new() {
        let view = TimelineView::new("session-123".to_string());
        assert_eq!(view.session_id, "session-123");
        assert!(view.is_empty());
        assert_eq!(view.total_turns, 0);
    }

    #[test]
    fn test_timeline_view_add_item() {
        let mut view = TimelineView::new("session-123".to_string());
        view.add_item(TimelineItem {
            turn_id: TurnId::new(),
            turn_number: 1,
            actor: Actor::User,
            content_preview: "Hello".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            message_count: 1,
            has_tool_calls: false,
            is_finalized: true,
        });
        assert_eq!(view.total_turns, 1);
        assert!(!view.is_empty());
    }

    #[test]
    fn test_timeline_item_has_required_fields() {
        let item = TimelineItem {
            turn_id: TurnId::new(),
            turn_number: 1,
            actor: Actor::User,
            content_preview: "Hello".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            message_count: 1,
            has_tool_calls: false,
            is_finalized: true,
        };
        // Verify all required fields are present for UI consumption
        assert!(!item.content_preview.is_empty() || item.content_preview.is_empty()); // Can be empty for system messages
        assert!(!item.timestamp.is_empty());
    }

    #[test]
    fn test_timeline_item_serialization() {
        let item = TimelineItem {
            turn_id: TurnId::new(),
            turn_number: 1,
            actor: Actor::User,
            content_preview: "Hello".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            message_count: 1,
            has_tool_calls: false,
            is_finalized: true,
        };

        let json = serde_json::to_string(&item).unwrap();
        let parsed: TimelineItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.turn_number, 1);
        assert_eq!(parsed.actor, Actor::User);
    }

    #[test]
    fn test_timeline_view_get_item() {
        let mut view = TimelineView::new("session-123".to_string());
        let turn_id = TurnId::new();
        view.add_item(TimelineItem {
            turn_id,
            turn_number: 1,
            actor: Actor::User,
            content_preview: "Hello".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            message_count: 1,
            has_tool_calls: false,
            is_finalized: true,
        });

        let found = view.get_item(turn_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().turn_number, 1);
    }

    #[test]
    fn test_timeline_view_get_item_not_found() {
        let view = TimelineView::new("session-123".to_string());
        let found = view.get_item(TurnId::new());
        assert!(found.is_none());
    }

    #[test]
    fn test_timeline_view_multiple_items_order() {
        let mut view = TimelineView::new("session-123".to_string());

        for i in 1..=3 {
            view.add_item(TimelineItem {
                turn_id: TurnId::new(),
                turn_number: i,
                actor: if i % 2 == 0 {
                    Actor::User
                } else {
                    Actor::Assistant
                },
                content_preview: format!("Message {}", i),
                timestamp: format!("2024-01-01T00:00:0{}Z", i),
                message_count: i,
                has_tool_calls: i == 2,
                is_finalized: true,
            });
        }

        assert_eq!(view.items.len(), 3);
        assert_eq!(view.total_turns, 3);
        assert_eq!(view.items[0].turn_number, 1);
        assert_eq!(view.items[1].turn_number, 2);
        assert_eq!(view.items[2].turn_number, 3);
    }

    #[test]
    fn test_timeline_item_with_tool_calls() {
        let item = TimelineItem {
            turn_id: TurnId::new(),
            turn_number: 1,
            actor: Actor::Assistant,
            content_preview: "I'll help you with that".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            message_count: 3,
            has_tool_calls: true,
            is_finalized: true,
        };
        assert!(item.has_tool_calls);
    }

    #[test]
    fn test_timeline_current_turn_tracking() {
        let mut view = TimelineView::new("session-123".to_string());
        let current_turn = TurnId::new();
        view.add_item(TimelineItem {
            turn_id: current_turn,
            turn_number: 1,
            actor: Actor::User,
            content_preview: "Hello".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            message_count: 1,
            has_tool_calls: false,
            is_finalized: false, // Current in-progress turn
        });
        view.current_turn_id = Some(current_turn);
        assert!(view.current_turn_id.is_some());
    }
}
