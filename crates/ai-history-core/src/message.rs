//! Message model for ai-history
//!
//! Task 17: Complete message modeling
//! - Store user, assistant, system, tool, summary, and internal messages as first-class records
//! - Support streamed deltas and final assembly

use super::TurnId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Message types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    User,
    Assistant,
    System,
    Tool,
    Summary,
    Internal,
}

impl Default for MessageType {
    fn default() -> Self {
        Self::User
    }
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::User => "user",
            MessageType::Assistant => "assistant",
            MessageType::System => "system",
            MessageType::Tool => "tool",
            MessageType::Summary => "summary",
            MessageType::Internal => "internal",
        }
    }
}

/// Message builder for constructing messages
#[derive(Debug, Clone)]
pub struct MessageBuilder {
    turn_id: TurnId,
    message_type: MessageType,
    content: Option<MessageContent>,
    metadata: HashMap<String, String>,
}

impl MessageBuilder {
    pub fn new(turn_id: TurnId, message_type: MessageType) -> Self {
        Self {
            turn_id,
            message_type,
            content: None,
            metadata: HashMap::new(),
        }
    }

    pub fn content(mut self, content: MessageContent) -> Self {
        self.content = Some(content);
        self
    }

    pub fn text_content(mut self, text: &str) -> Self {
        self.content = Some(MessageContent::Text(text.to_string()));
        self
    }

    pub fn stream_content(mut self) -> Self {
        self.content = Some(MessageContent::Stream {
            deltas: Vec::new(),
            final_content: None,
        });
        self
    }

    pub fn metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn build(self) -> Message {
        Message {
            id: MessageId::new(),
            turn_id: self.turn_id,
            message_type: self.message_type,
            content: self.content.unwrap_or(MessageContent::Text(String::new())),
            metadata: self.metadata,
            created_at: Utc::now(),
        }
    }
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    id: MessageId,
    turn_id: TurnId,
    message_type: MessageType,
    content: MessageContent,
    metadata: HashMap<String, String>,
    created_at: DateTime<Utc>,
}

impl Message {
    pub fn id(&self) -> MessageId {
        self.id
    }

    pub fn turn_id(&self) -> TurnId {
        self.turn_id
    }

    pub fn message_type(&self) -> MessageType {
        self.message_type
    }

    pub fn content(&self) -> &MessageContent {
        &self.content
    }

    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn new(turn_id: TurnId, message_type: MessageType, content: MessageContent) -> Self {
        Self {
            id: MessageId::new(),
            turn_id,
            message_type,
            content,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn builder(turn_id: TurnId, message_type: MessageType) -> MessageBuilder {
        MessageBuilder::new(turn_id, message_type)
    }

    pub fn user_message(turn_id: TurnId, content: &str) -> Self {
        Self::new(
            turn_id,
            MessageType::User,
            MessageContent::Text(content.to_string()),
        )
    }

    pub fn assistant_message(turn_id: TurnId, content: &str) -> Self {
        Self::new(
            turn_id,
            MessageType::Assistant,
            MessageContent::Text(content.to_string()),
        )
    }

    pub fn system_message(turn_id: TurnId, content: &str) -> Self {
        Self::new(
            turn_id,
            MessageType::System,
            MessageContent::Text(content.to_string()),
        )
    }

    pub fn internal_message(turn_id: TurnId, content: &str) -> Self {
        Self::new(
            turn_id,
            MessageType::Internal,
            MessageContent::Text(content.to_string()),
        )
    }

    pub fn tool_message(turn_id: TurnId, tool_call_id: &str, content: &str) -> Self {
        let mut msg = Self::new(
            turn_id,
            MessageType::Tool,
            MessageContent::Text(content.to_string()),
        );
        msg.metadata
            .insert("tool_call_id".to_string(), tool_call_id.to_string());
        msg
    }

    pub fn summary_message(turn_id: TurnId, content: &str) -> Self {
        Self::new(
            turn_id,
            MessageType::Summary,
            MessageContent::Text(content.to_string()),
        )
    }

    pub fn set_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }
}

/// Message ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(uuid::Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "msg:{}", self.0)
    }
}

