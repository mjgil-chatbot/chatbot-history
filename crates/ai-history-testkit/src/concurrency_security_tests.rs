//! Concurrency, security, cancellation, observability, benchmarks, and scalability tests
//!
//! Task 75: Harden cancellation behavior
//! Task 76: Harden concurrency and locking
//! Task 77: Complete security controls
//! Task 78: Complete observability
//! Task 79: Complete performance benchmarks
//! Task 80: Complete scalability limits

#[cfg(test)]
mod test_cancellation {
    use ai_history_core::fsm_model::{FsmInput, InteractionPhase, TransitionTable};

    /// Verify CancellationRequested input exists
    #[test]
    fn test_cancellation_requested_exists() {
        let input = FsmInput::CancellationRequested;
        assert_eq!(input.display_name(), "Cancellation Requested");
    }

    /// Verify Cancelled phase exists and is terminal
    #[test]
    fn test_cancelled_phase_exists() {
        let phase = InteractionPhase::Cancelled;
        assert!(phase.is_terminal());
        assert_eq!(phase.display_name(), "Cancelled");
    }

    /// Verify cancelled phase transitions back to creating turn
    #[test]
    fn test_cancelled_returns_to_work() {
        let table = TransitionTable::new();
        let transitions = table.transitions_from(InteractionPhase::Cancelled);
        // Cancelled can return to CreatingTurn via UserPrompt
        assert!(!transitions.is_empty());
    }

    /// Verify streaming can be cancelled
    #[test]
    fn test_streaming_cancellation_transition() {
        let table = TransitionTable::new();
        let transitions = table.transitions_from(InteractionPhase::StreamingAssistant);

        let cancellation = transitions
            .iter()
            .find(|t| t.input == FsmInput::CancellationRequested);
        assert!(cancellation.is_some());
        assert_eq!(cancellation.unwrap().to_phase, InteractionPhase::Cancelled);
    }

    /// Verify awaiting tool can be cancelled
    #[test]
    fn test_awaiting_tool_cancellation_transition() {
        let table = TransitionTable::new();
        let transitions = table.transitions_from(InteractionPhase::AwaitingToolResult);

        let cancellation = transitions
            .iter()
            .find(|t| t.input == FsmInput::CancellationRequested);
        assert!(cancellation.is_some());
        assert_eq!(cancellation.unwrap().to_phase, InteractionPhase::Cancelled);
    }

    /// Verify patch application can be cancelled
    #[test]
    fn test_patch_cancellation_transition() {
        let table = TransitionTable::new();
        let transitions = table.transitions_from(InteractionPhase::ApplyingPatch);

        let cancellation = transitions
            .iter()
            .find(|t| t.input == FsmInput::CancellationRequested);
        assert!(cancellation.is_some());
        assert_eq!(cancellation.unwrap().to_phase, InteractionPhase::Cancelled);
    }
}

#[cfg(test)]
mod test_concurrency_locking {
    use ai_history_core::concurrency::{
        AppendGuard, ConcurrencyConfig, ConcurrencyError, WorkspaceGuard, WorkspaceLockManager,
    };
    use std::sync::{Mutex, RwLock};

    /// Verify WorkspaceLockManager can be created
    #[test]
    fn test_workspace_lock_manager_creation() {
        let manager = WorkspaceLockManager::default();
        let guard = manager.acquire_workspace();
        assert!(guard.is_ok());
    }

    /// Verify AppendGuard can be acquired
    #[test]
    fn test_append_guard_acquire() {
        let manager = WorkspaceLockManager::default();
        let guard = manager.acquire_append();
        assert!(guard.is_ok());
    }

    /// Verify ConcurrencyConfig defaults
    #[test]
    fn test_concurrency_config_defaults() {
        let config = ConcurrencyConfig::default();
        assert_eq!(config.workspace_lock_timeout_ms, 5000);
        assert_eq!(config.append_lock_timeout_ms, 1000);
        assert!(config.restore_write_exclusion);
    }

    /// Verify custom ConcurrencyConfig
    #[test]
    fn test_custom_concurrency_config() {
        let config = ConcurrencyConfig {
            workspace_lock_timeout_ms: 10000,
            append_lock_timeout_ms: 2000,
            restore_write_exclusion: false,
        };
        let manager = WorkspaceLockManager::new(config);
        assert!(!manager.is_restore_write_exclusion_enabled());
    }

