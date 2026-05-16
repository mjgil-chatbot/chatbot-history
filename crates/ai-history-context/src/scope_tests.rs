//! Context scope verification tests
//!
//! Task 12: Verify context owns selected files, retrieved chunks, tool outputs,
//! summaries, memories, rules, model config, system prompt hash, feature flags,
//! and reproducibility references.

#[cfg(test)]
mod tests {
    use crate::{
        validate_turn_reproducibility, ContextSnapshot, LineRange, MemoryRef, MemoryType,
        ModelConfig, ReproducibilityInfo, RetrievedChunkRef, RuleRef, SelectedFileRef, SummaryRef,
        SummaryType, ToolOutputRef,
    };
    use ai_history_core::{ContentHash, Timestamp, TurnId};

    #[test]
    fn test_context_snapshot_new() {
        let turn_id = TurnId::new();
        let snapshot = ContextSnapshot::new(turn_id);
        assert_eq!(snapshot.turn_id, turn_id);
        assert!(snapshot.selected_files.is_empty());
        assert!(snapshot.retrieved_chunks.is_empty());
        assert!(snapshot.tool_outputs.is_empty());
        assert!(snapshot.summary_refs.is_empty());
        assert!(snapshot.memory_refs.is_empty());
        assert!(snapshot.rule_refs.is_empty());
    }

    #[test]
    fn test_context_snapshot_add_selected_file() {
        let mut snapshot = ContextSnapshot::new(TurnId::new());
        let file_ref = SelectedFileRef {
            path: "/workspace/src/main.rs".to_string(),
            blob_hash: ContentHash::new(b"content"),
            line_range: Some(LineRange { start: 1, end: 50 }),
            relevance_score: Some(0.95),
        };
        snapshot.add_selected_file(file_ref);
        assert_eq!(snapshot.selected_files.len(), 1);
    }

    #[test]
    fn test_context_snapshot_add_retrieved_chunk() {
        let mut snapshot = ContextSnapshot::new(TurnId::new());
        let chunk = RetrievedChunkRef {
            chunk_id: "chunk_1".to_string(),
            source: "vector_db".to_string(),
            blob_hash: ContentHash::new(b"chunk_content"),
            relevance_score: 0.8,
        };
        snapshot.add_retrieved_chunk(chunk);
        assert_eq!(snapshot.retrieved_chunks.len(), 1);
    }

    #[test]
    fn test_context_snapshot_add_tool_output() {
        let mut snapshot = ContextSnapshot::new(TurnId::new());
        let output = ToolOutputRef {
            tool_call_id: "call_123".to_string(),
            tool_name: "bash".to_string(),
            output_hash: ContentHash::new(b"tool output"),
            timestamp: Timestamp::now(),
        };
        snapshot.add_tool_output(output);
        assert_eq!(snapshot.tool_outputs.len(), 1);
    }

    #[test]
    fn test_context_snapshot_total_references() {
        let mut snapshot = ContextSnapshot::new(TurnId::new());
        assert_eq!(snapshot.total_references(), 0);

        snapshot.add_selected_file(SelectedFileRef {
            path: "file1.rs".to_string(),
            blob_hash: ContentHash::new(b"a"),
            line_range: None,
            relevance_score: None,
        });
        snapshot.add_retrieved_chunk(RetrievedChunkRef {
            chunk_id: "c1".to_string(),
            source: "src".to_string(),
            blob_hash: ContentHash::new(b"b"),
            relevance_score: 0.5,
        });
        snapshot.add_tool_output(ToolOutputRef {
            tool_call_id: "c2".to_string(),
            tool_name: "test".to_string(),
            output_hash: ContentHash::new(b"c"),
            timestamp: Timestamp::now(),
        });

        // Memory and rule refs also count
        snapshot.memory_refs.push(MemoryRef {
            memory_id: "mem1".to_string(),
            memory_hash: ContentHash::new(b"d"),
            memory_type: MemoryType::LongTerm,
        });
        snapshot.rule_refs.push(RuleRef {
            rule_set_id: "rules_1".to_string(),
            rules_hash: ContentHash::new(b"e"),
        });

        assert_eq!(snapshot.total_references(), 5);
    }

    #[test]
    fn test_selected_file_ref_fields() {
        let file_ref = SelectedFileRef {
            path: "/project/src/lib.rs".to_string(),
            blob_hash: ContentHash::new(b"file_content"),
            line_range: Some(LineRange {
                start: 10,
                end: 100,
            }),
            relevance_score: Some(0.75),
        };
        assert_eq!(file_ref.path, "/project/src/lib.rs");
        assert!(file_ref.line_range.is_some());
        assert!(file_ref.relevance_score.is_some());
    }

    #[test]
    fn test_line_range() {
        let range = LineRange { start: 1, end: 50 };
        assert_eq!(range.start, 1);
        assert_eq!(range.end, 50);
    }

