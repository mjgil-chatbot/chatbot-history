// AI History Context - Context snapshots for reproducibility
//
// This crate provides:
// - Context snapshot types
// - Selected file references
// - Retrieved chunk references
// - Prompt/rules hashes
// - Summary references
// - Tool output references

pub mod context;
pub mod reproducibility;
#[cfg(test)]
mod scope_tests;

pub use context::*;
pub use reproducibility::*;