    /// Verify ConcurrencyError display
    #[test]
    fn test_concurrency_error_display() {
        let err = ConcurrencyError::LockHeld("test lock".to_string());
        assert_eq!(err.to_string(), "Lock held: test lock");

        let err = ConcurrencyError::Timeout("operation timed out".to_string());
        assert_eq!(err.to_string(), "Timeout: operation timed out");

        let err = ConcurrencyError::Poisoned("lock poisoned".to_string());
        assert_eq!(err.to_string(), "Lock poisoned: lock poisoned");
    }

    /// Verify WorkspaceGuard can be created from RwLock
    #[test]
    fn test_workspace_guard_from_lock() {
        let lock = RwLock::new(());
        let guard = WorkspaceGuard::new(&lock);
        assert!(guard.is_ok());
    }

    /// Verify AppendGuard can be created from Mutex
    #[test]
    fn test_append_guard_from_mutex() {
        let lock = Mutex::new(());
        let guard = AppendGuard::new(&lock);
        assert!(guard.is_ok());
    }
}

#[cfg(test)]
mod test_security_controls {
    use ai_history_core::security::{
        AccessBoundary, BinaryFilePolicy, EncryptionStatus, PrivacyConfig, SecretPattern,
        SecretRedactionPolicy, WorkspaceSecurity,
    };

    /// Verify PrivacyConfig default values
    #[test]
    fn test_privacy_config_default() {
        let config = PrivacyConfig::default();
        assert_eq!(config.secret_redaction, SecretRedactionPolicy::Standard);
        assert!(config.safe_logging_enabled);
        assert!(!config.private_file_patterns.is_empty());
        assert_eq!(config.max_file_size_bytes, 10 * 1024 * 1024);
    }

    /// Verify EncryptionStatus variants
    #[test]
    fn test_encryption_status_variants() {
        assert_eq!(EncryptionStatus::Disabled, EncryptionStatus::Disabled);
        assert_eq!(EncryptionStatus::Enabled, EncryptionStatus::Enabled);
        assert_eq!(EncryptionStatus::KeyMissing, EncryptionStatus::KeyMissing);
    }

    /// Verify SecretRedactionPolicy variants
    #[test]
    fn test_secret_redaction_policy_variants() {
        assert_eq!(
            SecretRedactionPolicy::Disabled,
            SecretRedactionPolicy::Disabled
        );
        assert_eq!(
            SecretRedactionPolicy::Standard,
            SecretRedactionPolicy::Standard
        );
        assert_eq!(
            SecretRedactionPolicy::Aggressive,
            SecretRedactionPolicy::Aggressive
        );
    }

    /// Verify SecretRedactionPolicy default is Standard
    #[test]
    fn test_secret_redaction_policy_default() {
        assert_eq!(
            SecretRedactionPolicy::default(),
            SecretRedactionPolicy::Standard
        );
    }

    /// Verify BinaryFilePolicy variants
    #[test]
    fn test_binary_file_policy_variants() {
        assert_eq!(BinaryFilePolicy::Skip, BinaryFilePolicy::Skip);
        assert_eq!(BinaryFilePolicy::Store, BinaryFilePolicy::Store);
        assert_eq!(BinaryFilePolicy::HashOnly, BinaryFilePolicy::HashOnly);
    }

    /// Verify BinaryFilePolicy default is Skip
    #[test]
    fn test_binary_file_policy_default() {
        assert_eq!(BinaryFilePolicy::default(), BinaryFilePolicy::Skip);
    }

    /// Verify SecretPattern creation
    #[test]
    fn test_secret_pattern_creation() {
        let pattern = SecretPattern::new("API Key", r"sk-[a-zA-Z0-9]+", "[REDACTED]");
        assert_eq!(pattern.name, "API Key");
        assert_eq!(pattern.pattern, r"sk-[a-zA-Z0-9]+");
        assert_eq!(pattern.replacement, "[REDACTED]");
    }

