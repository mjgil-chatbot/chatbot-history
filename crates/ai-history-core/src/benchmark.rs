//! Performance benchmarks and limits for ai-history
//!
//! Defines:
//! - Performance thresholds for snapshot, restore, diff, append, GC operations
//! - Scalability limits for branches, checkpoints, blobs, events
//! - Benchmark result types and pass/fail criteria

use serde::{Deserialize, Serialize};

/// Benchmark operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BenchmarkOperation {
    SnapshotCreate,
    Restore,
    Diff,
    EventAppend,
    EventReplay,
    Gc,
    ImportExport,
    BranchQuery,
}

/// Benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub operation: BenchmarkOperation,
    pub duration_ms: u64,
    pub items_processed: usize,
    pub throughput: f64,
    pub passed: bool,
}

impl BenchmarkResult {
    pub fn new(operation: BenchmarkOperation, duration_ms: u64, items_processed: usize) -> Self {
        let throughput = if duration_ms > 0 {
            (items_processed as f64) / (duration_ms as f64 / 1000.0)
        } else {
            0.0
        };
        Self {
            operation,
            duration_ms,
            items_processed,
            throughput,
            passed: true, // Default, check against threshold
        }
    }

    pub fn with_pass(mut self, passed: bool) -> Self {
        self.passed = passed;
        self
    }
}

/// Performance threshold for an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThreshold {
    pub operation: BenchmarkOperation,
    pub max_duration_ms: u64,
    pub min_throughput: f64,
    pub description: String,
}

impl PerformanceThreshold {
    pub fn new(
        operation: BenchmarkOperation,
        max_duration_ms: u64,
        min_throughput: f64,
        description: &'static str,
    ) -> Self {
        Self {
            operation,
            max_duration_ms,
            min_throughput,
            description: description.to_string(),
        }
    }

    pub fn check(&self, result: &BenchmarkResult) -> bool {
        result.duration_ms <= self.max_duration_ms && result.throughput >= self.min_throughput
    }
}

/// Scalability limit definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalabilityLimit {
    pub name: String,
    pub category: LimitCategory,
    pub soft_limit: usize,
    pub hard_limit: usize,
    pub description: String,
}

impl ScalabilityLimit {
    pub fn new(
        name: &'static str,
        category: LimitCategory,
        soft_limit: usize,
        hard_limit: usize,
        description: &'static str,
    ) -> Self {
        Self {
            name: name.to_string(),
            category,
            soft_limit,
            hard_limit,
            description: description.to_string(),
        }
    }

    pub fn is_within_soft_limit(&self, value: usize) -> bool {
        value <= self.soft_limit
    }

    pub fn is_within_hard_limit(&self, value: usize) -> bool {
        value <= self.hard_limit
    }
}

/// Category of scalability limit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LimitCategory {
    Branch,
    Checkpoint,
    Blob,
    Event,
    Session,
    Manifest,
    File,
}

/// Benchmark suite configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    pub name: String,
    pub thresholds: Vec<PerformanceThreshold>,
    pub limits: Vec<ScalabilityLimit>,
}

impl Default for BenchmarkSuite {
    fn default() -> Self {
        Self::production()
    }
}

impl BenchmarkSuite {
    /// Production benchmark suite
    pub fn production() -> Self {
        Self {
            name: "production".to_string(),
            thresholds: vec![
                PerformanceThreshold::new(
                    BenchmarkOperation::SnapshotCreate,
                    500,
                    200.0,
                    "Snapshot creation should complete in < 500ms for 100 files",
                ),
                PerformanceThreshold::new(
                    BenchmarkOperation::Restore,
                    1000,
                    100.0,
                    "Restore should complete in < 1s for 100 files",
                ),
                PerformanceThreshold::new(
                    BenchmarkOperation::Diff,
                    100,
                    10000.0,
                    "Diff should complete in < 100ms for 1000 lines",
                ),
                PerformanceThreshold::new(
                    BenchmarkOperation::EventAppend,
                    10,
                    100.0,
                    "Event append should complete in < 10ms",
                ),
                PerformanceThreshold::new(
                    BenchmarkOperation::Gc,
                    5000,
                    200.0,
                    "GC should complete in < 5s for 1000 blobs",
                ),
            ],
            limits: vec![
                ScalabilityLimit::new(
                    "max_branches_per_session",
                    LimitCategory::Branch,
                    50,
                    200,
                    "Maximum branches per session",
                ),
                ScalabilityLimit::new(
                    "max_checkpoints_per_branch",
                    LimitCategory::Checkpoint,
                    100,
                    500,
                    "Maximum checkpoints per branch",
                ),
                ScalabilityLimit::new(
                    "max_blob_size_mb",
                    LimitCategory::Blob,
                    10,
                    100,
                    "Maximum blob size in MB",
                ),
                ScalabilityLimit::new(
                    "max_blobs_per_session",
                    LimitCategory::Blob,
                    10000,
                    100000,
                    "Maximum blobs per session",
                ),
                ScalabilityLimit::new(
                    "max_events_per_branch",
                    LimitCategory::Event,
                    10000,
                    100000,
                    "Maximum events per branch",
                ),
                ScalabilityLimit::new(
                    "max_sessions",
                    LimitCategory::Session,
                    100,
                    1000,
                    "Maximum sessions per store",
                ),
            ],
        }
    }
}

