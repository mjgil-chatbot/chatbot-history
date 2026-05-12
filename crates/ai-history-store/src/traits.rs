//! Storage traits for pluggable backends

use ai_history_core::{CoreResult, SessionId};

/// Trait for session storage
pub trait SessionStore {
    fn create_session(&mut self, session_id: SessionId, workspace_root: &str) -> CoreResult<()>;
    fn get_session(&self, session_id: SessionId) -> CoreResult<Option<SessionInfo>>;
    fn update_session(&mut self, session_id: SessionId) -> CoreResult<()>;
    fn delete_session(&mut self, session_id: SessionId) -> CoreResult<()>;
    fn list_sessions(&self) -> CoreResult<Vec<SessionInfo>>;
}

/// Session information
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: SessionId,
    pub title: String,
    pub active_branch: ai_history_core::BranchId,
    pub view_cursor: Option<ai_history_core::TurnId>,
    pub workspace_root: String,
    pub retention_config: String,
    pub project_identity: String,
    pub operation_state: String,
    pub created_at: String,
    pub updated_at: String,
    pub archival_state: bool,
}

/// Trait for branch storage
pub trait BranchStore {
    fn create_branch(&mut self, branch: BranchInfo) -> CoreResult<()>;
    fn get_branch(&self, branch_id: ai_history_core::BranchId) -> CoreResult<Option<BranchInfo>>;
    fn update_branch_head(
        &mut self,
        branch_id: ai_history_core::BranchId,
        turn_id: ai_history_core::TurnId,
    ) -> CoreResult<()>;
    fn list_branches(&self, session_id: SessionId) -> CoreResult<Vec<BranchInfo>>;
}

/// Branch information
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub id: ai_history_core::BranchId,
    pub session_id: SessionId,
    pub name: String,
    pub parent_branch_id: Option<ai_history_core::BranchId>,
    pub created_from_turn_id: Option<ai_history_core::TurnId>,
    pub head_turn_id: Option<ai_history_core::TurnId>,
    pub ancestry: Vec<ai_history_core::BranchId>,
    pub protected: bool,
    pub archived: bool,
    pub abandoned_future: bool,
    pub created_at: String,
}

/// Trait for turn storage
pub trait TurnStore {
    fn create_turn(&mut self, turn: TurnInfo) -> CoreResult<()>;
    fn get_turn(&self, turn_id: ai_history_core::TurnId) -> CoreResult<Option<TurnInfo>>;
    fn finalize_turn(&mut self, turn_id: ai_history_core::TurnId) -> CoreResult<()>;
    fn get_turns(&self, branch_id: ai_history_core::BranchId) -> CoreResult<Vec<TurnInfo>>;
}

/// Turn information
#[derive(Debug, Clone)]
pub struct TurnInfo {
    pub id: ai_history_core::TurnId,
    pub session_id: SessionId,
    pub branch_id: ai_history_core::BranchId,
    pub parent_turn_id: Option<ai_history_core::TurnId>,
    pub actor: String,
    pub created_at: String,
    pub finalized_at: Option<String>,
}

/// Trait for checkpoint storage
pub trait CheckpointStore {
    fn create_checkpoint(&mut self, checkpoint: CheckpointInfo) -> CoreResult<()>;
    fn get_checkpoint(
        &self,
        checkpoint_id: ai_history_core::CheckpointId,
    ) -> CoreResult<Option<CheckpointInfo>>;
    fn list_checkpoints(&self, session_id: SessionId) -> CoreResult<Vec<CheckpointInfo>>;
    fn delete_checkpoint(&mut self, checkpoint_id: ai_history_core::CheckpointId)
        -> CoreResult<()>;
}

/// Checkpoint information
#[derive(Debug, Clone)]
pub struct CheckpointInfo {
    pub id: ai_history_core::CheckpointId,
    pub session_id: SessionId,
    pub branch_id: ai_history_core::BranchId,
    pub turn_id: Option<ai_history_core::TurnId>,
    pub manifest_id: ai_history_core::ManifestId,
    pub reason: String,
    pub created_at: String,
}

/// Trait for blob storage
pub trait BlobStore {
    fn write_blob(&mut self, hash: &[u8], data: &[u8]) -> CoreResult<()>;
    fn read_blob(&self, hash: &[u8]) -> CoreResult<Option<Vec<u8>>>;
    fn has_blob(&self, hash: &[u8]) -> bool;
    fn delete_blob(&mut self, hash: &[u8]) -> CoreResult<()>;
    fn list_blobs(&self, session_id: SessionId) -> CoreResult<Vec<BlobInfo>>;
}

/// Blob information
#[derive(Debug, Clone)]
pub struct BlobInfo {
    pub id: String,
    pub hash: String,
    pub size: u64,
    pub created_at: String,
}

