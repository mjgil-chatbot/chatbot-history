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
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "session:{}", self.0)
    }
}

impl FromStr for SessionId {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = s.strip_prefix("session:") {
            rest.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        } else {
            s.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        }
    }
}

/// Branch identifier - represents a branch in the conversation DAG
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchId(Uuid);

impl BranchId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for BranchId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for BranchId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "branch:{}", self.0)
    }
}

impl FromStr for BranchId {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = s.strip_prefix("branch:") {
            rest.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        } else {
            s.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        }
    }
}

/// Turn identifier - represents a single turn in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TurnId(Uuid);

impl TurnId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for TurnId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for TurnId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "turn:{}", self.0)
    }
}

impl FromStr for TurnId {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = s.strip_prefix("turn:") {
            rest.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        } else {
            s.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        }
    }
}

/// Checkpoint identifier - represents a workspace snapshot checkpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckpointId(Uuid);

impl CheckpointId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for CheckpointId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for CheckpointId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "checkpoint:{}", self.0)
    }
}

impl FromStr for CheckpointId {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = s.strip_prefix("checkpoint:") {
            rest.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        } else {
            s.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        }
    }
}

/// Blob identifier - content-addressed blob reference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlobId(Uuid);

impl BlobId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for BlobId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for BlobId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "blob:{}", self.0)
    }
}

impl FromStr for BlobId {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = s.strip_prefix("blob:") {
            rest.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        } else {
            s.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        }
    }
}

/// Event sequence number - monotonically increasing per session/branch
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SequenceNum(u64);

impl SequenceNum {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn from_u64(n: u64) -> Self {
        Self(n)
    }

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for SequenceNum {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for SequenceNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "seq:{}", self.0)
    }
}

/// Manifest identifier - workspace snapshot manifest
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ManifestId(Uuid);

impl ManifestId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ManifestId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ManifestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "manifest:{}", self.0)
    }
}

impl FromStr for ManifestId {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = s.strip_prefix("manifest:") {
            rest.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        } else {
            s.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        }
    }
}

/// Patch identifier - file patch reference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PatchId(Uuid);

impl PatchId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for PatchId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for PatchId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "patch:{}", self.0)
    }
}

impl FromStr for PatchId {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = s.strip_prefix("patch:") {
            rest.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        } else {
            s.parse::<Uuid>().map(Self).map_err(|_| "invalid UUID")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_roundtrip() {
        let id = SessionId::new();
        let s = id.to_string();
        let parsed: SessionId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_branch_id_roundtrip() {
        let id = BranchId::new();
        let s = id.to_string();
        let parsed: BranchId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_turn_id_roundtrip() {
        let id = TurnId::new();
        let s = id.to_string();
        let parsed: TurnId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_checkpoint_id_roundtrip() {
        let id = CheckpointId::new();
        let s = id.to_string();
        let parsed: CheckpointId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_blob_id_roundtrip() {
        let id = BlobId::new();
        let s = id.to_string();
        let parsed: BlobId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_sequence_num_increment() {
        let seq = SequenceNum::new();
        assert_eq!(seq.as_u64(), 0);
        let next = seq.next();
        assert_eq!(next.as_u64(), 1);
    }

    #[test]
    fn test_manifest_id_roundtrip() {
        let id = ManifestId::new();
        let s = id.to_string();
        let parsed: ManifestId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_patch_id_roundtrip() {
        let id = PatchId::new();
        let s = id.to_string();
        let parsed: PatchId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }
}