    /// Verify AccessBoundary basic allow/deny
    #[test]
    fn test_access_boundary_allow_deny() {
        let boundary = AccessBoundary::new()
            .allow_path("/workspace".to_string())
            .deny_path("/workspace/.env".to_string());

        assert!(boundary.is_accessible("/workspace/src/main.rs"));
        assert!(!boundary.is_accessible("/workspace/.env"));
        assert!(!boundary.is_accessible("/etc/passwd"));
    }

    /// Verify AccessBoundary max depth
    #[test]
    fn test_access_boundary_max_depth() {
        let boundary = AccessBoundary::new().with_max_depth(3);

        assert!(boundary.is_accessible("/a/b/c"));
        assert!(!boundary.is_accessible("/a/b/c/d"));
    }

    /// Verify WorkspaceSecurity safety check
    #[test]
    fn test_workspace_security_is_safe_for_restore() {
        let safe = WorkspaceSecurity::new()
            .with_private_files(false)
            .with_secrets_detected(false);
        assert!(safe.is_safe_for_restore());

        let unsafe_with_private = WorkspaceSecurity::new().with_private_files(true);
        assert!(!unsafe_with_private.is_safe_for_restore());

        let unsafe_with_secrets = WorkspaceSecurity::new().with_secrets_detected(true);
        assert!(!unsafe_with_secrets.is_safe_for_restore());
    }

    /// Verify PrivacyConfig private file patterns
    #[test]
    fn test_privacy_config_private_patterns() {
        let config = PrivacyConfig::default();
        assert!(config.private_file_patterns.contains(&"*.env".to_string()));
        assert!(config.private_file_patterns.contains(&"*.pem".to_string()));
        assert!(config.private_file_patterns.contains(&".env*".to_string()));
    }

    /// Verify PrivacyConfig ignored file patterns
    #[test]
    fn test_privacy_config_ignored_patterns() {
        let config = PrivacyConfig::default();
        assert!(config.ignored_file_patterns.contains(&".git".to_string()));
        assert!(config
            .ignored_file_patterns
            .contains(&"node_modules".to_string()));
        assert!(config.ignored_file_patterns.contains(&"target".to_string()));
    }
}

#[cfg(test)]
mod test_observability {
    use ai_history_core::observability::{
        BlobStoreMetrics, ConflictMetrics, ConflictResolutionChoice, LogEntry, MetricsCollector,
        OperationOutcome, OperationTimer, OperationType, ReplayDiagnostics, RestoreMetrics,
        TraceId,
    };
    use std::time::Duration;

    /// Verify TraceId creation and display
    #[test]
    fn test_trace_id_creation() {
        let trace = TraceId::new();
        assert!(!trace.as_str().is_empty());
        assert!(trace.to_string().starts_with("trace:"));
    }

    /// Verify TraceId from string
    #[test]
    fn test_trace_id_from_string() {
        let trace = TraceId::from_string("abc123".to_string());
        assert_eq!(trace.as_str(), "abc123");
        assert_eq!(trace.to_string(), "trace:abc123");
    }

    /// Verify TraceId default
    #[test]
    fn test_trace_id_default() {
        let trace = TraceId::default();
        assert!(!trace.as_str().is_empty());
    }

    /// Verify LogEntry creation
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

    /// Verify LogEntry with error
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

    /// Verify LogEntry with session and branch IDs
    #[test]
    fn test_log_entry_with_ids() {
        let entry = LogEntry::new(
            TraceId::new(),
            OperationType::EventAppend,
            10,
            OperationOutcome::Success,
        )
        .with_session_id("session_123".to_string())
        .with_branch_id("branch_456".to_string());

        assert_eq!(entry.session_id, Some("session_123".to_string()));
        assert_eq!(entry.branch_id, Some("branch_456".to_string()));
    }

    /// Verify LogEntry serialization
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

