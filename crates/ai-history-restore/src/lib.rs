// AI History Restore - Restore planning and execution
//
// This crate provides:
// - Restore planning
// - Conflict detection
// - Restore execution
// - Safety checkpoint integration
// - Verification

#![cfg_attr(test, allow(dead_code, unused_imports, unused_mut, unused_variables))]

pub mod conflict;
pub mod executor;
pub mod planner;

pub use conflict::*;
pub use executor::{
    execute_restore, execute_restore_transactional, verify_restore, verify_restore_simple,
    RestoreVerificationError,
};
pub use planner::*;
