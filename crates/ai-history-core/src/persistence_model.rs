//! Persistence model for the AI history system
//!
//! Defines:
//! - Append-only event sourcing for chat/tool history
//! - Content-addressed snapshots for workspace state
//! - Branch DAGs for forks
//! - Materialized views for UI reads

use serde::{Deserialize, Serialize};

/// Persistence strategy type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistenceStrategy {
    /// Append-only journal for events
    EventSourcing,
    /// Content-addressed snapshots
    Snapshot,
    /// Directed Acyclic Graph for branches
    BranchDag,
    /// Pre-computed read model
    MaterializedView,
}

/// Data category for persistence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataCategory {
    /// Chat messages and tool calls (append-only journal)
    ChatHistory,
    /// Workspace file snapshots (content-addressed)
    WorkspaceState,
    /// Branch structure and relationships (DAG)
    BranchStructure,
    /// Pre-computed UI views (materialized)
    UiViews,
}

/// Persistence requirements for a data type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceRequirement {
    pub data_type: &'static str,
    pub category: DataCategory,
    pub strategy: PersistenceStrategy,
    pub consistency: ConsistencyLevel,
    pub retention: RetentionPolicy,
    pub deduplication: bool,
}

/// Consistency level for persistence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    /// Strong consistency - always reads latest
    Strong,
    /// Eventual consistency - may read slightly stale
    Eventual,
    /// Causal consistency - respects happens-before
    Causal,
}

impl Default for ConsistencyLevel {
    fn default() -> Self {
        Self::Strong
    }
}

/// Retention policy for data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetentionPolicy {
    /// Keep forever
    Permanent,
    /// Keep until explicitly deleted
    UntilDeleted,
    /// Keep for N days
    Days(u32),
    /// Keep N checkpoints
    CheckpointCount(u32),
}

impl RetentionPolicy {
    /// Check if data should be retained
    pub fn should_retain(&self, _age_days: u32, _checkpoint_count: u32) -> bool {
        match self {
            RetentionPolicy::Permanent => true,
            RetentionPolicy::UntilDeleted => true,
            RetentionPolicy::Days(n) => _age_days <= *n,
            RetentionPolicy::CheckpointCount(n) => _checkpoint_count <= *n,
        }
    }
}

/// Event sourcing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSourcingConfig {
    pub enabled: bool,
    pub journal_path: &'static str,
    pub sequence_per_session: bool,
    pub checksum_algorithm: &'static str,
    pub causation_tracking: bool,
    pub correlation_tracking: bool,
}

impl Default for EventSourcingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            journal_path: "journal/events",
            sequence_per_session: true,
            checksum_algorithm: "SHA-256",
            causation_tracking: true,
            correlation_tracking: true,
        }
    }
}

/// Content-addressed storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAddressingConfig {
    pub enabled: bool,
    pub hash_algorithm: &'static str,
    pub deduplication_enabled: bool,
    pub compression_enabled: bool,
    pub blob_store_path: &'static str,
}

impl Default for ContentAddressingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hash_algorithm: "SHA-256",
            deduplication_enabled: true,
            compression_enabled: true,
            blob_store_path: "blobs",
        }
    }
}

/// Branch DAG configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchDagConfig {
    pub enabled: bool,
    pub preserve_abandoned_futures: bool,
    pub max_branch_depth: Option<u32>,
    pub branch_name_uniqueness: bool,
}

impl Default for BranchDagConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            preserve_abandoned_futures: true,
            max_branch_depth: None,
            branch_name_uniqueness: true,
        }
    }
}

/// Materialized view configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedViewConfig {
    pub enabled: bool,
    pub update_on_events: bool,
    pub views: Vec<String>,
}

impl Default for MaterializedViewConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            update_on_events: true,
            views: vec![
                "chat_transcript".to_string(),
                "branch_timeline".to_string(),
                "checkpoint_list".to_string(),
                "file_change_summary".to_string(),
                "restore_history".to_string(),
            ],
        }
    }
}

/// Complete persistence model
#[derive(Debug, Clone, Default)]
pub struct PersistenceModel {
    pub event_sourcing: EventSourcingConfig,
    pub content_addressing: ContentAddressingConfig,
    pub branch_dag: BranchDagConfig,
    pub materialized_views: MaterializedViewConfig,
}

