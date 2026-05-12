//! Event schema definitions and validation

use super::event::*;
use ai_history_core::{Actor, CoreError, CoreResult};

/// Event schema version
pub const EVENT_SCHEMA_VERSION: &str = "1.0";

/// Schema for validating events
pub struct EventSchema;

impl EventSchema {
    /// Validate an event's structure and contents
    pub fn validate(event: &Event) -> CoreResult<()> {
        match event {
            Event::UserMessageAdded(data) => {
                if data.content.is_empty() {
                    return Err(CoreError::Deserialization(
                        "UserMessage content cannot be empty".to_string(),
                    ));
                }
            }
            Event::AssistantMessageDelta(data) => {
                if data.delta.is_empty() && data.sequence > 0 {
                    return Err(CoreError::Deserialization(
                        "Empty delta after first delta".to_string(),
                    ));
                }
            }
            Event::ToolCallRequested(data) => {
                if data.tool_name.is_empty() {
                    return Err(CoreError::Deserialization(
                        "Tool name cannot be empty".to_string(),
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Get schema version from an event
    pub fn get_version(_event: &Event) -> &'static str {
        EVENT_SCHEMA_VERSION
    }
}

/// Causation and correlation IDs for event tracking
#[derive(Debug, Clone)]
pub struct EventMetadata {
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
    pub actor: Actor,
    pub sequence_num: Option<u64>,
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self {
            causation_id: None,
            correlation_id: None,
            actor: Actor::System,
            sequence_num: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_history_core::{Timestamp, TurnId};

    #[test]
    fn test_validate_user_message_empty() {
        let event = Event::UserMessageAdded(UserMessageData {
            turn_id: TurnId::new(),
            content: "".to_string(),
            timestamp: Timestamp::now(),
        });
        assert!(EventSchema::validate(&event).is_err());
    }

    #[test]
    fn test_validate_user_message_valid() {
        let event = Event::UserMessageAdded(UserMessageData {
            turn_id: TurnId::new(),
            content: "Hello".to_string(),
            timestamp: Timestamp::now(),
        });
        assert!(EventSchema::validate(&event).is_ok());
    }

    #[test]
    fn test_validate_tool_request_empty_name() {
        let event = Event::ToolCallRequested(ToolCallRequestedData {
            turn_id: TurnId::new(),
            tool_call_id: "call_1".to_string(),
            tool_name: "".to_string(),
            args: serde_json::json!({}),
            timestamp: Timestamp::now(),
        });
        assert!(EventSchema::validate(&event).is_err());
    }

    #[test]
    fn test_event_metadata_default() {
        let metadata = EventMetadata::default();
        assert!(metadata.causation_id.is_none());
        assert!(metadata.correlation_id.is_none());
        assert_eq!(metadata.actor, Actor::System);
    }
}
