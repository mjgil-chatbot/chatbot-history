//! Context snapshots for ai-history
//!
//! Captures for each turn:
//! - Selected files, retrieved chunks, embeddings references
//! - Tool outputs, summaries, memory/rule references
//! - System prompt hash

use crate::ContentHash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context snapshot ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSnapshotId(uuid::Uuid);

impl ContextSnapshotId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for ContextSnapshotId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ContextSnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ctx:{}", self.0)
    }
}

/// Selected file reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedFile {
    pub path: String,
    pub hash: ContentHash,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
}

/// Retrieved chunk reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedChunk {
    pub chunk_id: String,
    pub source: String,
    pub hash: ContentHash,
    pub relevance_score: Option<f32>,
}

/// Tool output reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutputRef {
    pub tool_call_id: String,
    pub tool_name: String,
    pub output_hash: ContentHash,
}

/// Model configuration reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfigRef {
    pub model_id: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

/// Feature flags for a turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub flags: HashMap<String, bool>,
}

impl FeatureFlags {
    pub fn new() -> Self {
        Self {
            flags: HashMap::new(),
        }
    }

    pub fn with_flag(mut self, name: &str, value: bool) -> Self {
        self.flags.insert(name.to_string(), value);
        self
    }

    pub fn get(&self, name: &str) -> Option<bool> {
        self.flags.get(name).copied()
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Context snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub id: ContextSnapshotId,
    pub selected_files: Vec<SelectedFile>,
    pub retrieved_chunks: Vec<RetrievedChunk>,
    pub tool_outputs: Vec<ToolOutputRef>,
    pub summary_refs: Vec<String>,
    pub memory_refs: Vec<String>,
    pub rule_refs: Vec<String>,
    pub system_prompt_hash: Option<ContentHash>,
    pub model_config: Option<ModelConfigRef>,
    pub feature_flags: Option<FeatureFlags>,
    pub turn_id: String,
}

impl ContextSnapshot {
    pub fn new(turn_id: String) -> Self {
        Self {
            id: ContextSnapshotId::new(),
            selected_files: Vec::new(),
            retrieved_chunks: Vec::new(),
            tool_outputs: Vec::new(),
            summary_refs: Vec::new(),
            memory_refs: Vec::new(),
            rule_refs: Vec::new(),
            system_prompt_hash: None,
            model_config: None,
            feature_flags: None,
            turn_id,
        }
    }

    pub fn with_selected_file(mut self, path: String, hash: ContentHash) -> Self {
        self.selected_files.push(SelectedFile {
            path,
            hash,
            line_start: None,
            line_end: None,
        });
        self
    }

    pub fn with_retrieved_chunk(mut self, chunk: RetrievedChunk) -> Self {
        self.retrieved_chunks.push(chunk);
        self
    }

    pub fn with_tool_output(mut self, output: ToolOutputRef) -> Self {
        self.tool_outputs.push(output);
        self
    }

    pub fn with_summary_ref(mut self, summary_id: String) -> Self {
        self.summary_refs.push(summary_id);
        self
    }

    pub fn with_memory_ref(mut self, memory_id: String) -> Self {
        self.memory_refs.push(memory_id);
        self
    }

    pub fn with_rule_ref(mut self, rule_id: String) -> Self {
        self.rule_refs.push(rule_id);
        self
    }

    pub fn with_system_prompt_hash(mut self, hash: ContentHash) -> Self {
        self.system_prompt_hash = Some(hash);
        self
    }

    pub fn with_model_config(mut self, config: ModelConfigRef) -> Self {
        self.model_config = Some(config);
        self
    }

    pub fn with_feature_flags(mut self, flags: FeatureFlags) -> Self {
        self.feature_flags = Some(flags);
        self
    }

    pub fn file_count(&self) -> usize {
        self.selected_files.len()
    }

    pub fn chunk_count(&self) -> usize {
        self.retrieved_chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.selected_files.is_empty()
            && self.retrieved_chunks.is_empty()
            && self.tool_outputs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_snapshot_id_new() {
        let id = ContextSnapshotId::new();
        assert!(!format!("{}", id).is_empty());
    }

    #[test]
    fn test_context_snapshot_new() {
        let snapshot = ContextSnapshot::new("turn_123".to_string());
        assert_eq!(snapshot.turn_id, "turn_123");
        assert!(snapshot.selected_files.is_empty());
    }

    #[test]
    fn test_context_snapshot_chain() {
        let hash = ContentHash::new(b"test");
        let snapshot = ContextSnapshot::new("turn_1".to_string())
            .with_selected_file("src/main.rs".to_string(), hash)
            .with_tool_output(ToolOutputRef {
                tool_call_id: "call_1".to_string(),
                tool_name: "bash".to_string(),
                output_hash: hash,
            })
            .with_system_prompt_hash(hash);

        assert_eq!(snapshot.file_count(), 1);
        assert_eq!(snapshot.tool_outputs.len(), 1);
        assert!(snapshot.system_prompt_hash.is_some());
    }

    #[test]
    fn test_context_snapshot_is_empty() {
        let empty = ContextSnapshot::new("turn_1".to_string());
        assert!(empty.is_empty());

        let non_empty = empty.with_selected_file("test.rs".to_string(), ContentHash::new(b"x"));
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_retrieved_chunk() {
        let chunk = RetrievedChunk {
            chunk_id: "chunk_1".to_string(),
            source: "vector_db".to_string(),
            hash: ContentHash::new(b"content"),
            relevance_score: Some(0.95),
        };
        assert_eq!(chunk.chunk_id, "chunk_1");
        assert!(chunk.relevance_score.is_some());
    }

    #[test]
    fn test_model_config_ref() {
        let config = ModelConfigRef {
            model_id: "gpt-4".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
        };
        assert_eq!(config.model_id, "gpt-4");
        assert_eq!(config.temperature, Some(0.7));
    }

    #[test]
    fn test_feature_flags_new() {
        let flags = FeatureFlags::new();
        assert!(flags.get("test").is_none());
    }

    #[test]
    fn test_feature_flags_with_flag() {
        let flags = FeatureFlags::new()
            .with_flag("streaming", true)
            .with_flag("tools", false);
        assert_eq!(flags.get("streaming"), Some(true));
        assert_eq!(flags.get("tools"), Some(false));
        assert_eq!(flags.get("unknown"), None);
    }

    #[test]
    fn test_context_snapshot_with_model_config() {
        let config = ModelConfigRef {
            model_id: "gpt-4".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
        };
        let snapshot = ContextSnapshot::new("turn_1".to_string()).with_model_config(config);
        assert!(snapshot.model_config.is_some());
        assert_eq!(snapshot.model_config.unwrap().model_id, "gpt-4");
    }

    #[test]
    fn test_context_snapshot_with_feature_flags() {
        let flags = FeatureFlags::new().with_flag("streaming", true);
        let snapshot = ContextSnapshot::new("turn_1".to_string()).with_feature_flags(flags);
        assert!(snapshot.feature_flags.is_some());
    }
}