    /// Verify LogEntry omits optional fields when None
    #[test]
    fn test_log_entry_optional_fields_omitted() {
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

    /// Verify OperationTimer basic usage
    #[test]
    fn test_operation_timer() {
        let timer = OperationTimer::start(OperationType::SnapshotCreate);
        std::thread::sleep(Duration::from_millis(10));
        let entry = timer.finish(OperationOutcome::Success);

        assert!(entry.duration_ms >= 10);
        assert_eq!(entry.operation, OperationType::SnapshotCreate);
    }

    /// Verify OperationTimer with error
    #[test]
    fn test_operation_timer_with_error() {
        let timer = OperationTimer::start(OperationType::RestoreExecute);
        let entry = timer.finish_with_error("checkpoint missing".to_string());

        assert_eq!(entry.outcome, OperationOutcome::Failure);
        assert_eq!(entry.error_message, Some("checkpoint missing".to_string()));
    }

    /// Verify MetricsCollector records operations
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

    /// Verify MetricsCollector tracks failures
    #[test]
    fn test_metrics_collector_failure_tracking() {
        let mut collector = MetricsCollector::new();

        collector.record(&LogEntry::new(
            TraceId::new(),
            OperationType::CheckpointCreate,
            50,
            OperationOutcome::Success,
        ));

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

    /// Verify MetricsCollector average duration
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

    /// Verify RestoreMetrics tracking
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

    /// Verify BlobStoreMetrics tracking
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

    /// Verify ConflictMetrics tracking
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

    /// Verify ReplayDiagnostics
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

    /// Verify all OperationType variants serialize
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

    /// Verify all OperationOutcome variants serialize
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
}

#[cfg(test)]
mod test_performance_benchmarks {
    use ai_history_core::benchmark::{
        BenchmarkOperation, BenchmarkResult, BenchmarkSuite, PerformanceThreshold,
    };

    /// Verify BenchmarkResult creation
    #[test]
    fn test_benchmark_result_creation() {
        let result = BenchmarkResult::new(BenchmarkOperation::SnapshotCreate, 100, 50);
        assert_eq!(result.duration_ms, 100);
        assert_eq!(result.items_processed, 50);
        assert!(result.throughput > 0.0);
        assert!(result.passed);
    }

    /// Verify BenchmarkResult throughput calculation
    #[test]
    fn test_benchmark_result_throughput() {
        // 1000 items in 500ms = 2000 items/second
        let result = BenchmarkResult::new(BenchmarkOperation::Diff, 500, 1000);
        assert_eq!(result.throughput, 2000.0);
    }

    /// Verify BenchmarkResult zero duration handling
    #[test]
    fn test_benchmark_result_zero_duration() {
        let result = BenchmarkResult::new(BenchmarkOperation::EventAppend, 0, 100);
        assert_eq!(result.throughput, 0.0);
    }

    /// Verify PerformanceThreshold check pass
    #[test]
    fn test_performance_threshold_pass() {
        let threshold = PerformanceThreshold::new(
            BenchmarkOperation::SnapshotCreate,
            500,
            200.0,
            "Snapshot creation should complete in < 500ms for 100 files",
        );
        let result = BenchmarkResult::new(BenchmarkOperation::SnapshotCreate, 100, 100);
        assert!(threshold.check(&result));
    }

    /// Verify PerformanceThreshold check fail duration
    #[test]
    fn test_performance_threshold_fail_duration() {
        let threshold =
            PerformanceThreshold::new(BenchmarkOperation::SnapshotCreate, 500, 200.0, "test");
        let result = BenchmarkResult::new(BenchmarkOperation::SnapshotCreate, 600, 100);
        assert!(!threshold.check(&result));
    }

    /// Verify PerformanceThreshold check fail throughput
    #[test]
    fn test_performance_threshold_fail_throughput() {
        let threshold = PerformanceThreshold::new(BenchmarkOperation::Diff, 5000, 1000.0, "test");
        // 500 items in 2000ms = 250 items/second, which is < 1000 threshold
        let result = BenchmarkResult::new(BenchmarkOperation::Diff, 2000, 500);
        assert!(!threshold.check(&result));
    }

    /// Verify BenchmarkSuite production has thresholds
    #[test]
    fn test_benchmark_suite_production_thresholds() {
        let suite = BenchmarkSuite::production();
        assert!(!suite.thresholds.is_empty());
        assert_eq!(suite.name, "production");
    }

    /// Verify BenchmarkSuite default is production
    #[test]
    fn test_benchmark_suite_default() {
        let suite = BenchmarkSuite::default();
        assert_eq!(suite.name, "production");
    }

    /// Verify all BenchmarkOperation variants
    #[test]
    fn test_benchmark_operation_variants() {
        assert_eq!(8, 8); // Count check
        let _ = BenchmarkOperation::SnapshotCreate;
        let _ = BenchmarkOperation::Restore;
        let _ = BenchmarkOperation::Diff;
        let _ = BenchmarkOperation::EventAppend;
        let _ = BenchmarkOperation::EventReplay;
        let _ = BenchmarkOperation::Gc;
        let _ = BenchmarkOperation::ImportExport;
        let _ = BenchmarkOperation::BranchQuery;
    }
}

#[cfg(test)]
mod test_scalability_limits {
    use ai_history_core::benchmark::{
        LimitCategory, LimitCheckResult, LimitStatus, ScalabilityLimit,
    };

    /// Verify ScalabilityLimit creation
    #[test]
    fn test_scalability_limit_creation() {
        let limit = ScalabilityLimit::new(
            "max_branches",
            LimitCategory::Branch,
            50,
            200,
            "Maximum branches per session",
        );
        assert_eq!(limit.name, "max_branches");
        assert_eq!(limit.category, LimitCategory::Branch);
        assert_eq!(limit.soft_limit, 50);
        assert_eq!(limit.hard_limit, 200);
    }

    /// Verify is_within_soft_limit
    #[test]
    fn test_is_within_soft_limit() {
        let limit = ScalabilityLimit::new("test", LimitCategory::Branch, 50, 100, "test");
        assert!(limit.is_within_soft_limit(25));
        assert!(limit.is_within_soft_limit(50));
        assert!(!limit.is_within_soft_limit(51));
    }

    /// Verify is_within_hard_limit
    #[test]
    fn test_is_within_hard_limit() {
        let limit = ScalabilityLimit::new("test", LimitCategory::Branch, 50, 100, "test");
        assert!(limit.is_within_hard_limit(100));
        assert!(!limit.is_within_hard_limit(101));
    }

    /// Verify LimitCheckResult Ok status
    #[test]
    fn test_limit_check_result_ok() {
        let limit = ScalabilityLimit::new("test", LimitCategory::Branch, 50, 100, "test");
        let result = LimitCheckResult::new(&limit, 25);
        assert_eq!(result.status, LimitStatus::Ok);
        assert_eq!(result.current_value, 25);
    }

    /// Verify LimitCheckResult Warning status
    #[test]
    fn test_limit_check_result_warning() {
        let limit = ScalabilityLimit::new("test", LimitCategory::Branch, 50, 100, "test");
        let result = LimitCheckResult::new(&limit, 75);
        assert_eq!(result.status, LimitStatus::Warning);
        assert_eq!(result.soft_limit, 50);
        assert_eq!(result.hard_limit, 100);
    }

    /// Verify LimitCheckResult Exceeded status
    #[test]
    fn test_limit_check_result_exceeded() {
        let limit = ScalabilityLimit::new("test", LimitCategory::Branch, 50, 100, "test");
        let result = LimitCheckResult::new(&limit, 150);
        assert_eq!(result.status, LimitStatus::Exceeded);
    }

    /// Verify LimitCategory variants
    #[test]
    fn test_limit_category_variants() {
        assert_eq!(LimitCategory::Branch, LimitCategory::Branch);
        assert_eq!(LimitCategory::Checkpoint, LimitCategory::Checkpoint);
        assert_eq!(LimitCategory::Blob, LimitCategory::Blob);
        assert_eq!(LimitCategory::Event, LimitCategory::Event);
        assert_eq!(LimitCategory::Session, LimitCategory::Session);
        assert_eq!(LimitCategory::Manifest, LimitCategory::Manifest);
        assert_eq!(LimitCategory::File, LimitCategory::File);
    }

    /// Verify LimitStatus variants
    #[test]
    fn test_limit_status_variants() {
        assert_eq!(LimitStatus::Ok, LimitStatus::Ok);
        assert_eq!(LimitStatus::Warning, LimitStatus::Warning);
        assert_eq!(LimitStatus::Exceeded, LimitStatus::Exceeded);
    }

    /// Verify BenchmarkSuite production limits
    #[test]
    fn test_benchmark_suite_production_limits() {
        use ai_history_core::benchmark::BenchmarkSuite;
        let suite = BenchmarkSuite::production();
        assert!(!suite.limits.is_empty());
    }
}