// Blanket impl for mutable references - allows &mut T where T: BlobStore
impl<T: BlobStore + ?Sized> BlobStore for &mut T {
    fn write_blob(&mut self, hash: &[u8], data: &[u8]) -> CoreResult<()> {
        (**self).write_blob(hash, data)
    }
    fn read_blob(&self, hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
        (**self).read_blob(hash)
    }
    fn has_blob(&self, hash: &[u8]) -> bool {
        (**self).has_blob(hash)
    }
    fn delete_blob(&mut self, hash: &[u8]) -> CoreResult<()> {
        (**self).delete_blob(hash)
    }
    fn list_blobs(&self, session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
        (**self).list_blobs(session_id)
    }
}

/// Trait for manifest storage
pub trait ManifestStore {
    fn create_manifest(&mut self, manifest: ManifestInfo) -> CoreResult<()>;
    fn get_manifest(
        &self,
        manifest_id: ai_history_core::ManifestId,
    ) -> CoreResult<Option<ManifestInfo>>;
}

/// Manifest information
#[derive(Debug, Clone)]
pub struct ManifestInfo {
    pub id: ai_history_core::ManifestId,
    pub session_id: SessionId,
    pub checkpoint_id: ai_history_core::CheckpointId,
    pub manifest_data: String,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_info_debug() {
        let info = SessionInfo {
            id: SessionId::new(),
            title: "Test".to_string(),
            active_branch: ai_history_core::BranchId::new(),
            view_cursor: None,
            workspace_root: "/tmp".to_string(),
            retention_config: "default".to_string(),
            project_identity: "default".to_string(),
            operation_state: "active".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            archival_state: false,
        };
        assert!(format!("{:?}", info).contains("SessionInfo"));
    }

