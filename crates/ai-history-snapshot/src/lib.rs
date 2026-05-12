// AI History Snapshot - Workspace snapshot and manifest management
//
// This crate provides:
// - Workspace scanning
// - Manifest generation
// - Blob writes
// - Snapshot comparison
// - Deduplication

#![cfg_attr(test, allow(unused_imports, unused_mut, unused_variables))]

pub mod blob_store;
pub mod comparison;
pub mod manifest;
pub use blob_store::*;
pub use comparison::*;
pub use manifest::*;
