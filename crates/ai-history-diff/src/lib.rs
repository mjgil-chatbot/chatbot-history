// AI History Diff - Text diff and patch management
//
// This crate provides:
// - Text detection
// - Diff generation
// - Inverse diff generation
// - Patch validation
// - Three-way merge

#![cfg_attr(test, allow(unused_imports, unused_variables))]

pub mod diff;
pub mod merge;
pub mod patch;
#[cfg(test)]
mod scope_tests;

pub use diff::*;
pub use merge::*;
pub use patch::*;
