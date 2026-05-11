//! Observability types for structured logging and metrics
//!
//! Provides trace IDs, operation durations, restore outcomes, conflict counts,
//! blob-store failures, and replay diagnostics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Trace ID for correlating operations across the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(String);

impl TraceId {
    /// Create a new trace ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Create from existing string
    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    /// Get the trace ID as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TraceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "trace:{}", self.0)
    }
}

/// Operation type for metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationType {
    SnapshotCreate,
    RestoreExecute,
    EventAppend,
    BlobRead,
    BlobWrite,
    BranchCreate,
    BranchFork,
    CheckpointCreate,
    ConflictDetect,
    ConflictResolve,
    ReplayEvent,
    GcRun,
    ImportExport,
}

/// Structured log entry for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub trace_id: TraceId,
    pub operation: OperationType,
    pub duration_ms: u64,
    pub timestamp: String,
    pub outcome: OperationOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(
        trace_id: TraceId,
        operation: OperationType,
        duration_ms: u64,
        outcome: OperationOutcome,
    ) -> Self {
        Self {
            trace_id,
            operation,
            duration_ms,
            timestamp: chrono::Utc::now().to_rfc3339(),
            outcome,
            error_message: None,
            session_id: None,
            branch_id: None,
        }
    }

    /// Create a log entry with error
    pub fn with_error(mut self, error: String) -> Self {
        self.error_message = Some(error);
        self
    }

    /// Add session ID
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Add branch ID
    pub fn with_branch_id(mut self, branch_id: String) -> Self {
        self.branch_id = Some(branch_id);
        self
    }
}

/// Operation outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationOutcome {
    Success,
    Failure,
    Cancelled,
    Conflict,
}

/// Operation timer for measuring duration
pub struct OperationTimer {
    start: Instant,
    trace_id: TraceId,
    operation: OperationType,
}

impl OperationTimer {
    /// Start timing an operation
    pub fn start(operation: OperationType) -> Self {
        Self {
            start: Instant::now(),
            trace_id: TraceId::new(),
            operation,
        }
    }

    /// Start with specific trace ID
    pub fn start_with_trace(trace_id: TraceId, operation: OperationType) -> Self {
        Self {
            start: Instant::now(),
            trace_id,
            operation,
        }
    }

    /// Finish timing and return log entry
    pub fn finish(self, outcome: OperationOutcome) -> LogEntry {
        let duration_ms = self.start.elapsed().as_millis() as u64;
        LogEntry::new(self.trace_id, self.operation, duration_ms, outcome)
    }

    /// Finish with error
    pub fn finish_with_error(self, error: String) -> LogEntry {
        let duration_ms = self.start.elapsed().as_millis() as u64;
        LogEntry::new(
            self.trace_id,
            self.operation,
            duration_ms,
            OperationOutcome::Failure,
        )
        .with_error(error)
    }
}

/// Operation statistics
#[derive(Debug, Clone, Default)]
struct OperationStats {
    count: u64,
    total_duration_ms: u64,
    failures: u64,
}

/// Metrics collector for aggregations
#[derive(Debug, Default)]
pub struct MetricsCollector {
    operations: HashMap<OperationType, OperationStats>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an operation outcome
    pub fn record(&mut self, entry: &LogEntry) {
        let stats = self
            .operations
            .entry(entry.operation)
            .or_insert_with(OperationStats::default);
        stats.count += 1;
        stats.total_duration_ms += entry.duration_ms;
        if matches!(entry.outcome, OperationOutcome::Failure) {
            stats.failures += 1;
        }
    }

    /// Get operation count
    pub fn operation_count(&self, op: OperationType) -> u64 {
        self.operations.get(&op).map(|s| s.count).unwrap_or(0)
    }

    /// Get failure count
    pub fn failure_count(&self, op: OperationType) -> u64 {
        self.operations.get(&op).map(|s| s.failures).unwrap_or(0)
    }

    /// Get average duration for an operation
    pub fn average_duration(&self, op: OperationType) -> Option<f64> {
        self.operations.get(&op).map(|s| {
            if s.count > 0 {
                s.total_duration_ms as f64 / s.count as f64
            } else {
                0.0
            }
        })
    }

    /// Get total duration for an operation
    pub fn total_duration(&self, op: OperationType) -> u64 {
        self.operations
            .get(&op)
            .map(|s| s.total_duration_ms)
            .unwrap_or(0)
    }
}

