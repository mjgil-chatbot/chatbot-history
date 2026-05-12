// AI History Events - Versioned event definitions and serialization
//
// This crate defines:
// - Versioned event enums
// - Payload serialization
// - Event validation
// - Event checksums
// - Causation/correlation IDs
// - Replay interfaces

#![cfg_attr(test, allow(unused_imports, unused_variables))]

pub mod event;
pub mod replay;
pub mod schema;
pub use event::*;