/// Limit check result
#[derive(Debug, Clone)]
pub struct LimitCheckResult {
    pub limit_name: String,
    pub current_value: usize,
    pub soft_limit: usize,
    pub hard_limit: usize,
    pub status: LimitStatus,
}

impl LimitCheckResult {
    pub fn new(limit: &ScalabilityLimit, current_value: usize) -> Self {
        let status = if current_value > limit.hard_limit {
            LimitStatus::Exceeded
        } else if current_value > limit.soft_limit {
            LimitStatus::Warning
        } else {
            LimitStatus::Ok
        };
        Self {
            limit_name: limit.name.to_string(),
            current_value,
            soft_limit: limit.soft_limit,
            hard_limit: limit.hard_limit,
            status,
        }
    }
}

/// Status of a limit check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitStatus {
    Ok,
    Warning,
    Exceeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result_new() {
        let result = BenchmarkResult::new(BenchmarkOperation::SnapshotCreate, 100, 50);
        assert_eq!(result.duration_ms, 100);
        assert_eq!(result.items_processed, 50);
        assert!(result.throughput > 0.0);
    }

    #[test]
    fn test_performance_threshold_check_pass() {
        let threshold =
            PerformanceThreshold::new(BenchmarkOperation::SnapshotCreate, 500, 200.0, "test");
        let result = BenchmarkResult::new(BenchmarkOperation::SnapshotCreate, 100, 100);
        assert!(threshold.check(&result));
    }

    #[test]
    fn test_performance_threshold_check_fail_duration() {
        let threshold =
            PerformanceThreshold::new(BenchmarkOperation::SnapshotCreate, 500, 200.0, "test");
        let result = BenchmarkResult::new(BenchmarkOperation::SnapshotCreate, 600, 100);
        assert!(!threshold.check(&result));
    }

    #[test]
    fn test_scalability_limit_within_soft() {
        let limit = ScalabilityLimit::new("test", LimitCategory::Branch, 50, 100, "test");
        assert!(limit.is_within_soft_limit(25));
        assert!(limit.is_within_soft_limit(50));
        assert!(!limit.is_within_soft_limit(51));
    }

    #[test]
    fn test_scalability_limit_within_hard() {
        let limit = ScalabilityLimit::new("test", LimitCategory::Branch, 50, 100, "test");
        assert!(limit.is_within_hard_limit(100));
        assert!(!limit.is_within_hard_limit(101));
    }

    #[test]
    fn test_limit_check_result_ok() {
        let limit = ScalabilityLimit::new("test", LimitCategory::Branch, 50, 100, "test");
        let result = LimitCheckResult::new(&limit, 25);
        assert_eq!(result.status, LimitStatus::Ok);
    }

    #[test]
    fn test_limit_check_result_warning() {
        let limit = ScalabilityLimit::new("test", LimitCategory::Branch, 50, 100, "test");
        let result = LimitCheckResult::new(&limit, 75);
        assert_eq!(result.status, LimitStatus::Warning);
    }

    #[test]
    fn test_limit_check_result_exceeded() {
        let limit = ScalabilityLimit::new("test", LimitCategory::Branch, 50, 100, "test");
        let result = LimitCheckResult::new(&limit, 150);
        assert_eq!(result.status, LimitStatus::Exceeded);
    }

    #[test]
    fn test_benchmark_suite_production_has_thresholds() {
        let suite = BenchmarkSuite::production();
        assert!(!suite.thresholds.is_empty());
        assert!(!suite.limits.is_empty());
    }

    #[test]
    fn test_benchmark_suite_default_is_production() {
        let suite = BenchmarkSuite::default();
        assert_eq!(suite.name, "production");
    }
}