impl PersistenceModel {
    /// Create with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom configuration
    pub fn with_config(
        event_sourcing: EventSourcingConfig,
        content_addressing: ContentAddressingConfig,
        branch_dag: BranchDagConfig,
        materialized_views: MaterializedViewConfig,
    ) -> Self {
        Self {
            event_sourcing,
            content_addressing,
            branch_dag,
            materialized_views,
        }
    }

    /// Get persistence requirements for all data types
    pub fn requirements() -> Vec<PersistenceRequirement> {
        vec![
            // Chat history - event sourcing
            PersistenceRequirement {
                data_type: "UserMessage",
                category: DataCategory::ChatHistory,
                strategy: PersistenceStrategy::EventSourcing,
                consistency: ConsistencyLevel::Strong,
                retention: RetentionPolicy::Permanent,
                deduplication: false,
            },
            PersistenceRequirement {
                data_type: "AssistantMessage",
                category: DataCategory::ChatHistory,
                strategy: PersistenceStrategy::EventSourcing,
                consistency: ConsistencyLevel::Strong,
                retention: RetentionPolicy::Permanent,
                deduplication: false,
            },
            PersistenceRequirement {
                data_type: "ToolCall",
                category: DataCategory::ChatHistory,
                strategy: PersistenceStrategy::EventSourcing,
                consistency: ConsistencyLevel::Strong,
                retention: RetentionPolicy::Permanent,
                deduplication: false,
            },
            // Workspace state - content addressing
            PersistenceRequirement {
                data_type: "FileSnapshot",
                category: DataCategory::WorkspaceState,
                strategy: PersistenceStrategy::Snapshot,
                consistency: ConsistencyLevel::Strong,
                retention: RetentionPolicy::UntilDeleted,
                deduplication: true,
            },
            PersistenceRequirement {
                data_type: "Manifest",
                category: DataCategory::WorkspaceState,
                strategy: PersistenceStrategy::Snapshot,
                consistency: ConsistencyLevel::Strong,
                retention: RetentionPolicy::UntilDeleted,
                deduplication: true,
            },
            PersistenceRequirement {
                data_type: "Blob",
                category: DataCategory::WorkspaceState,
                strategy: PersistenceStrategy::Snapshot,
                consistency: ConsistencyLevel::Strong,
                retention: RetentionPolicy::UntilDeleted,
                deduplication: true,
            },
            // Branch structure - DAG
            PersistenceRequirement {
                data_type: "Branch",
                category: DataCategory::BranchStructure,
                strategy: PersistenceStrategy::BranchDag,
                consistency: ConsistencyLevel::Strong,
                retention: RetentionPolicy::Permanent,
                deduplication: false,
            },
            PersistenceRequirement {
                data_type: "Turn",
                category: DataCategory::BranchStructure,
                strategy: PersistenceStrategy::BranchDag,
                consistency: ConsistencyLevel::Strong,
                retention: RetentionPolicy::Permanent,
                deduplication: false,
            },
            // UI views - materialized
            PersistenceRequirement {
                data_type: "ChatTranscript",
                category: DataCategory::UiViews,
                strategy: PersistenceStrategy::MaterializedView,
                consistency: ConsistencyLevel::Eventual,
                retention: RetentionPolicy::UntilDeleted,
                deduplication: false,
            },
            PersistenceRequirement {
                data_type: "CheckpointList",
                category: DataCategory::UiViews,
                strategy: PersistenceStrategy::MaterializedView,
                consistency: ConsistencyLevel::Eventual,
                retention: RetentionPolicy::UntilDeleted,
                deduplication: false,
            },
        ]
    }

    /// Check if a data type should use event sourcing
    pub fn is_event_sourced(data_type: &str) -> bool {
        matches!(
            data_type,
            "UserMessage" | "AssistantMessage" | "ToolCall" | "Event"
        )
    }