/// Restore operation metrics
#[derive(Debug, Clone, Default)]
pub struct RestoreMetrics {
    pub total_restores: u64,
    pub successful_restores: u64,
    pub failed_restores: u64,
    pub cancelled_restores: u64,
    pub conflicts_detected: u64,
}

impl RestoreMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_restore(&mut self, outcome: &OperationOutcome) {
        self.total_restores += 1;
        match outcome {
            OperationOutcome::Success => self.successful_restores += 1,
            OperationOutcome::Failure => self.failed_restores += 1,
            OperationOutcome::Cancelled => self.cancelled_restores += 1,
            OperationOutcome::Conflict => self.conflicts_detected += 1,
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_restores > 0 {
            self.successful_restores as f64 / self.total_restores as f64
        } else {
            0.0
        }
    }
}

/// Blob store metrics
#[derive(Debug, Clone, Default)]
pub struct BlobStoreMetrics {
    pub total_reads: u64,
    pub total_writes: u64,
    pub failed_reads: u64,
    pub failed_writes: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
}

impl BlobStoreMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_read(&mut self, bytes: u64, success: bool) {
        self.total_reads += 1;
        self.bytes_read += bytes;
        if !success {
            self.failed_reads += 1;
        }
    }

    pub fn record_write(&mut self, bytes: u64, success: bool) {
        self.total_writes += 1;
        self.bytes_written += bytes;
        if !success {
            self.failed_writes += 1;
        }
    }

    pub fn read_failure_rate(&self) -> f64 {
        if self.total_reads > 0 {
            self.failed_reads as f64 / self.total_reads as f64
        } else {
            0.0
        }
    }
}

/// Conflict metrics
#[derive(Debug, Clone, Default)]
pub struct ConflictMetrics {
    pub conflicts_detected: u64,
    pub conflicts_resolved_keep_current: u64,
    pub conflicts_resolved_restore_checkpoint: u64,
    pub conflicts_resolved_merge: u64,
    pub conflicts_resolved_save_and_restore: u64,
    pub conflicts_cancelled: u64,
}

impl ConflictMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_conflict(&mut self, resolution: &ConflictResolutionChoice) {
        self.conflicts_detected += 1;
        match resolution {
            ConflictResolutionChoice::KeepCurrent => self.conflicts_resolved_keep_current += 1,
            ConflictResolutionChoice::RestoreCheckpoint => {
                self.conflicts_resolved_restore_checkpoint += 1
            }
            ConflictResolutionChoice::Merge => self.conflicts_resolved_merge += 1,
            ConflictResolutionChoice::SaveCurrentAndRestore => {
                self.conflicts_resolved_save_and_restore += 1
            }
            ConflictResolutionChoice::Cancel => self.conflicts_cancelled += 1,
        }
    }
}

/// Conflict resolution choice
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolutionChoice {
    KeepCurrent,
    RestoreCheckpoint,
    Merge,
    SaveCurrentAndRestore,
    Cancel,
}

/// Replay diagnostics
#[derive(Debug, Clone, Default)]
pub struct ReplayDiagnostics {
    pub events_replayed: u64,
    pub replay_failures: u64,
    pub replay_time_ms: u64,
}

