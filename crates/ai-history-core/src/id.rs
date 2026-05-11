//! Typed identifiers for the ai-history system
//!
//! All IDs are wrapper types around UUID v4 to ensure type safety
//! and prevent mixing different ID types.

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use uuid::Uuid;
/// Turn identifier - represents a single turn in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TurnId(Uuid);
impl TurnId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }}