    #[test]
    fn test_retrieved_chunk_ref() {
        let chunk = RetrievedChunkRef {
            chunk_id: "vec_123".to_string(),
            source: "embedding_db".to_string(),
            blob_hash: ContentHash::new(b"vector_content"),
            relevance_score: 0.92,
        };
        assert!(chunk.relevance_score > 0.9);
    }

    #[test]
    fn test_tool_output_ref() {
        let output = ToolOutputRef {
            tool_call_id: "tool_call_456".to_string(),
            tool_name: "read_file".to_string(),
            output_hash: ContentHash::new(b"file contents"),
            timestamp: Timestamp::now(),
        };
        assert!(!output.tool_call_id.is_empty());
        assert!(!output.tool_name.is_empty());
    }

    #[test]
    fn test_summary_ref_types() {
        let auto_ref = SummaryRef {
            summary_id: "sum_1".to_string(),
            summary_hash: ContentHash::new(b"auto_summary"),
            summary_type: SummaryType::AutoGenerated,
        };
        assert!(matches!(auto_ref.summary_type, SummaryType::AutoGenerated));

        let user_ref = SummaryRef {
            summary_id: "sum_2".to_string(),
            summary_hash: ContentHash::new(b"user_summary"),
            summary_type: SummaryType::UserProvided,
        };
        assert!(matches!(user_ref.summary_type, SummaryType::UserProvided));
    }

    #[test]
    fn test_memory_ref_types() {
        let long_term = MemoryRef {
            memory_id: "mem_1".to_string(),
            memory_hash: ContentHash::new(b"long_mem"),
            memory_type: MemoryType::LongTerm,
        };
        assert!(matches!(long_term.memory_type, MemoryType::LongTerm));

        let short_term = MemoryRef {
            memory_id: "mem_2".to_string(),
            memory_hash: ContentHash::new(b"short_mem"),
            memory_type: MemoryType::ShortTerm,
        };
        assert!(matches!(short_term.memory_type, MemoryType::ShortTerm));
    }

    #[test]
    fn test_model_config_default() {
        let config = ModelConfig::default();
        // Model config has default values
        assert!(!config.provider.is_empty() || !config.model.is_empty());
        // Temperature is optional
        assert!(config.temperature.is_none() || config.temperature.unwrap() >= 0.0);
    }

    #[test]
    fn test_context_snapshot_serialization() {
        let mut snapshot = ContextSnapshot::new(TurnId::new());
        snapshot.add_selected_file(SelectedFileRef {
            path: "test.rs".to_string(),
            blob_hash: ContentHash::new(b"content"),
            line_range: None,
            relevance_score: None,
        });

        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: ContextSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.selected_files.len(), 1);
    }

    #[test]
    fn test_reproducibility_info_new() {
        let ctx = ContextSnapshot::new(TurnId::new());
        let info = ReproducibilityInfo::new(ctx);
        assert!(info.can_reproduce);
        assert!(info.missing_dependencies.is_empty());
    }

    #[test]
    fn test_reproducibility_info_add_missing() {
        let ctx = ContextSnapshot::new(TurnId::new());
        let mut info = ReproducibilityInfo::new(ctx);
        info.add_missing_dependency("blob:abc123".to_string());
        assert!(!info.can_reproduce);
        assert_eq!(info.missing_dependencies.len(), 1);
    }

    #[test]
    fn test_rule_ref() {
        let rule = RuleRef {
            rule_set_id: "security_rules".to_string(),
            rules_hash: ContentHash::new(b"security_rules_content"),
        };
        assert!(!rule.rule_set_id.is_empty());
    }

    #[test]
    fn test_context_snapshot_system_prompt_hash() {
        let snapshot = ContextSnapshot::new(TurnId::new());
        // system_prompt_hash should be default (empty hash)
        assert_eq!(snapshot.system_prompt_hash, ContentHash::default());
    }

    #[test]
    fn test_context_snapshot_model_config() {
        let snapshot = ContextSnapshot::new(TurnId::new());
        // Should have default model config with defaults from impl
        assert!(
            !snapshot.model_config.provider.is_empty() || !snapshot.model_config.model.is_empty()
        );
    }

    #[test]
    fn test_content_hash_in_context() {
        let hash = ContentHash::new(b"test content for hash");
        let file_ref = SelectedFileRef {
            path: "test.txt".to_string(),
            blob_hash: hash,
            line_range: None,
            relevance_score: None,
        };
        assert_eq!(file_ref.blob_hash.to_hex().len(), 64);
    }

    #[test]
    fn test_timestamp_in_context() {
        let ts = Timestamp::now();
        let output = ToolOutputRef {
            tool_call_id: "call_1".to_string(),
            tool_name: "ls".to_string(),
            output_hash: ContentHash::new(b"result"),
            timestamp: ts,
        };
        // Timestamp should be stored
        assert!(output.timestamp.to_string().len() > 0);
    }

    #[test]
    fn test_validate_turn_reproducibility() {
        let ctx = ContextSnapshot::new(TurnId::new());
        let result = validate_turn_reproducibility(&ctx);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(info.can_reproduce);
    }
}
