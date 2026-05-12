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
pub mod planner;
pub use conflict::*;