impl std::str::FromStr for MessageId {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = s.strip_prefix("msg:") {
            rest.parse().map(Self).map_err(|_| "invalid UUID")
        } else {
            s.parse().map(Self).map_err(|_| "invalid UUID")
        }
    }
}

/// Message content - supports both complete text and streamed deltas
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessageContent {
    /// Plain text content
    Text(String),
    /// Streamed deltas being accumulated
    Stream {
        deltas: Vec<String>,
        final_content: Option<String>,
    },
    /// Tool call result
    ToolResult {
        tool_call_id: String,
        tool_name: String,
        result: String,
    },
}

impl MessageContent {
    pub fn text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(s) => Some(s),
            MessageContent::Stream {
                final_content: Some(s),
                ..
            } => Some(s),
            _ => None,
        }
    }

    pub fn append_delta(&mut self, delta: &str) {
        match self {
            MessageContent::Stream { deltas, .. } => {
                deltas.push(delta.to_string());
            }
            MessageContent::Text(s) => {
                let current = std::mem::take(s);
                if current.is_empty() {
                    *self = MessageContent::Stream {
                        deltas: vec![delta.to_string()],
                        final_content: None,
                    };
                } else {
                    *self = MessageContent::Stream {
                        deltas: vec![current, delta.to_string()],
                        final_content: None,
                    };
                }
            }
            _ => {}
        }
    }

    pub fn finalize(&mut self) {
        if let MessageContent::Stream {
            deltas,
            final_content,
        } = self
        {
            if final_content.is_none() {
                *final_content = Some(deltas.join(""));
            }
        }
    }
}

/// Assistant message with streaming support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub id: MessageId,
    pub turn_id: TurnId,
    pub deltas: Vec<Delta>,
    pub final_content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub created_at: DateTime<Utc>,
}

