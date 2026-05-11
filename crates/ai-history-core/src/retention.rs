//! Retention and compaction policies for ai-history
//!
//! Defines:
//! - Retention policy for old checkpoints
//! - Blob garbage collection
//! - Event compaction and summary creation
//! - Protected named checkpoints

use crate::CheckpointId;
use serde::{Deserialize, Serialize};

/// Retention period
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetentionPeriod {
    /// Keep forever
    Forever,
    /// Keep for N days
    Days(u32),
    /// Keep N checkpoints
    CheckpointCount(u32),
    /// Keep until explicitly deleted
    UntilDeleted,
}

impl Default for RetentionPeriod {
    fn default() -> Self {
        Self::Days(30)
    }
}

/// Retention rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionRule {
    pub name: String,
    pub target: RetentionTarget,
    pub period: RetentionPeriod,
    pub is_protected: bool,
}

impl RetentionRule {
    pub fn new(name: String, target: RetentionTarget, period: RetentionPeriod) -> Self {
        Self {
            name,
            target,
            period,
            is_protected: false,
        }
    }

    pub fn protected(mut self) -> Self {
        self.is_protected = true;
        self
    }
}

/// What the rule applies to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetentionTarget {
    /// All checkpoints
    AllCheckpoints,
    /// Named checkpoints only
    NamedCheckpoints,
    /// Safety checkpoints
    SafetyCheckpoints,
    /// Auto checkpoints
    AutoCheckpoints,
    /// Blobs not referenced by any checkpoint
    OrphanedBlobs,
    /// Events older than threshold
    OldEvents,
}

impl RetentionRule {
    /// Check if a checkpoint with given properties should be retained
    pub fn should_retain(
        &self,
        _checkpoint_id: CheckpointId,
        age_days: u32,
        checkpoint_count: u32,
        is_named: bool,
        is_protected: bool,
    ) -> bool {
        // Protected items are never deleted
        if is_protected || self.is_protected {
            return true;
        }

        match &self.target {
            RetentionTarget::AllCheckpoints => {}
            RetentionTarget::NamedCheckpoints => {
                // Named checkpoints targeted by this rule are always retained
                if is_named {
                    return true;
                }
                // Non-named checkpoints are subject to period
            }
            RetentionTarget::SafetyCheckpoints => return true,
            RetentionTarget::AutoCheckpoints => {}
            RetentionTarget::OrphanedBlobs => return true,
            RetentionTarget::OldEvents => {}
        }

        match self.period {
            RetentionPeriod::Forever => true,
            RetentionPeriod::UntilDeleted => true,
            RetentionPeriod::Days(n) => age_days <= n,
            RetentionPeriod::CheckpointCount(n) => checkpoint_count <= n,
        }
    }
}

/// Compaction strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactionStrategy {
    /// No compaction
    None,
    /// Summarize events into a summary checkpoint
    Summarize,
    /// Delete old events after summarization
    DeleteOld,
}

impl Default for CompactionStrategy {
    fn default() -> Self {
        Self::Summarize
    }
}

/// Compaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    pub strategy: CompactionStrategy,
    pub event_count_threshold: u64,
    pub age_days_threshold: u32,
    pub preserve_named_checkpoints: bool,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            strategy: CompactionStrategy::default(),
            event_count_threshold: 10000,
            age_days_threshold: 90,
            preserve_named_checkpoints: true,
        }
    }
}

/// Garbage collection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcResult {
    pub blobs_checked: u64,
    pub blobs_deleted: u64,
    pub checkpoints_checked: u64,
    pub checkpoints_deleted: u64,
    pub errors: Vec<String>,
}

impl GcResult {
    pub fn new() -> Self {
        Self {
            blobs_checked: 0,
            blobs_deleted: 0,
            checkpoints_checked: 0,
            checkpoints_deleted: 0,
            errors: Vec::new(),
        }
    }

    pub fn with_counts(
        mut self,
        blobs_checked: u64,
        blobs_deleted: u64,
        checkpoints_checked: u64,
        checkpoints_deleted: u64,
    ) -> Self {
        self.blobs_checked = blobs_checked;
        self.blobs_deleted = blobs_deleted;
        self.checkpoints_checked = checkpoints_checked;
        self.checkpoints_deleted = checkpoints_deleted;
        self
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Default for GcResult {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retention_period_default() {
        assert_eq!(RetentionPeriod::default(), RetentionPeriod::Days(30));
    }

    #[test]
    fn test_retention_rule_forever() {
        let rule = RetentionRule::new(
            "keep_forever".to_string(),
            RetentionTarget::AllCheckpoints,
            RetentionPeriod::Forever,
        );

        assert!(rule.should_retain(CheckpointId::new(), 9999, 9999, false, false));
    }

    #[test]
    fn test_retention_rule_days() {
        let rule = RetentionRule::new(
            "keep_30_days".to_string(),
            RetentionTarget::AllCheckpoints,
            RetentionPeriod::Days(30),
        );

        assert!(rule.should_retain(CheckpointId::new(), 10, 0, false, false));
        assert!(!rule.should_retain(CheckpointId::new(), 60, 0, false, false));
    }

    #[test]
    fn test_retention_rule_named_protected() {
        let rule = RetentionRule::new(
            "named_protected".to_string(),
            RetentionTarget::NamedCheckpoints,
            RetentionPeriod::Days(7),
        );

        // Named checkpoints are always protected regardless of age
        assert!(rule.should_retain(CheckpointId::new(), 100, 0, true, false));
        // Non-named are subject to period
        assert!(rule.should_retain(CheckpointId::new(), 5, 0, false, false));
        assert!(!rule.should_retain(CheckpointId::new(), 10, 0, false, false));
    }

    #[test]
    fn test_retention_rule_protected_flag() {
        let rule = RetentionRule::new(
            "protected".to_string(),
            RetentionTarget::AllCheckpoints,
            RetentionPeriod::Days(1),
        )
        .protected();

        // Even old, non-named checkpoints are protected
        assert!(rule.should_retain(CheckpointId::new(), 999, 0, false, false));
    }

    #[test]
    fn test_compaction_config_default() {
        let config = CompactionConfig::default();
        assert_eq!(config.strategy, CompactionStrategy::Summarize);
        assert_eq!(config.event_count_threshold, 10000);
        assert_eq!(config.age_days_threshold, 90);
    }

    #[test]
    fn test_gc_result_new() {
        let result = GcResult::new();
        assert_eq!(result.blobs_checked, 0);
        assert_eq!(result.blobs_deleted, 0);
        assert!(result.is_success());
    }

    #[test]
    fn test_gc_result_with_counts() {
        let result = GcResult::new().with_counts(100, 25, 50, 5);

        assert_eq!(result.blobs_checked, 100);
        assert_eq!(result.blobs_deleted, 25);
        assert_eq!(result.checkpoints_checked, 50);
        assert_eq!(result.checkpoints_deleted, 5);
    }

    #[test]
    fn test_gc_result_with_error() {
        let mut result = GcResult::new();
        result.add_error("Failed to delete blob".to_string());

        assert!(!result.is_success());
        assert_eq!(result.errors.len(), 1);
    }
}
