// AI History Testkit - Test fixtures and builders
//
// This crate provides:
// - Session builders
// - Branch builders
// - Event builders
// - Turn builders
// - Snapshot builders
// - Dirty workspace builders
// - Fake blob stores
// - Deterministic clocks
// - Acceptance scenarios
// - Completion evidence gate
// - Core boundary tests
// - Consolidation tests
// - FSM design tests
// - Store design tests
// - Snapshot design tests
// - Diff design tests
// - Restore design tests
// - Context design tests

#![cfg_attr(
    test,
    allow(
        dead_code,
        unused_comparisons,
        unused_imports,
        unused_mut,
        unused_variables
    )
)]

#[cfg(test)]
mod acceptance;
#[cfg(test)]
mod audit_tests;
#[cfg(test)]
mod branch_dag_tests;
#[cfg(test)]
mod branch_fork_policy_tests;
pub mod builders;
pub mod ci_validation;
#[cfg(test)]
mod cli_tests;
#[cfg(test)]
mod concurrency_security_tests;
#[cfg(test)]
mod consolidation_tests;
#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod core_boundary_tests;
#[cfg(test)]
mod cursor_semantics_tests;
#[cfg(test)]
mod diff_tests;
#[cfg(test)]
mod event_lifecycle_tests;
pub mod evidence_gate;
pub mod fakes;
#[cfg(test)]
mod final_tasks_tests;
#[cfg(test)]
mod forward_navigation_tests;
#[cfg(test)]
mod fsm_tests;
#[cfg(test)]
mod gc_retention_import_tests;
pub mod generators;
#[cfg(test)]
mod navigation_cli_tests;
#[cfg(test)]
mod replay_and_fixtures_tests;
#[cfg(test)]
mod restore_conflict_tests;
#[cfg(test)]
mod restore_tests;
#[cfg(test)]
mod scope_tests;
#[cfg(test)]
mod session_persistence_tests;
#[cfg(test)]
mod snapshot_tests;
pub use builders::*;
pub use ci_validation::*;
pub use evidence_gate::*;
pub use fakes::*;
pub use generators::*;