    /// Check if a data type uses content addressing
    pub fn is_content_addressed(data_type: &str) -> bool {
        matches!(
            data_type,
            "FileSnapshot" | "Manifest" | "Blob" | "Checkpoint"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistence_model_new() {
        let model = PersistenceModel::new();
        assert!(model.event_sourcing.enabled);
        assert!(model.content_addressing.enabled);
        assert!(model.branch_dag.enabled);
    }

    #[test]
    fn test_retention_policy_permanent() {
        let policy = RetentionPolicy::Permanent;
        assert!(policy.should_retain(9999, 9999));
    }

    #[test]
    fn test_retention_policy_days() {
        let policy = RetentionPolicy::Days(30);
        assert!(policy.should_retain(10, 0));
        assert!(!policy.should_retain(60, 0));
    }

    #[test]
    fn test_retention_policy_checkpoint_count() {
        let policy = RetentionPolicy::CheckpointCount(5);
        assert!(policy.should_retain(0, 3));
        assert!(!policy.should_retain(0, 10));
    }

    #[test]
    fn test_persistence_model_requirements() {
        let reqs = PersistenceModel::requirements();
        assert!(!reqs.is_empty());
        assert!(reqs.iter().any(|r| r.data_type == "UserMessage"));
        assert!(reqs.iter().any(|r| r.data_type == "FileSnapshot"));
        assert!(reqs.iter().any(|r| r.data_type == "Branch"));
    }

    #[test]
    fn test_persistence_model_is_event_sourced() {
        assert!(PersistenceModel::is_event_sourced("UserMessage"));
        assert!(PersistenceModel::is_event_sourced("AssistantMessage"));
        assert!(PersistenceModel::is_event_sourced("ToolCall"));
        assert!(!PersistenceModel::is_event_sourced("FileSnapshot"));
    }

    #[test]
    fn test_persistence_model_is_content_addressed() {
        assert!(PersistenceModel::is_content_addressed("FileSnapshot"));
        assert!(PersistenceModel::is_content_addressed("Manifest"));
        assert!(PersistenceModel::is_content_addressed("Blob"));
        assert!(!PersistenceModel::is_content_addressed("UserMessage"));
    }

    #[test]
    fn test_consistency_level_default() {
        assert_eq!(ConsistencyLevel::default(), ConsistencyLevel::Strong);
    }

    #[test]
    fn test_consistency_level_serialization() {
        let level = ConsistencyLevel::Causal;
        let json = serde_json::to_string(&level).unwrap();
        let parsed: ConsistencyLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ConsistencyLevel::Causal);
    }

    #[test]
    fn test_data_category_serialization() {
        let cat = DataCategory::ChatHistory;
        let json = serde_json::to_string(&cat).unwrap();
        let parsed: DataCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, DataCategory::ChatHistory);
    }

    #[test]
    fn test_event_sourcing_config_default() {
        let config = EventSourcingConfig::default();
        assert!(config.enabled);
        assert!(config.causation_tracking);
        assert!(config.correlation_tracking);
    }

    #[test]
    fn test_content_addressing_config_default() {
        let config = ContentAddressingConfig::default();
        assert!(config.enabled);
        assert!(config.deduplication_enabled);
        assert_eq!(config.hash_algorithm, "SHA-256");
    }

    #[test]
    fn test_branch_dag_config_default() {
        let config = BranchDagConfig::default();
        assert!(config.enabled);
        assert!(config.preserve_abandoned_futures);
    }

    #[test]
    fn test_materialized_view_config_default() {
        let config = MaterializedViewConfig::default();
        assert!(config.enabled);
        assert!(config.update_on_events);
        assert!(!config.views.is_empty());
    }

    #[test]
    fn test_persistence_requirement_serialization() {
        let req = PersistenceRequirement {
            data_type: "Test",
            category: DataCategory::ChatHistory,
            strategy: PersistenceStrategy::EventSourcing,
            consistency: ConsistencyLevel::Strong,
            retention: RetentionPolicy::Permanent,
            deduplication: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Test"));
    }

    #[test]
    fn test_persistence_strategy_serialization() {
        let strategy = PersistenceStrategy::EventSourcing;
        let json = serde_json::to_string(&strategy).unwrap();
        let parsed: PersistenceStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PersistenceStrategy::EventSourcing);
    }

    #[test]
    fn test_retention_policy_serialization() {
        let policy = RetentionPolicy::Days(7);
        let json = serde_json::to_string(&policy).unwrap();
        let parsed: RetentionPolicy = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, RetentionPolicy::Days(7)));
    }

    #[test]
    fn test_all_requirements_have_strategies() {
        for req in PersistenceModel::requirements() {
            assert!(
                !req.data_type.is_empty(),
                "Requirement should have data_type"
            );
        }
    }
}
