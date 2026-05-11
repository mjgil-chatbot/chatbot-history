//! Typed identifiers for the ai-history system
//!
//! All IDs are wrapper types around UUID v4 to ensure type safety
//! and prevent mixing different ID types.

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use uuid::Uuid;
/// Session identifier - represents a single AI interaction session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);
impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }}
/// Branch identifier - represents a branch in the conversation DAG
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchId(Uuid);
impl BranchId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }}
/// Turn identifier - represents a single turn in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TurnId(Uuid);
impl TurnId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }}
/// Checkpoint identifier - represents a workspace snapshot checkpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckpointId(Uuid);
impl CheckpointId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }}
