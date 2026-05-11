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
    }}
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
    }}
impl MessageBuilder {
    pub fn content(mut self, content: MessageContent) -> Self {
        self.content = Some(content);
        self
    }}
impl MessageBuilder {
    pub fn text_content(mut self, text: &str) -> Self {
        self.content = Some(MessageContent::Text(text.to_string()));
        self
    }}
impl MessageBuilder {
    pub fn stream_content(mut self) -> Self {
        self.content = Some(MessageContent::Stream {
            deltas: Vec::new(),
            final_content: None,
        });
        self
    }}
impl MessageBuilder {
    pub fn metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }}
impl MessageBuilder {
    pub fn build(self) -> Message {
        Message {
            id: MessageId::new(),
            turn_id: self.turn_id,
            message_type: self.message_type,
            content: self.content.unwrap_or(MessageContent::Text(String::new())),
            metadata: self.metadata,
            created_at: Utc::now(),
        }
    }}
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
    }}
impl Message {
    pub fn turn_id(&self) -> TurnId {
        self.turn_id
    }}
impl Message {
    pub fn message_type(&self) -> MessageType {
        self.message_type
    }}
impl Message {
    pub fn content(&self) -> &MessageContent {
        &self.content
    }}
impl Message {
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }}
impl Message {
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }}
impl Message {
    pub fn new(turn_id: TurnId, message_type: MessageType, content: MessageContent) -> Self {
        Self {
            id: MessageId::new(),
            turn_id,
            message_type,
            content,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }}
impl Message {
    pub fn builder(turn_id: TurnId, message_type: MessageType) -> MessageBuilder {
        MessageBuilder::new(turn_id, message_type)
    }}
impl Message {
    pub fn user_message(turn_id: TurnId, content: &str) -> Self {
        Self::new(
            turn_id,
            MessageType::User,
            MessageContent::Text(content.to_string()),
        )
    }}
impl Message {
    pub fn assistant_message(turn_id: TurnId, content: &str) -> Self {
        Self::new(
            turn_id,
            MessageType::Assistant,
            MessageContent::Text(content.to_string()),
        )
    }}
impl Message {
    pub fn system_message(turn_id: TurnId, content: &str) -> Self {
        Self::new(
            turn_id,
            MessageType::System,
            MessageContent::Text(content.to_string()),
        )
    }}
impl Message {
    pub fn internal_message(turn_id: TurnId, content: &str) -> Self {
        Self::new(
            turn_id,
            MessageType::Internal,
            MessageContent::Text(content.to_string()),
        )
    }}
impl Message {
    pub fn tool_message(turn_id: TurnId, tool_call_id: &str, content: &str) -> Self {
        let mut msg = Self::new(
            turn_id,
            MessageType::Tool,
            MessageContent::Text(content.to_string()),
        );
        msg.metadata
            .insert("tool_call_id".to_string(), tool_call_id.to_string());
        msg
    }}
impl Message {
    pub fn summary_message(turn_id: TurnId, content: &str) -> Self {
        Self::new(
            turn_id,
            MessageType::Summary,
            MessageContent::Text(content.to_string()),
        )
    }}
impl Message {
    pub fn set_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }}
/// Message ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(uuid::Uuid);
impl MessageId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }}
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
    }}
impl MessageContent {
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
    }}
impl MessageContent {
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
    }}
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
    }}
impl AssistantMessage {
    pub fn add_delta(&mut self, text: &str, sequence: u64) {
        self.deltas.push(Delta {
            text: text.to_string(),
            sequence,
        });
    }}
impl AssistantMessage {
    pub fn add_tool_call(&mut self, tool_call: ToolCall) {
        self.tool_calls.push(tool_call);
    }}
impl AssistantMessage {
    pub fn finalize(&mut self) {
        self.final_content = Some(
            self.deltas
                .iter()
                .map(|d| d.text.clone())
                .collect::<Vec<_>>()
                .join(""),
        );
    }}
impl AssistantMessage {
    pub fn assembled_content(&self) -> String {
        self.final_content.clone().unwrap_or_default()
    }}
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