impl ReplayDiagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_event(&mut self) {
        self.events_replayed += 1;
    }

    pub fn record_failure(&mut self) {
        self.replay_failures += 1;
    }

    pub fn set_duration(&mut self, duration: Duration) {
        self.replay_time_ms = duration.as_millis() as u64;
    }

    pub fn events_per_second(&self) -> f64 {
        if self.replay_time_ms > 0 {
            (self.events_replayed as f64 / self.replay_time_ms as f64) * 1000.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_id_new() {
        let trace = TraceId::new();
        assert!(!trace.as_str().is_empty());
    }

    #[test]
    fn test_trace_id_display() {
        let trace = TraceId::from_string("abc123".to_string());
        assert_eq!(trace.to_string(), "trace:abc123");
    }

    #[test]
    fn test_trace_id_default() {
        let trace = TraceId::default();
        assert!(!trace.as_str().is_empty());
    }

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new(
            TraceId::new(),
            OperationType::SnapshotCreate,
            150,
            OperationOutcome::Success,
        );
        assert_eq!(entry.operation, OperationType::SnapshotCreate);
        assert_eq!(entry.duration_ms, 150);
        assert_eq!(entry.outcome, OperationOutcome::Success);
        assert!(entry.error_message.is_none());
        assert!(entry.session_id.is_none());
    }

    #[test]
    fn test_log_entry_with_error() {
        let entry = LogEntry::new(
            TraceId::new(),
            OperationType::RestoreExecute,
            500,
            OperationOutcome::Failure,
        )
        .with_error("blob not found".to_string());

        assert_eq!(entry.outcome, OperationOutcome::Failure);
        assert_eq!(entry.error_message, Some("blob not found".to_string()));
    }

    #[test]
    fn test_log_entry_with_session_id() {
        let entry = LogEntry::new(
            TraceId::new(),
            OperationType::EventAppend,
            10,
            OperationOutcome::Success,
        )
        .with_session_id("session_123".to_string());

        assert_eq!(entry.session_id, Some("session_123".to_string()));
    }

    #[test]
    fn test_log_entry_with_branch_id() {
        let entry = LogEntry::new(
            TraceId::new(),
            OperationType::BranchCreate,
            20,
            OperationOutcome::Success,
        )
        .with_branch_id("branch_456".to_string());

        assert_eq!(entry.branch_id, Some("branch_456".to_string()));
    }

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry::new(
            TraceId::from_string("test123".to_string()),
            OperationType::EventAppend,
            10,
            OperationOutcome::Success,
        );

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: LogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.operation, OperationType::EventAppend);
        assert_eq!(parsed.trace_id.as_str(), "test123");
    }

    #[test]
    fn test_log_entry_excludes_optional_fields_when_none() {
        let entry = LogEntry::new(
            TraceId::new(),
            OperationType::BlobRead,
            5,
            OperationOutcome::Success,
        );

        let json = serde_json::to_string(&entry).unwrap();
        assert!(!json.contains("error_message"));
        assert!(!json.contains("session_id"));
        assert!(!json.contains("branch_id"));
    }

    #[test]
    fn test_operation_timer_basic() {
        let timer = OperationTimer::start(OperationType::SnapshotCreate);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let entry = timer.finish(OperationOutcome::Success);

        assert!(entry.duration_ms >= 10);
        assert_eq!(entry.operation, OperationType::SnapshotCreate);
    }

    #[test]
    fn test_operation_timer_with_error() {
        let timer = OperationTimer::start(OperationType::RestoreExecute);
        let entry = timer.finish_with_error("checkpoint missing".to_string());

        assert_eq!(entry.outcome, OperationOutcome::Failure);
        assert_eq!(entry.error_message, Some("checkpoint missing".to_string()));
    }

    #[test]
    fn test_metrics_collector_record() {
        let mut collector = MetricsCollector::new();
        let entry = LogEntry::new(
            TraceId::new(),
            OperationType::BranchCreate,
            100,
            OperationOutcome::Success,
        );

        collector.record(&entry);
        assert_eq!(collector.operation_count(OperationType::BranchCreate), 1);
        assert_eq!(collector.failure_count(OperationType::BranchCreate), 0);
    }

    #[test]
    fn test_metrics_collector_failure_tracking() {
        let mut collector = MetricsCollector::new();

        // Record success
        collector.record(&LogEntry::new(
            TraceId::new(),
            OperationType::CheckpointCreate,
            50,
            OperationOutcome::Success,
        ));

        // Record failure
        collector.record(
            &LogEntry::new(
                TraceId::new(),
                OperationType::CheckpointCreate,
                200,
                OperationOutcome::Failure,
            )
            .with_error("timeout".to_string()),
        );

        assert_eq!(
            collector.operation_count(OperationType::CheckpointCreate),
            2
        );
        assert_eq!(collector.failure_count(OperationType::CheckpointCreate), 1);
    }

    #[test]
    fn test_metrics_collector_average_duration() {
        let mut collector = MetricsCollector::new();

        collector.record(&LogEntry::new(
            TraceId::new(),
            OperationType::BlobWrite,
            100,
            OperationOutcome::Success,
        ));
        collector.record(&LogEntry::new(
            TraceId::new(),
            OperationType::BlobWrite,
            200,
            OperationOutcome::Success,
        ));

        let avg = collector.average_duration(OperationType::BlobWrite);
        assert_eq!(avg, Some(150.0));
    }

    #[test]
    fn test_metrics_collector_total_duration() {
        let mut collector = MetricsCollector::new();

        collector.record(&LogEntry::new(
            TraceId::new(),
            OperationType::GcRun,
            50,
            OperationOutcome::Success,
        ));
        collector.record(&LogEntry::new(
            TraceId::new(),
            OperationType::GcRun,
            150,
            OperationOutcome::Success,
        ));

        assert_eq!(collector.total_duration(OperationType::GcRun), 200);
    }

    #[test]
    fn test_restore_metrics() {
        let mut metrics = RestoreMetrics::new();

        metrics.record_restore(&OperationOutcome::Success);
        metrics.record_restore(&OperationOutcome::Success);
        metrics.record_restore(&OperationOutcome::Failure);
        metrics.record_restore(&OperationOutcome::Cancelled);

        assert_eq!(metrics.total_restores, 4);
        assert_eq!(metrics.successful_restores, 2);
        assert_eq!(metrics.failed_restores, 1);
        assert_eq!(metrics.cancelled_restores, 1);
        assert_eq!(metrics.success_rate(), 0.5);
    }

    #[test]
    fn test_blob_store_metrics() {
        let mut metrics = BlobStoreMetrics::new();

        metrics.record_read(100, true);
        metrics.record_read(200, false);
        metrics.record_write(50, true);

        assert_eq!(metrics.total_reads, 2);
        assert_eq!(metrics.bytes_read, 300);
        assert_eq!(metrics.failed_reads, 1);
        assert_eq!(metrics.read_failure_rate(), 0.5);
    }

    #[test]
    fn test_conflict_metrics() {
        let mut metrics = ConflictMetrics::new();

        metrics.record_conflict(&ConflictResolutionChoice::KeepCurrent);
        metrics.record_conflict(&ConflictResolutionChoice::RestoreCheckpoint);
        metrics.record_conflict(&ConflictResolutionChoice::Cancel);

        assert_eq!(metrics.conflicts_detected, 3);
        assert_eq!(metrics.conflicts_resolved_keep_current, 1);
        assert_eq!(metrics.conflicts_resolved_restore_checkpoint, 1);
        assert_eq!(metrics.conflicts_cancelled, 1);
    }

    #[test]
    fn test_replay_diagnostics() {
        let mut diagnostics = ReplayDiagnostics::new();

        for _ in 0..100 {
            diagnostics.record_event();
        }
        diagnostics.record_failure();
        diagnostics.record_failure();
        diagnostics.set_duration(Duration::from_millis(500));

        assert_eq!(diagnostics.events_replayed, 100);
        assert_eq!(diagnostics.replay_failures, 2);
        assert_eq!(diagnostics.events_per_second(), 200.0);
    }

    #[test]
    fn test_all_operation_types_serialize() {
        for op_type in [
            OperationType::SnapshotCreate,
            OperationType::RestoreExecute,
            OperationType::EventAppend,
            OperationType::BlobRead,
            OperationType::BlobWrite,
            OperationType::BranchCreate,
            OperationType::BranchFork,
            OperationType::CheckpointCreate,
            OperationType::ConflictDetect,
            OperationType::ConflictResolve,
            OperationType::ReplayEvent,
            OperationType::GcRun,
            OperationType::ImportExport,
        ] {
            let json = serde_json::to_string(&op_type).unwrap();
            let parsed: OperationType = serde_json::from_str(&json).unwrap();
            assert_eq!(op_type, parsed);
        }
    }

    #[test]
    fn test_all_outcomes_serialize() {
        for outcome in [
            OperationOutcome::Success,
            OperationOutcome::Failure,
            OperationOutcome::Cancelled,
            OperationOutcome::Conflict,
        ] {
            let json = serde_json::to_string(&outcome).unwrap();
            let parsed: OperationOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(outcome, parsed);
        }
    }

    #[test]
    fn test_sensitive_content_not_in_log() {
        // Log entries should not contain sensitive paths or content
        let entry = LogEntry::new(
            TraceId::new(),
            OperationType::BlobRead,
            5,
            OperationOutcome::Success,
        );

        let json = serde_json::to_string(&entry).unwrap();
        // Should only contain metadata, not content
        assert!(!json.contains("content"));
        assert!(!json.contains("password"));
        assert!(!json.contains("secret"));
    }
}