    #[test]
    fn test_branch_info_clone() {
        let info = BranchInfo {
            id: ai_history_core::BranchId::new(),
            session_id: SessionId::new(),
            name: "main".to_string(),
            parent_branch_id: None,
            created_from_turn_id: None,
            head_turn_id: None,
            ancestry: Vec::new(),
            protected: false,
            archived: false,
            abandoned_future: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let cloned = info.clone();
        assert_eq!(info.name, cloned.name);
        assert_eq!(info.id, cloned.id);
    }

    #[test]
    fn test_turn_info_clone() {
        let info = TurnInfo {
            id: ai_history_core::TurnId::new(),
            session_id: SessionId::new(),
            branch_id: ai_history_core::BranchId::new(),
            parent_turn_id: None,
            actor: "User".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            finalized_at: None,
        };
        let cloned = info.clone();
        assert_eq!(info.actor, cloned.actor);
    }

    #[test]
    fn test_checkpoint_info_clone() {
        let info = CheckpointInfo {
            id: ai_history_core::CheckpointId::new(),
            session_id: SessionId::new(),
            branch_id: ai_history_core::BranchId::new(),
            turn_id: Some(ai_history_core::TurnId::new()),
            manifest_id: ai_history_core::ManifestId::new(),
            reason: "test".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let cloned = info.clone();
        assert_eq!(info.reason, cloned.reason);
    }

    #[test]
    fn test_blob_info_clone() {
        let info = BlobInfo {
            id: "blob_1".to_string(),
            hash: "abc123".to_string(),
            size: 1024,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let cloned = info.clone();
        assert_eq!(info.hash, cloned.hash);
        assert_eq!(info.size, cloned.size);
    }

    #[test]
    fn test_session_info_equality() {
        let session_id = SessionId::new();
        let info1 = SessionInfo {
            id: session_id,
            title: "Test".to_string(),
            active_branch: ai_history_core::BranchId::new(),
            view_cursor: None,
            workspace_root: "/tmp".to_string(),
            retention_config: "default".to_string(),
            project_identity: "default".to_string(),
            operation_state: "active".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            archival_state: false,
        };
        let info2 = SessionInfo {
            id: session_id,
            title: "Test".to_string(),
            active_branch: ai_history_core::BranchId::new(),
            view_cursor: None,
            workspace_root: "/tmp".to_string(),
            retention_config: "default".to_string(),
            project_identity: "default".to_string(),
            operation_state: "active".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            archival_state: false,
        };
        assert_eq!(info1.id, info2.id);
    }

    #[test]
    fn test_branch_info_partial_eq() {
        let id = ai_history_core::BranchId::new();
        let info1 = BranchInfo {
            id,
            session_id: SessionId::new(),
            name: "main".to_string(),
            parent_branch_id: None,
            created_from_turn_id: None,
            head_turn_id: None,
            ancestry: Vec::new(),
            protected: false,
            archived: false,
            abandoned_future: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let info2 = BranchInfo {
            id,
            session_id: SessionId::new(),
            name: "main".to_string(),
            parent_branch_id: None,
            created_from_turn_id: None,
            head_turn_id: None,
            ancestry: Vec::new(),
            protected: false,
            archived: false,
            abandoned_future: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        assert_eq!(info1.id, info2.id);
        assert_eq!(info1.name, info2.name);
    }

    #[test]
    fn test_turn_info_with_parent() {
        let parent_turn_id = ai_history_core::TurnId::new();
        let info = TurnInfo {
            id: ai_history_core::TurnId::new(),
            session_id: SessionId::new(),
            branch_id: ai_history_core::BranchId::new(),
            parent_turn_id: Some(parent_turn_id),
            actor: "User".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            finalized_at: None,
        };
        assert!(info.parent_turn_id.is_some());
    }

    #[test]
    fn test_checkpoint_info_with_turn() {
        let turn_id = ai_history_core::TurnId::new();
        let info = CheckpointInfo {
            id: ai_history_core::CheckpointId::new(),
            session_id: SessionId::new(),
            branch_id: ai_history_core::BranchId::new(),
            turn_id: Some(turn_id),
            manifest_id: ai_history_core::ManifestId::new(),
            reason: "before_patch".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(info.turn_id.is_some());
        assert_eq!(info.reason, "before_patch");
    }

    #[test]
    fn test_blob_info_all_fields() {
        let info = BlobInfo {
            id: "blob_abc".to_string(),
            hash: "def456".to_string(),
            size: 2048,
            created_at: "2024-01-02T12:00:00Z".to_string(),
        };
        assert_eq!(info.id, "blob_abc");
        assert_eq!(info.hash, "def456");
        assert_eq!(info.size, 2048);
    }

    #[test]
    fn test_manifest_info_clone() {
        let info = ManifestInfo {
            id: ai_history_core::ManifestId::new(),
            session_id: SessionId::new(),
            checkpoint_id: ai_history_core::CheckpointId::new(),
            manifest_data: "{\"files\":[]}".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let cloned = info.clone();
        assert_eq!(info.id, cloned.id);
        assert_eq!(info.manifest_data, cloned.manifest_data);
    }
}

/// Compile-time tests verifying trait conformance for storage adapters
#[cfg(test)]
mod trait_conformance_tests {
    use super::*;
    use ai_history_core::{BranchId, CheckpointId, ManifestId, TurnId};

    /// Verify SessionInfo can be created and used with trait methods
    #[test]
    fn test_session_info_with_all_fields() {
        let info = SessionInfo {
            id: SessionId::new(),
            title: "Test Session".to_string(),
            active_branch: ai_history_core::BranchId::new(),
            view_cursor: None,
            workspace_root: "/workspace".to_string(),
            retention_config: "default".to_string(),
            project_identity: "project-1".to_string(),
            operation_state: "active".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            archival_state: false,
        };
        assert!(!info.workspace_root.is_empty());
    }

    /// Verify BranchInfo can be created with typed BranchId
    #[test]
    fn test_branch_info_with_typed_ids() {
        let branch_id = BranchId::new();
        let session_id = SessionId::new();
        let info = BranchInfo {
            id: branch_id,
            session_id,
            name: "feature-x".to_string(),
            parent_branch_id: None,
            created_from_turn_id: None,
            head_turn_id: None,
            ancestry: Vec::new(),
            protected: false,
            archived: false,
            abandoned_future: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        assert_eq!(info.name, "feature-x");
    }

    /// Verify TurnInfo can be created with all parent turn tracking
    #[test]
    fn test_turn_info_complete() {
        let turn_id = TurnId::new();
        let parent_turn_id = TurnId::new();
        let info = TurnInfo {
            id: turn_id,
            session_id: SessionId::new(),
            branch_id: BranchId::new(),
            parent_turn_id: Some(parent_turn_id),
            actor: "Assistant".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            finalized_at: Some("2024-01-01T00:01:00Z".to_string()),
        };
        assert!(info.parent_turn_id.is_some());
        assert!(info.finalized_at.is_some());
    }

    /// Verify CheckpointInfo with manifest and turn association
    #[test]
    fn test_checkpoint_info_complete() {
        let info = CheckpointInfo {
            id: CheckpointId::new(),
            session_id: SessionId::new(),
            branch_id: BranchId::new(),
            turn_id: Some(TurnId::new()),
            manifest_id: ManifestId::new(),
            reason: "before refactor".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(info.turn_id.is_some());
        assert!(!info.reason.is_empty());
    }

    /// Verify BlobInfo for content-addressed storage
    #[test]
    fn test_blob_info_for_deduplication() {
        let info = BlobInfo {
            id: "blob_abc123".to_string(),
            hash: "sha256:abc123def456".to_string(),
            size: 4096,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(info.hash.starts_with("sha256:"));
    }

    /// Verify ManifestInfo contains proper JSON data
    #[test]
    fn test_manifest_info_with_json() {
        let manifest_data = r#"{"version":1,"files":[{"path":"src/main.rs","hash":"abc123"}]}"#;
        let info = ManifestInfo {
            id: ManifestId::new(),
            session_id: SessionId::new(),
            checkpoint_id: CheckpointId::new(),
            manifest_data: manifest_data.to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        // Verify it's valid JSON by parsing
        let parsed: serde_json::Value = serde_json::from_str(&info.manifest_data).unwrap();
        assert_eq!(parsed["version"], 1);
    }
}