impl AssistantMessage {
    pub fn new(turn_id: TurnId) -> Self {
        Self {
            id: MessageId::new(),
            turn_id,
            deltas: Vec::new(),
            final_content: None,
            tool_calls: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_delta(&mut self, text: &str, sequence: u64) {
        self.deltas.push(Delta {
            text: text.to_string(),
            sequence,
        });
    }

    pub fn add_tool_call(&mut self, tool_call: ToolCall) {
        self.tool_calls.push(tool_call);
    }

    pub fn finalize(&mut self) {
        self.final_content = Some(
            self.deltas
                .iter()
                .map(|d| d.text.clone())
                .collect::<Vec<_>>()
                .join(""),
        );
    }

    pub fn assembled_content(&self) -> String {
        self.final_content.clone().unwrap_or_default()
    }
}

/// A delta in a streamed message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub text: String,
    pub sequence: u64,
}

/// Tool call within an assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_user() {
        let turn_id = TurnId::new();
        let msg = Message::user_message(turn_id, "Hello");
        assert!(matches!(msg.message_type, MessageType::User));
        assert_eq!(msg.content.text(), Some("Hello"));
    }

    #[test]
    fn test_message_stream_content() {
        let _turn_id = TurnId::new();
        let mut content = MessageContent::Text("".to_string());
        content.append_delta("Hello ");
        content.append_delta("World");

        if let MessageContent::Stream { deltas, .. } = content {
            assert_eq!(deltas.len(), 2);
        } else {
            panic!("Expected Stream variant");
        }
    }

    #[test]
    fn test_assistant_message_finalize() {
        let turn_id = TurnId::new();
        let mut msg = AssistantMessage::new(turn_id);
        msg.add_delta("Hello ", 0);
        msg.add_delta("World", 1);
        msg.finalize();

        assert_eq!(msg.assembled_content(), "Hello World");
    }

    #[test]
    fn test_message_id_roundtrip() {
        let id = MessageId::new();
        let s = id.to_string();
        let parsed: MessageId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_message_assistant_streaming_reconstruction() {
        let turn_id = TurnId::new();
        let mut msg = AssistantMessage::new(turn_id);
        msg.add_delta("Hel", 0);
        msg.add_delta("lo, ", 1);
        msg.add_delta("wor", 2);
        msg.add_delta("ld!", 3);
        msg.finalize();

        let content = msg.assembled_content();
        assert_eq!(content, "Hello, world!");
    }

    #[test]
    fn test_message_interrupted_stream() {
        let turn_id = TurnId::new();
        let mut msg = AssistantMessage::new(turn_id);
        msg.add_delta("Part", 0);
        msg.add_delta("ial ", 1);
        msg.add_delta("res", 2);
        // Not finalized - interrupted

        assert_eq!(msg.assembled_content(), "");
        assert!(msg.final_content.is_none());
    }

    #[test]
    fn test_message_ordering_with_sequence() {
        let turn_id = TurnId::new();
        let mut msg = AssistantMessage::new(turn_id);
        msg.add_delta("A", 0);
        msg.add_delta("B", 2); // Non-sequential
        msg.add_delta("C", 1); // Reordered

        // Deltas should be stored in order received
        assert_eq!(msg.deltas[0].sequence, 0);
        assert_eq!(msg.deltas[1].sequence, 2);
        assert_eq!(msg.deltas[2].sequence, 1);
    }

    #[test]
    fn test_message_system_message() {
        let turn_id = TurnId::new();
        let msg = Message::system_message(turn_id, "System prompt");
        assert!(matches!(msg.message_type, MessageType::System));
        assert_eq!(msg.content.text(), Some("System prompt"));
    }

    #[test]
    fn test_message_tool_message_with_metadata() {
        let turn_id = TurnId::new();
        let msg = Message::tool_message(turn_id, "tool-123", "file content");
        assert!(matches!(msg.message_type, MessageType::Tool));
        assert_eq!(
            msg.metadata.get("tool_call_id"),
            Some(&"tool-123".to_string())
        );
    }

    #[test]
    fn test_message_set_metadata() {
        let turn_id = TurnId::new();
        let mut msg = Message::user_message(turn_id, "test");
        msg.set_metadata("key1", "value1");
        msg.set_metadata("key2", "value2");
        assert_eq!(msg.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(msg.metadata.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_message_content_finalize_converts_stream() {
        let turn_id = TurnId::new();
        let mut content = MessageContent::Text("".to_string());
        content.append_delta("Hello");
        content.finalize();

        assert_eq!(content.text(), Some("Hello"));
    }

    #[test]
    fn test_message_content_text_append_delta_converts_to_stream() {
        let mut content = MessageContent::Text("Existing".to_string());
        content.append_delta(" text");

        assert!(matches!(content, MessageContent::Stream { .. }));
    }

    #[test]
    fn test_tool_call_in_assistant_message() {
        let turn_id = TurnId::new();
        let mut msg = AssistantMessage::new(turn_id);
        msg.add_tool_call(ToolCall {
            id: "call_1".to_string(),
            name: "read_file".to_string(),
            arguments: r#"{"path": "test.txt"}"#.to_string(),
        });

        assert_eq!(msg.tool_calls.len(), 1);
        assert_eq!(msg.tool_calls[0].name, "read_file");
    }

    #[test]
    fn test_message_content_stream_final_content_none_initially() {
        let content = MessageContent::Stream {
            deltas: vec!["a".to_string(), "b".to_string()],
            final_content: None,
        };
        assert_eq!(content.text(), None); // No final content yet
    }

    #[test]
    fn test_message_content_stream_with_final_content() {
        let content = MessageContent::Stream {
            deltas: vec!["hello".to_string()],
            final_content: Some("hello".to_string()),
        };
        assert_eq!(content.text(), Some("hello"));
    }

    #[test]
    fn test_message_tool_result_content() {
        let content = MessageContent::ToolResult {
            tool_call_id: "call_123".to_string(),
            tool_name: "bash".to_string(),
            result: "file contents".to_string(),
        };

        match content {
            MessageContent::ToolResult {
                tool_call_id,
                tool_name,
                result,
            } => {
                assert_eq!(tool_call_id, "call_123");
                assert_eq!(tool_name, "bash");
                assert_eq!(result, "file contents");
            }
            _ => panic!("Expected ToolResult variant"),
        }
    }

    #[test]
    fn test_message_id_parse_with_and_without_prefix() {
        let id = MessageId::new();
        let with_prefix = format!("msg:{}", id.0);
        let parsed_with: MessageId = with_prefix.parse().unwrap();
        assert_eq!(id, parsed_with);

        let without_prefix = id.0.to_string();
        let parsed_without: MessageId = without_prefix.parse().unwrap();
        assert_eq!(id, parsed_without);
    }

    #[test]
    fn test_message_id_parse_error() {
        let result: Result<MessageId, _> = "not-a-valid-id".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_message_id_display_format() {
        let id = MessageId::new();
        let s = id.to_string();
        assert!(s.starts_with("msg:"));
    }

    #[test]
    fn test_assistant_message_deltas_accumulation() {
        let turn_id = TurnId::new();
        let mut msg = AssistantMessage::new(turn_id);

        for i in 0..100 {
            msg.add_delta(&format!("chunk_{}", i), i);
        }

        assert_eq!(msg.deltas.len(), 100);
        assert_eq!(msg.deltas[99].text, "chunk_99");
    }

    #[test]
    fn test_message_created_at_is_set() {
        let turn_id = TurnId::new();
        let msg = Message::user_message(turn_id, "test");
        // created_at should be set to Utc::now()
        assert!(msg.created_at() <= chrono::Utc::now());
    }

    #[test]
    fn test_message_type_internal() {
        let turn_id = TurnId::new();
        let msg = Message::internal_message(turn_id, "internal data");
        assert!(matches!(msg.message_type(), MessageType::Internal));
        assert_eq!(msg.content().text(), Some("internal data"));
    }

    #[test]
    fn test_message_type_as_str() {
        assert_eq!(MessageType::User.as_str(), "user");
        assert_eq!(MessageType::Assistant.as_str(), "assistant");
        assert_eq!(MessageType::System.as_str(), "system");
        assert_eq!(MessageType::Tool.as_str(), "tool");
        assert_eq!(MessageType::Summary.as_str(), "summary");
        assert_eq!(MessageType::Internal.as_str(), "internal");
    }

    #[test]
    fn test_message_builder() {
        let turn_id = TurnId::new();
        let msg = Message::builder(turn_id, MessageType::User)
            .text_content("Hello from builder")
            .metadata("source", "test")
            .build();

        assert_eq!(msg.content().text(), Some("Hello from builder"));
        assert_eq!(msg.metadata().get("source"), Some(&"test".to_string()));
        assert_eq!(msg.message_type(), MessageType::User);
    }

    #[test]
    fn test_message_builder_stream() {
        let turn_id = TurnId::new();
        let msg = Message::builder(turn_id, MessageType::Assistant)
            .stream_content()
            .build();

        assert!(matches!(msg.content(), MessageContent::Stream { .. }));
    }

    #[test]
    fn test_message_summary_message() {
        let turn_id = TurnId::new();
        let msg = Message::summary_message(turn_id, "Summary of conversation");
        assert!(matches!(msg.message_type(), MessageType::Summary));
        assert_eq!(msg.content().text(), Some("Summary of conversation"));
    }

    #[test]
    fn test_message_id_accessor() {
        let turn_id = TurnId::new();
        let msg = Message::user_message(turn_id, "test");
        assert_eq!(msg.turn_id(), turn_id);
    }
}
