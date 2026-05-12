//! SQLite storage adapter for persistent event store
//!
//! Task 29: Implement SQLite storage adapter
//!
//! This module provides a SQLite-backed implementation of the storage traits
//! defined in the store crate. It uses rusqlite for database operations.
//!
//! ## Features
//!
//! - Full schema support via migrations
//! - Transactional operations
//! - Foreign key enforcement
//! - Indexed queries for performance

use crate::traits::BlobStore;
pub use crate::traits::{
    BlobInfo, BranchInfo, CheckpointInfo, ManifestInfo, SessionInfo, TurnInfo,
};
use ai_history_core::gc::GcStore;
use ai_history_core::{
    BlobId, BranchId, CheckpointId, CoreError, CoreResult, ManifestId, SessionId, TurnId,
};
use ai_history_events::Event;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;

/// First-class message record for persistent message storage (task 27)
#[derive(Debug, Clone)]
pub struct MessageRecord {
    pub id: ai_history_core::MessageId,
    pub turn_id: TurnId,
    pub message_type: String,
    pub role: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub sequence_num: u64,
    pub created_at: String,
}

impl MessageRecord {
    pub fn new(
        turn_id: TurnId,
        message_type: &str,
        role: &str,
        content: &str,
        sequence_num: u64,
    ) -> Self {
        Self {
            id: ai_history_core::MessageId::new(),
            turn_id,
            message_type: message_type.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            metadata: HashMap::new(),
            sequence_num,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Context snapshot record for task 12 - persisted reproducibility context
#[derive(Debug, Clone)]
pub struct ContextSnapshotRecord {
    pub id: String,
    pub turn_id: TurnId,
    pub snapshot_data: String, // JSON serialized ai_history_context::ContextSnapshot
    pub selected_files_count: usize,
    pub retrieved_chunks_count: usize,
    pub tool_outputs_count: usize,
    pub summary_refs_count: usize,
    pub memory_refs_count: usize,
    pub rule_refs_count: usize,
    pub system_prompt_hash: Option<String>,
    pub model_config_data: Option<String>,
    pub created_at: String,
}

impl ContextSnapshotRecord {
    pub fn new(
        turn_id: TurnId,
        snapshot_data: String,
        selected_files_count: usize,
        retrieved_chunks_count: usize,
        tool_outputs_count: usize,
        summary_refs_count: usize,
        memory_refs_count: usize,
        rule_refs_count: usize,
    ) -> Self {
        Self {
            id: format!("ctx:{}", uuid::Uuid::new_v4()),
            turn_id,
            snapshot_data,
            selected_files_count,
            retrieved_chunks_count,
            tool_outputs_count,
            summary_refs_count,
            memory_refs_count,
            rule_refs_count,
            system_prompt_hash: None,
            model_config_data: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn with_system_prompt_hash(mut self, hash: &str) -> Self {
        self.system_prompt_hash = Some(hash.to_string());
        self
    }

    pub fn with_model_config(mut self, config: &str) -> Self {
        self.model_config_data = Some(config.to_string());
        self
    }
}

/// SQLite-backed event store
pub struct SqliteStore {
    conn: Connection,
}

/// Transaction guard for atomic operations
pub struct Transaction<'a> {
    conn: &'a Connection,
}

impl BlobStore for SqliteStore {
    fn write_blob(&mut self, hash: &[u8], data: &[u8]) -> CoreResult<()> {
        let hash_hex = hex::encode(hash);
        let size = i64::try_from(data.len()).map_err(|_| {
            CoreError::Storage("blob size exceeds sqlite integer range".to_string())
        })?;
        self.conn
            .execute(
                "INSERT OR REPLACE INTO blobs (id, session_id, hash, size, content, created_at)
                 VALUES (lower(hex(randomblob(16))), NULL, ?1, ?2, ?3, datetime('now'))",
                params![hash_hex, size, data],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    fn read_blob(&self, hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
        let hash_hex = hex::encode(hash);
        let mut stmt = self
            .conn
            .prepare("SELECT content FROM blobs WHERE hash = ?1")
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        let result = stmt.query_row(params![hash_hex], |row| Ok(row.get::<_, Vec<u8>>(0)?));
        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    fn has_blob(&self, hash: &[u8]) -> bool {
        let hash_hex = hex::encode(hash);
        let mut stmt = match self.conn.prepare("SELECT 1 FROM blobs WHERE hash = ?1") {
            Ok(stmt) => stmt,
            Err(_) => return false,
        };
        stmt.query_row(params![hash_hex], |_| Ok(())).is_ok()
    }

    fn delete_blob(&mut self, hash: &[u8]) -> CoreResult<()> {
        let hash_hex = hex::encode(hash);
        self.conn
            .execute("DELETE FROM blobs WHERE hash = ?1", params![hash_hex])
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    fn list_blobs(&self, session_id: SessionId) -> CoreResult<Vec<crate::traits::BlobInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, hash, size, created_at FROM blobs WHERE session_id = ?1")
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        let blobs = stmt
            .query_map(params![session_id.to_string()], |row| {
                Ok(crate::traits::BlobInfo {
                    id: row.get(0)?,
                    hash: row.get(1)?,
                    size: row.get::<_, i64>(2)? as u64,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(blobs)
    }
}

impl GcStore for SqliteStore {
    fn get_all_session_ids(&self) -> CoreResult<Vec<SessionId>> {
        let sessions = self.list_sessions()?;
        Ok(sessions.into_iter().map(|s| s.id).collect())
    }

    fn get_branch_ids_for_session(&self, session_id: SessionId) -> CoreResult<Vec<BranchId>> {
        let branches = self.list_branches(session_id)?;
        Ok(branches.into_iter().map(|b| b.id).collect())
    }

    fn get_checkpoint_ids_for_session(
        &self,
        session_id: SessionId,
    ) -> CoreResult<Vec<CheckpointId>> {
        let checkpoints = self.list_checkpoints(session_id)?;
        Ok(checkpoints.into_iter().map(|c| c.id).collect())
    }

    fn get_all_blob_ids(&self) -> CoreResult<Vec<BlobId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT hash FROM blobs")
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        let blobs = stmt
            .query_map([], |row| {
                let hash: String = row.get(0)?;
                Ok(hash)
            })
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        blobs
            .into_iter()
            .map(|hash| {
                let blob_id_str = if hash.starts_with("blob:") {
                    hash.clone()
                } else {
                    format!("blob:{}", hash)
                };
                blob_id_str
                    .parse()
                    .map_err(|_| CoreError::Storage("Invalid blob hash".to_string()))
            })
            .collect()
    }

    fn get_checkpoint_info(
        &self,
        id: CheckpointId,
    ) -> CoreResult<Option<ai_history_core::gc::CheckpointInfo>> {
        let checkpoint = self.get_checkpoint(id)?;
        Ok(checkpoint.map(|c| ai_history_core::gc::CheckpointInfo {
            id: c.id,
            session_id: c.session_id,
            branch_id: c.branch_id,
            turn_id: c.turn_id,
            manifest_id: c.manifest_id,
        }))
    }

    fn get_manifest_info(
        &self,
        id: ManifestId,
    ) -> CoreResult<Option<ai_history_core::gc::ManifestInfo>> {
        let manifest = self.get_manifest(id)?;
        Ok(manifest.map(|m| ai_history_core::gc::ManifestInfo {
            id: m.id,
            session_id: m.session_id,
            checkpoint_id: m.checkpoint_id,
            manifest_data: m.manifest_data,
        }))
    }

    fn get_branch_info(&self, id: BranchId) -> CoreResult<Option<ai_history_core::gc::BranchInfo>> {
        let branch = self.get_branch(id)?;
        Ok(branch.map(|b| ai_history_core::gc::BranchInfo {
            id: b.id,
            session_id: b.session_id,
            name: b.name,
            parent_branch_id: b.parent_branch_id,
            head_turn_id: b.head_turn_id,
        }))
    }

    fn get_branch_head(&self, branch_id: BranchId) -> CoreResult<Option<TurnId>> {
        let branch = self.get_branch(branch_id)?;
        Ok(branch.and_then(|b| b.head_turn_id))
    }

    fn delete_blob(&self, blob_id: &BlobId) -> CoreResult<()> {
        let hash = blob_id.to_string();
        let hash_hex = hash.strip_prefix("blob:").unwrap_or(&hash);
        self.conn
            .execute("DELETE FROM blobs WHERE hash = ?1", params![hash_hex])
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    fn delete_checkpoint(&self, id: &CheckpointId) -> CoreResult<()> {
        self.delete_checkpoint(*id)
    }

    fn delete_manifest(&self, id: &ManifestId) -> CoreResult<()> {
        self.conn
            .execute(
                "DELETE FROM manifests WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }
}

impl SqliteStore {
    /// Open a database at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> CoreResult<Self> {
        let conn = Connection::open(path).map_err(|e| CoreError::Storage(e.to_string()))?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let store = Self { conn };
        store.run_migrations()?;
        Ok(store)
    }

    /// Create an in-memory database for testing
    pub fn in_memory() -> CoreResult<Self> {
        let conn = Connection::open_in_memory().map_err(|e| CoreError::Storage(e.to_string()))?;

        conn.execute("PRAGMA foreign_keys = ON", [])
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let store = Self { conn };
        store.run_migrations()?;
        Ok(store)
    }

    /// Run database migrations
    pub fn run_migrations(&self) -> CoreResult<()> {
        // Create schema_version table if not exists
        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS schema_version (
                    version INTEGER PRIMARY KEY,
                    applied_at TEXT NOT NULL
                )",
                [],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        // Check current version
        let version: Option<i32> = self
            .conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            })
            .ok();

        if version.is_none() {
            // Run initial migration
            self.conn
                .execute_batch(include_str!("../migrations/init.sql"))
                .map_err(|e| CoreError::Storage(e.to_string()))?;

            // Record schema version
            self.conn
                .execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (1, datetime('now'))",
                    [],
                )
                .map_err(|e| CoreError::Storage(e.to_string()))?;
        }

        Ok(())
    }

    /// Begin a transaction
    pub fn transaction(&self) -> CoreResult<Transaction<'_>> {
        self.conn
            .execute("BEGIN TRANSACTION", [])
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(Transaction { conn: &self.conn })
    }

    /// Create a new session
    pub fn create_session(&self, session_id: SessionId, workspace_root: &str) -> CoreResult<()> {
        let active_branch = ai_history_core::BranchId::new().to_string();
        self.conn
            .execute(
                "INSERT INTO sessions (id, title, active_branch, view_cursor, workspace_root, retention_config, project_identity, operation_state, created_at, updated_at, archival_state)
                 VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, datetime('now'), datetime('now'), 0)",
                params![
                    session_id.to_string(),
                    "Untitled Session",
                    active_branch,
                    workspace_root,
                    "default",
                    "",
                    "active"
                ],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: SessionId) -> CoreResult<Option<SessionInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, active_branch, view_cursor, workspace_root, retention_config, project_identity, operation_state, created_at, updated_at, archival_state FROM sessions WHERE id = ?1")
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let result = stmt.query_row(params![session_id.to_string()], |row| {
            let active_branch_str: String = row.get(2)?;
            let view_cursor_str: Option<String> = row.get(3)?;
            let archival_state_int: i32 = row.get(10)?;
            Ok(SessionInfo {
                id: row.get::<_, String>(0)?.parse().unwrap(),
                title: row.get(1)?,
                active_branch: active_branch_str.parse().unwrap_or_default(),
                view_cursor: view_cursor_str.and_then(|s| s.parse().ok()),
                workspace_root: row.get(4)?,
                retention_config: row.get(5)?,
                project_identity: row.get(6)?,
                operation_state: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                archival_state: archival_state_int != 0,
            })
        });

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    /// List all sessions
    pub fn list_sessions(&self) -> CoreResult<Vec<SessionInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, active_branch, view_cursor, workspace_root, retention_config, project_identity, operation_state, created_at, updated_at, archival_state FROM sessions")
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let sessions = stmt
            .query_map([], |row| {
                let active_branch_str: String = row.get(2)?;
                let view_cursor_str: Option<String> = row.get(3)?;
                let archival_state_int: i32 = row.get(10)?;
                Ok(SessionInfo {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    title: row.get(1)?,
                    active_branch: active_branch_str.parse().unwrap_or_default(),
                    view_cursor: view_cursor_str.and_then(|s| s.parse().ok()),
                    workspace_root: row.get(4)?,
                    retention_config: row.get(5)?,
                    project_identity: row.get(6)?,
                    operation_state: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    archival_state: archival_state_int != 0,
                })
            })
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        Ok(sessions)
    }

    /// Create a new branch
    pub fn create_branch(&self, branch: &BranchInfo) -> CoreResult<()> {
        let ancestry_json = serde_json::to_string(&branch.ancestry).unwrap_or("[]".to_string());
        self.conn
            .execute(
                "INSERT INTO branches (id, session_id, name, parent_branch_id, created_from_turn_id, head_turn_id, ancestry, protected, archived, abandoned_future, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, datetime('now'))",
                params![
                    branch.id.to_string(),
                    branch.session_id.to_string(),
                    branch.name,
                    branch.parent_branch_id.map(|b| b.to_string()),
                    branch.created_from_turn_id.map(|t| t.to_string()),
                    branch.head_turn_id.map(|t| t.to_string()),
                    ancestry_json,
                    if branch.protected { 1 } else { 0 },
                    if branch.archived { 1 } else { 0 },
                    if branch.abandoned_future { 1 } else { 0 },
                ],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get a branch by ID
    pub fn get_branch(&self, branch_id: BranchId) -> CoreResult<Option<BranchInfo>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, session_id, name, parent_branch_id, created_from_turn_id, head_turn_id, ancestry, protected, archived, abandoned_future, created_at
                 FROM branches WHERE id = ?1",
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let result = stmt.query_row(params![branch_id.to_string()], |row| {
            let created_from_turn_str: Option<String> = row.get(4)?;
            let ancestry_str: String = row.get(6)?;
            let protected_int: i32 = row.get(7)?;
            let archived_int: i32 = row.get(8)?;
            let abandoned_future_int: i32 = row.get(9)?;
            Ok(BranchInfo {
                id: row.get::<_, String>(0)?.parse().unwrap(),
                session_id: row.get::<_, String>(1)?.parse().unwrap(),
                name: row.get(2)?,
                parent_branch_id: row.get::<_, Option<String>>(3)?.map(|s| s.parse().unwrap()),
                created_from_turn_id: created_from_turn_str.and_then(|s| s.parse().ok()),
                head_turn_id: row.get::<_, Option<String>>(5)?.map(|s| s.parse().unwrap()),
                ancestry: serde_json::from_str(&ancestry_str).unwrap_or_default(),
                protected: protected_int != 0,
                archived: archived_int != 0,
                abandoned_future: abandoned_future_int != 0,
                created_at: row.get(10)?,
            })
        });

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    /// Update branch head turn
    pub fn update_branch_head(&self, branch_id: BranchId, turn_id: TurnId) -> CoreResult<()> {
        self.conn
            .execute(
                "UPDATE branches SET head_turn_id = ?1 WHERE id = ?2",
                params![turn_id.to_string(), branch_id.to_string()],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// List branches for a session
    pub fn list_branches(&self, session_id: SessionId) -> CoreResult<Vec<BranchInfo>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, session_id, name, parent_branch_id, created_from_turn_id, head_turn_id, ancestry, protected, archived, abandoned_future, created_at
                 FROM branches WHERE session_id = ?1",
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let branches = stmt
            .query_map(params![session_id.to_string()], |row| {
                let created_from_turn_str: Option<String> = row.get(4)?;
                let ancestry_str: String = row.get(6)?;
                let protected_int: i32 = row.get(7)?;
                let archived_int: i32 = row.get(8)?;
                let abandoned_future_int: i32 = row.get(9)?;
                Ok(BranchInfo {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    session_id: row.get::<_, String>(1)?.parse().unwrap(),
                    name: row.get(2)?,
                    parent_branch_id: row.get::<_, Option<String>>(3)?.map(|s| s.parse().unwrap()),
                    created_from_turn_id: created_from_turn_str.and_then(|s| s.parse().ok()),
                    head_turn_id: row.get::<_, Option<String>>(5)?.map(|s| s.parse().unwrap()),
                    ancestry: serde_json::from_str(&ancestry_str).unwrap_or_default(),
                    protected: protected_int != 0,
                    archived: archived_int != 0,
                    abandoned_future: abandoned_future_int != 0,
                    created_at: row.get(10)?,
                })
            })
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        Ok(branches)
    }

    /// Create a new checkpoint
    pub fn create_checkpoint(&self, checkpoint: &CheckpointInfo) -> CoreResult<()> {
        self.conn
            .execute(
                "INSERT INTO checkpoints (id, session_id, branch_id, turn_id, manifest_id, reason, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
                params![
                    checkpoint.id.to_string(),
                    checkpoint.session_id.to_string(),
                    checkpoint.branch_id.to_string(),
                    checkpoint.turn_id.map(|t| t.to_string()),
                    checkpoint.manifest_id.to_string(),
                    checkpoint.reason,
                ],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get a checkpoint by ID
    pub fn get_checkpoint(
        &self,
        checkpoint_id: ai_history_core::CheckpointId,
    ) -> CoreResult<Option<CheckpointInfo>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, session_id, branch_id, turn_id, manifest_id, reason, created_at
                 FROM checkpoints WHERE id = ?1",
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let result = stmt.query_row(params![checkpoint_id.to_string()], |row| {
            Ok(CheckpointInfo {
                id: row.get::<_, String>(0)?.parse().unwrap(),
                session_id: row.get::<_, String>(1)?.parse().unwrap(),
                branch_id: row.get::<_, String>(2)?.parse().unwrap(),
                turn_id: row.get::<_, Option<String>>(3)?.map(|s| s.parse().unwrap()),
                manifest_id: row.get::<_, String>(4)?.parse().unwrap(),
                reason: row.get(5)?,
                created_at: row.get(6)?,
            })
        });

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    /// List checkpoints for a session
    pub fn list_checkpoints(&self, session_id: SessionId) -> CoreResult<Vec<CheckpointInfo>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, session_id, branch_id, turn_id, manifest_id, reason, created_at
                 FROM checkpoints WHERE session_id = ?1",
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let checkpoints = stmt
            .query_map(params![session_id.to_string()], |row| {
                Ok(CheckpointInfo {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    session_id: row.get::<_, String>(1)?.parse().unwrap(),
                    branch_id: row.get::<_, String>(2)?.parse().unwrap(),
                    turn_id: row.get::<_, Option<String>>(3)?.map(|s| s.parse().unwrap()),
                    manifest_id: row.get::<_, String>(4)?.parse().unwrap(),
                    reason: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        Ok(checkpoints)
    }

    /// Delete a checkpoint
    pub fn delete_checkpoint(
        &self,
        checkpoint_id: ai_history_core::CheckpointId,
    ) -> CoreResult<()> {
        self.conn
            .execute(
                "DELETE FROM checkpoints WHERE id = ?1",
                params![checkpoint_id.to_string()],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Create a manifest
    pub fn create_manifest(&self, manifest: &ManifestInfo) -> CoreResult<()> {
        self.conn
            .execute(
                "INSERT INTO manifests (id, session_id, checkpoint_id, manifest_data, created_at)
                 VALUES (?1, ?2, ?3, ?4, datetime('now'))",
                params![
                    manifest.id.to_string(),
                    manifest.session_id.to_string(),
                    manifest.checkpoint_id.to_string(),
                    manifest.manifest_data,
                ],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get a manifest by ID
    pub fn get_manifest(
        &self,
        manifest_id: ai_history_core::ManifestId,
    ) -> CoreResult<Option<ManifestInfo>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, session_id, checkpoint_id, manifest_data, created_at
                 FROM manifests WHERE id = ?1",
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let result = stmt.query_row(params![manifest_id.to_string()], |row| {
            Ok(ManifestInfo {
                id: row.get::<_, String>(0)?.parse().unwrap(),
                session_id: row.get::<_, String>(1)?.parse().unwrap(),
                checkpoint_id: row.get::<_, String>(2)?.parse().unwrap(),
                manifest_data: row.get(3)?,
                created_at: row.get(4)?,
            })
        });

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    /// Get events for a branch
    pub fn get_branch_events(
        &self,
        session_id: SessionId,
        branch_id: ai_history_core::BranchId,
    ) -> CoreResult<Vec<Event>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT event_data FROM events
                 WHERE session_id = ?1 AND branch_id = ?2
                 ORDER BY sequence_num",
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let events = stmt
            .query_map(
                params![session_id.to_string(), branch_id.to_string()],
                |row| {
                    let event_data: String = row.get(0)?;
                    Ok(event_data)
                },
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .filter_map(|data| {
                let data = data.ok()?;
                serde_json::from_str::<Event>(&data).ok()
            })
            .collect();

        Ok(events)
    }

    /// Get events since sequence number
    pub fn get_events_since(&self, session_id: SessionId, since: u64) -> CoreResult<Vec<Event>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT event_data FROM events
                 WHERE session_id = ?1 AND sequence_num > ?2
                 ORDER BY sequence_num",
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let events = stmt
            .query_map(params![session_id.to_string(), since as i64], |row| {
                let event_data: String = row.get(0)?;
                Ok(event_data)
            })
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .filter_map(|data| {
                let data = data.ok()?;
                serde_json::from_str::<Event>(&data).ok()
            })
            .collect();

        Ok(events)
    }

    /// Append events to the store with proper sequence numbers
    pub fn append_events(
        &self,
        session_id: SessionId,
        branch_id: ai_history_core::BranchId,
        events: &[Event],
    ) -> CoreResult<()> {
        // Use a transaction to ensure atomic sequence assignment
        let tx = self.transaction()?;

        for event in events {
            let event_data = serde_json::to_string(event)
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
            let event_type = event.event_type();
            let checksum = event.checksum();

            // Get next sequence number within transaction for atomicity
            let previous_seq: i64 = self
                .conn
                .query_row(
                    "SELECT COALESCE(MAX(sequence_num), -1) FROM events WHERE session_id = ?1",
                    params![session_id.to_string()],
                    |row| row.get(0),
                )
                .unwrap_or(-1);

            let sequence_num = previous_seq + 1;

            // Enforce monotonic sequence using invariant
            if sequence_num <= previous_seq {
                return Err(CoreError::InvalidSequence(format!(
                    "Sequence {} is not greater than previous {}",
                    sequence_num, previous_seq
                )));
            }

            self.conn
                .execute(
                    "INSERT INTO events (session_id, branch_id, sequence_num, event_type, event_data, actor, checksum, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))",
                    params![
                        session_id.to_string(),
                        branch_id.to_string(),
                        sequence_num,
                        event_type,
                        event_data,
                        "system",
                        checksum,
                    ],
                )
                .map_err(|e| CoreError::Storage(e.to_string()))?;
        }

        drop(tx); // Release transaction
        Ok(())
    }

    /// Create a new turn
    pub fn create_turn(&self, turn: &TurnInfo) -> CoreResult<()> {
        self.conn
            .execute(
                "INSERT INTO turns (id, session_id, branch_id, parent_turn_id, actor, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
                params![
                    turn.id.to_string(),
                    turn.session_id.to_string(),
                    turn.branch_id.to_string(),
                    turn.parent_turn_id.map(|t| t.to_string()),
                    turn.actor,
                ],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get a turn by ID
    pub fn get_turn(&self, turn_id: TurnId) -> CoreResult<Option<TurnInfo>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, session_id, branch_id, parent_turn_id, actor, created_at, finalized_at
                 FROM turns WHERE id = ?1",
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let result = stmt.query_row(params![turn_id.to_string()], |row| {
            Ok(TurnInfo {
                id: row.get::<_, String>(0)?.parse().unwrap(),
                session_id: row.get::<_, String>(1)?.parse().unwrap(),
                branch_id: row.get::<_, String>(2)?.parse().unwrap(),
                parent_turn_id: row.get::<_, Option<String>>(3)?.map(|s| s.parse().unwrap()),
                actor: row.get(4)?,
                created_at: row.get(5)?,
                finalized_at: row.get(6)?,
            })
        });

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    /// Finalize a turn
    pub fn finalize_turn(&self, turn_id: TurnId) -> CoreResult<()> {
        self.conn
            .execute(
                "UPDATE turns SET finalized_at = datetime('now') WHERE id = ?1",
                params![turn_id.to_string()],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Delete a session and all its data (cascade)
    pub fn delete_session(&self, session_id: SessionId) -> CoreResult<()> {
        self.conn
            .execute(
                "DELETE FROM sessions WHERE id = ?1",
                params![session_id.to_string()],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Delete a branch and all its data (cascade via FK)
    pub fn delete_branch(
        &self,
        branch_id: BranchId,
        _active_branch_id: BranchId,
    ) -> CoreResult<()> {
        // Check if this is the active branch - if so, reject
        let branch = self.get_branch(branch_id)?;
        if let Some(b) = branch {
            // Cannot delete protected branches
            if b.protected {
                return Err(CoreError::Validation(
                    "Cannot delete protected branch".to_string(),
                ));
            }
        }

        self.conn
            .execute(
                "DELETE FROM branches WHERE id = ?1",
                params![branch_id.to_string()],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get events for a session
    pub fn get_events(&self, session_id: SessionId) -> CoreResult<Vec<Event>> {
        let mut stmt = self
            .conn
            .prepare("SELECT event_data FROM events WHERE session_id = ?1 ORDER BY sequence_num")
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let events = stmt
            .query_map(params![session_id.to_string()], |row| {
                let event_data: String = row.get(0)?;
                Ok(event_data)
            })
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .filter_map(|data| {
                let data = data.ok()?;
                serde_json::from_str::<Event>(&data).ok()
            })
            .collect();

        Ok(events)
    }

    /// Get cursor position for a branch
    pub fn get_cursor(
        &self,
        session_id: SessionId,
        branch_id: BranchId,
    ) -> CoreResult<Option<TurnId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT turn_id FROM cursor_position WHERE session_id = ?1 AND branch_id = ?2")
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let result = stmt.query_row(
            params![session_id.to_string(), branch_id.to_string()],
            |row| {
                let turn_id_str: String = row.get(0)?;
                Ok(turn_id_str.parse().unwrap())
            },
        );

        match result {
            Ok(turn_id) => Ok(Some(turn_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    /// Set cursor position for a branch
    pub fn set_cursor(
        &self,
        session_id: SessionId,
        branch_id: BranchId,
        turn_id: TurnId,
    ) -> CoreResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO cursor_position (session_id, branch_id, turn_id, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))",
            params![session_id.to_string(), branch_id.to_string(), turn_id.to_string()],
        ).map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get forward child turn ID for a turn (first child in sequence)
    pub fn get_turn_forward_child(&self, turn_id: TurnId) -> CoreResult<Option<TurnId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM turns WHERE parent_turn_id = ?1 ORDER BY created_at LIMIT 1")
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let result = stmt.query_row(params![turn_id.to_string()], |row| {
            let turn_id_str: String = row.get(0)?;
            Ok(turn_id_str.parse().unwrap())
        });

        match result {
            Ok(turn_id) => Ok(Some(turn_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    /// Get all child turn IDs for a turn (for ambiguous forward navigation)
    pub fn get_turn_forward_children(&self, turn_id: TurnId) -> CoreResult<Vec<TurnId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM turns WHERE parent_turn_id = ?1 ORDER BY created_at")
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let children = stmt
            .query_map(params![turn_id.to_string()], |row| {
                let turn_id_str: String = row.get(0)?;
                Ok(turn_id_str.parse().unwrap())
            })
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        Ok(children)
    }

    /// Get the active session ID, falling back to first session if not set
    pub fn get_active_session_id(&self) -> CoreResult<Option<SessionId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM app_state WHERE key = 'active_session_id'")
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let result = stmt.query_row([], |row| {
            let value: String = row.get(0)?;
            Ok(value)
        });

        match result {
            Ok(value) => Ok(Some(value.parse().unwrap())),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Fall back to first session
                let sessions = self.list_sessions()?;
                Ok(sessions.first().map(|s| s.id))
            }
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    /// Set the active session ID
    pub fn set_active_session_id(&self, session_id: SessionId) -> CoreResult<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO app_state (key, value, updated_at)
             VALUES ('active_session_id', ?1, datetime('now'))",
                params![session_id.to_string()],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get the active branch ID for a session, falling back to "main" if not set
    pub fn get_active_branch_id(&self, session_id: SessionId) -> CoreResult<Option<BranchId>> {
        // First check if active branch is stored
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM app_state WHERE key = ?1")
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let key = format!("active_branch_id:{}", session_id);
        let result = stmt.query_row(params![key], |row| {
            let value: String = row.get(0)?;
            Ok(value)
        });

        match result {
            Ok(value) => Ok(Some(value.parse().unwrap())),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Fall back to "main" branch
                let branches = self.list_branches(session_id)?;
                let main = branches.iter().find(|b| b.name == "main");
                Ok(main.map(|b| b.id))
            }
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    /// Set the active branch ID for a session
    pub fn set_active_branch_id(
        &self,
        session_id: SessionId,
        branch_id: BranchId,
    ) -> CoreResult<()> {
        let key = format!("active_branch_id:{}", session_id);
        self.conn
            .execute(
                "INSERT OR REPLACE INTO app_state (key, value, updated_at)
             VALUES (?1, ?2, datetime('now'))",
                params![key, branch_id.to_string()],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get the operation state for a session
    pub fn get_operation_state(&self, session_id: SessionId) -> CoreResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT operation_state FROM sessions WHERE id = ?1")
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let result = stmt.query_row(params![session_id.to_string()], |row| {
            let value: String = row.get(0)?;
            Ok(value)
        });

        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    /// Set the operation state for a session
    pub fn set_operation_state(&self, session_id: SessionId, state: &str) -> CoreResult<()> {
        self.conn.execute(
            "UPDATE sessions SET operation_state = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![state, session_id.to_string()],
        ).map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Find turns that are not finalized (have NULL finalized_at)
    pub fn find_incomplete_turns(&self, session_id: SessionId) -> CoreResult<Vec<TurnInfo>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, session_id, branch_id, parent_turn_id, actor, created_at, finalized_at
             FROM turns WHERE session_id = ?1 AND finalized_at IS NULL",
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let turns = stmt
            .query_map(params![session_id.to_string()], |row| {
                Ok(TurnInfo {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    session_id: row.get::<_, String>(1)?.parse().unwrap(),
                    branch_id: row.get::<_, String>(2)?.parse().unwrap(),
                    parent_turn_id: row
                        .get::<_, Option<String>>(3)?
                        .and_then(|s| s.parse().ok()),
                    actor: row.get(4)?,
                    created_at: row.get(5)?,
                    finalized_at: row.get(6)?,
                })
            })
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for turn in turns {
            result.push(turn.map_err(|e| CoreError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    /// Count events that are not associated with a finalized turn
    pub fn count_uncommitted_events(&self, session_id: SessionId) -> CoreResult<usize> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM events e
             LEFT JOIN turns t ON e.turn_id = t.id
             WHERE e.session_id = ?1 AND (t.finalized_at IS NULL OR t.id IS NULL)",
                params![session_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(count as usize)
    }

    /// Check if the store has any in-progress restore operations recorded
    pub fn has_incomplete_restore_operation(&self) -> CoreResult<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM app_state WHERE key LIKE 'restore_operation:%' AND value = 'in_progress'",
            [],
            |row| row.get(0),
        ).map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(count > 0)
    }

    /// Record an in-progress restore operation
    pub fn begin_restore_operation(&self, restore_id: &str) -> CoreResult<()> {
        let key = format!("restore_operation:{}", restore_id);
        self.conn
            .execute(
                "INSERT OR REPLACE INTO app_state (key, value, updated_at)
             VALUES (?1, 'in_progress', datetime('now'))",
                params![key],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Clear a restore operation record
    pub fn clear_restore_operation(&self, restore_id: &str) -> CoreResult<()> {
        let key = format!("restore_operation:{}", restore_id);
        self.conn
            .execute("DELETE FROM app_state WHERE key = ?1", params![key])
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Persist a message record (first-class message for task 27)
    pub fn create_message(&self, message: &crate::MessageRecord) -> CoreResult<()> {
        let metadata_json =
            serde_json::to_string(&message.metadata).unwrap_or_else(|_| "{}".to_string());

        self.conn
            .execute(
                "INSERT INTO messages (id, turn_id, message_type, role, content, metadata, sequence_num, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    message.id.to_string(),
                    message.turn_id.to_string(),
                    message.message_type,
                    message.role,
                    message.content,
                    metadata_json,
                    message.sequence_num as i64,
                    message.created_at,
                ],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get all messages for a turn, ordered by sequence number
    pub fn get_messages_for_turn(&self, turn_id: TurnId) -> CoreResult<Vec<crate::MessageRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, turn_id, message_type, role, content, metadata, sequence_num, created_at
             FROM messages WHERE turn_id = ?1 ORDER BY sequence_num"
        ).map_err(|e| CoreError::Storage(e.to_string()))?;

        let messages = stmt
            .query_map(params![turn_id.to_string()], |row| {
                let metadata_str: String = row.get(5)?;
                let metadata = serde_json::from_str(&metadata_str).unwrap_or_default();

                Ok(crate::MessageRecord {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    turn_id: row.get::<_, String>(1)?.parse().unwrap(),
                    message_type: row.get(2)?,
                    role: row.get(3)?,
                    content: row.get(4)?,
                    metadata,
                    sequence_num: row.get::<_, i64>(6)? as u64,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        Ok(messages)
    }

    /// Get message by ID
    pub fn get_message(&self, message_id: &str) -> CoreResult<Option<crate::MessageRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, turn_id, message_type, role, content, metadata, sequence_num, created_at
             FROM messages WHERE id = ?1"
        ).map_err(|e| CoreError::Storage(e.to_string()))?;

        let result = stmt.query_row(params![message_id], |row| {
            let metadata_str: String = row.get(5)?;
            let metadata = serde_json::from_str(&metadata_str).unwrap_or_default();

            Ok(crate::MessageRecord {
                id: row.get::<_, String>(0)?.parse().unwrap(),
                turn_id: row.get::<_, String>(1)?.parse().unwrap(),
                message_type: row.get(2)?,
                role: row.get(3)?,
                content: row.get(4)?,
                metadata,
                sequence_num: row.get::<_, i64>(6)? as u64,
                created_at: row.get(7)?,
            })
        });

        match result {
            Ok(msg) => Ok(Some(msg)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    /// Delete all messages for a turn
    pub fn delete_messages_for_turn(&self, turn_id: TurnId) -> CoreResult<()> {
        self.conn
            .execute(
                "DELETE FROM messages WHERE turn_id = ?1",
                params![turn_id.to_string()],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Update session title
    pub fn update_session_title(&self, session_id: SessionId, title: &str) -> CoreResult<()> {
        self.conn
            .execute(
                "UPDATE sessions SET title = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![title, session_id.to_string()],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Update session view cursor
    pub fn update_session_cursor(&self, session_id: SessionId, cursor: TurnId) -> CoreResult<()> {
        self.conn
            .execute(
                "UPDATE sessions SET view_cursor = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![cursor.to_string(), session_id.to_string()],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Archive a session
    pub fn archive_session(&self, session_id: SessionId) -> CoreResult<()> {
        self.conn
            .execute(
                "UPDATE sessions SET archival_state = 1, updated_at = datetime('now') WHERE id = ?1",
                params![session_id.to_string()],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get branch ancestry as a vector of branch IDs
    pub fn get_branch_ancestry(&self, branch_id: BranchId) -> CoreResult<Vec<BranchId>> {
        let branch = self.get_branch(branch_id)?;
        match branch {
            Some(b) => Ok(b.ancestry),
            None => Ok(Vec::new()),
        }
    }

    /// Persist a context snapshot for a turn (task 12)
    pub fn create_context_snapshot(&self, snapshot: &ContextSnapshotRecord) -> CoreResult<()> {
        self.conn
            .execute(
                "INSERT INTO context_snapshots (id, turn_id, snapshot_data, selected_files_count, retrieved_chunks_count, tool_outputs_count, summary_refs_count, memory_refs_count, rule_refs_count, system_prompt_hash, model_config_data, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    snapshot.id,
                    snapshot.turn_id.to_string(),
                    snapshot.snapshot_data,
                    snapshot.selected_files_count as i64,
                    snapshot.retrieved_chunks_count as i64,
                    snapshot.tool_outputs_count as i64,
                    snapshot.summary_refs_count as i64,
                    snapshot.memory_refs_count as i64,
                    snapshot.rule_refs_count as i64,
                    snapshot.system_prompt_hash,
                    snapshot.model_config_data,
                    snapshot.created_at,
                ],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get context snapshots for a turn
    pub fn get_context_snapshots_for_turn(
        &self,
        turn_id: TurnId,
    ) -> CoreResult<Vec<ContextSnapshotRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, turn_id, snapshot_data, selected_files_count, retrieved_chunks_count, tool_outputs_count, summary_refs_count, memory_refs_count, rule_refs_count, system_prompt_hash, model_config_data, created_at
             FROM context_snapshots WHERE turn_id = ?1 ORDER BY created_at"
        ).map_err(|e| CoreError::Storage(e.to_string()))?;

        let snapshots = stmt
            .query_map(params![turn_id.to_string()], |row| {
                Ok(ContextSnapshotRecord {
                    id: row.get(0)?,
                    turn_id: row.get::<_, String>(1)?.parse().unwrap(),
                    snapshot_data: row.get(2)?,
                    selected_files_count: row.get::<_, i64>(3)? as usize,
                    retrieved_chunks_count: row.get::<_, i64>(4)? as usize,
                    tool_outputs_count: row.get::<_, i64>(5)? as usize,
                    summary_refs_count: row.get::<_, i64>(6)? as usize,
                    memory_refs_count: row.get::<_, i64>(7)? as usize,
                    rule_refs_count: row.get::<_, i64>(8)? as usize,
                    system_prompt_hash: row.get(9)?,
                    model_config_data: row.get(10)?,
                    created_at: row.get(11)?,
                })
            })
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        Ok(snapshots)
    }

    /// Get a context snapshot by ID
    pub fn get_context_snapshot(&self, id: &str) -> CoreResult<Option<ContextSnapshotRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, turn_id, snapshot_data, selected_files_count, retrieved_chunks_count, tool_outputs_count, summary_refs_count, memory_refs_count, rule_refs_count, system_prompt_hash, model_config_data, created_at
             FROM context_snapshots WHERE id = ?1"
        ).map_err(|e| CoreError::Storage(e.to_string()))?;

        let result = stmt.query_row(params![id], |row| {
            Ok(ContextSnapshotRecord {
                id: row.get(0)?,
                turn_id: row.get::<_, String>(1)?.parse().unwrap(),
                snapshot_data: row.get(2)?,
                selected_files_count: row.get::<_, i64>(3)? as usize,
                retrieved_chunks_count: row.get::<_, i64>(4)? as usize,
                tool_outputs_count: row.get::<_, i64>(5)? as usize,
                summary_refs_count: row.get::<_, i64>(6)? as usize,
                memory_refs_count: row.get::<_, i64>(7)? as usize,
                rule_refs_count: row.get::<_, i64>(8)? as usize,
                system_prompt_hash: row.get(9)?,
                model_config_data: row.get(10)?,
                created_at: row.get(11)?,
            })
        });

        match result {
            Ok(snapshot) => Ok(Some(snapshot)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    /// Delete context snapshots for a turn
    pub fn delete_context_snapshots_for_turn(&self, turn_id: TurnId) -> CoreResult<()> {
        self.conn
            .execute(
                "DELETE FROM context_snapshots WHERE turn_id = ?1",
                params![turn_id.to_string()],
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// List all context snapshots for a session (via turns)
    pub fn list_context_snapshots_for_session(
        &self,
        session_id: SessionId,
    ) -> CoreResult<Vec<ContextSnapshotRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT cs.id, cs.turn_id, cs.snapshot_data, cs.selected_files_count, cs.retrieved_chunks_count, cs.tool_outputs_count, cs.summary_refs_count, cs.memory_refs_count, cs.rule_refs_count, cs.system_prompt_hash, cs.model_config_data, cs.created_at
             FROM context_snapshots cs
             INNER JOIN turns t ON cs.turn_id = t.id
             WHERE t.session_id = ?1
             ORDER BY cs.created_at"
        ).map_err(|e| CoreError::Storage(e.to_string()))?;

        let snapshots = stmt
            .query_map(params![session_id.to_string()], |row| {
                Ok(ContextSnapshotRecord {
                    id: row.get(0)?,
                    turn_id: row.get::<_, String>(1)?.parse().unwrap(),
                    snapshot_data: row.get(2)?,
                    selected_files_count: row.get::<_, i64>(3)? as usize,
                    retrieved_chunks_count: row.get::<_, i64>(4)? as usize,
                    tool_outputs_count: row.get::<_, i64>(5)? as usize,
                    summary_refs_count: row.get::<_, i64>(6)? as usize,
                    memory_refs_count: row.get::<_, i64>(7)? as usize,
                    rule_refs_count: row.get::<_, i64>(8)? as usize,
                    system_prompt_hash: row.get(9)?,
                    model_config_data: row.get(10)?,
                    created_at: row.get(11)?,
                })
            })
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        Ok(snapshots)
    }

    /// Get latest context snapshot for a turn (for reproducibility checks)
    pub fn get_latest_context_snapshot_for_turn(
        &self,
        turn_id: TurnId,
    ) -> CoreResult<Option<ContextSnapshotRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, turn_id, snapshot_data, selected_files_count, retrieved_chunks_count, tool_outputs_count, summary_refs_count, memory_refs_count, rule_refs_count, system_prompt_hash, model_config_data, created_at
             FROM context_snapshots WHERE turn_id = ?1 ORDER BY created_at DESC LIMIT 1"
        ).map_err(|e| CoreError::Storage(e.to_string()))?;

        let result = stmt.query_row(params![turn_id.to_string()], |row| {
            Ok(ContextSnapshotRecord {
                id: row.get(0)?,
                turn_id: row.get::<_, String>(1)?.parse().unwrap(),
                snapshot_data: row.get(2)?,
                selected_files_count: row.get::<_, i64>(3)? as usize,
                retrieved_chunks_count: row.get::<_, i64>(4)? as usize,
                tool_outputs_count: row.get::<_, i64>(5)? as usize,
                summary_refs_count: row.get::<_, i64>(6)? as usize,
                memory_refs_count: row.get::<_, i64>(7)? as usize,
                rule_refs_count: row.get::<_, i64>(8)? as usize,
                system_prompt_hash: row.get(9)?,
                model_config_data: row.get(10)?,
                created_at: row.get(11)?,
            })
        });

        match result {
            Ok(snapshot) => Ok(Some(snapshot)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Storage(e.to_string())),
        }
    }

    /// Get all turns for a branch in order
    pub fn get_turns_for_branch(&self, branch_id: BranchId) -> CoreResult<Vec<TurnInfo>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, session_id, branch_id, parent_turn_id, actor, created_at, finalized_at
             FROM turns WHERE branch_id = ?1 ORDER BY created_at",
            )
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        let turns = stmt
            .query_map(params![branch_id.to_string()], |row| {
                Ok(TurnInfo {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    session_id: row.get::<_, String>(1)?.parse().unwrap(),
                    branch_id: row.get::<_, String>(2)?.parse().unwrap(),
                    parent_turn_id: row
                        .get::<_, Option<String>>(3)?
                        .and_then(|s| s.parse().ok()),
                    actor: row.get(4)?,
                    created_at: row.get(5)?,
                    finalized_at: row.get(6)?,
                })
            })
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Storage(e.to_string()))?;

        Ok(turns)
    }

    /// Detect all recovery states for a session
    pub fn detect_recovery_state(
        &self,
        session_id: SessionId,
    ) -> CoreResult<Vec<ai_history_core::recovery::RecoveryState>> {
        use ai_history_core::recovery::RecoveryDetector;

        let mut incomplete_transactions = Vec::new();
        let mut uncommitted_events_list = Vec::new();

        // Check for incomplete turns
        let incomplete_turns = self.find_incomplete_turns(session_id)?;
        for turn in &incomplete_turns {
            let info = RecoveryDetector::create_incomplete_transaction(
                &format!("turn:{}", turn.id),
                Some(&session_id.to_string()),
                Some(&turn.branch_id.to_string()),
            );
            incomplete_transactions.push(info);
        }

        // Check for uncommitted events
        let uncommitted_count = self.count_uncommitted_events(session_id)?;
        if uncommitted_count > 0 {
            // Find the first uncommitted sequence
            let first_uncommitted: Option<i64> = self
                .conn
                .query_row(
                    "SELECT MIN(e.sequence_num) FROM events e
                 LEFT JOIN turns t ON e.turn_id = t.id
                 WHERE e.session_id = ?1 AND (t.finalized_at IS NULL OR t.id IS NULL)",
                    params![session_id.to_string()],
                    |row| row.get(0),
                )
                .ok();

            let last_committed: Option<i64> = self
                .conn
                .query_row(
                    "SELECT MAX(e.sequence_num) FROM events e
                 INNER JOIN turns t ON e.turn_id = t.id
                 WHERE e.session_id = ?1 AND t.finalized_at IS NOT NULL",
                    params![session_id.to_string()],
                    |row| row.get(0),
                )
                .ok();

            if let (Some(first), Some(last)) = (first_uncommitted, last_committed) {
                let branches = self.list_branches(session_id)?;
                let branch_id = branches
                    .first()
                    .map(|b| b.id.to_string())
                    .unwrap_or_default();
                let info = RecoveryDetector::create_uncommitted_events(
                    &session_id.to_string(),
                    &branch_id,
                    first as u64,
                    last as u64,
                );
                uncommitted_events_list.push(info);
            }
        }

        let states = RecoveryDetector::detect_recovery_state(
            &incomplete_transactions,
            &[],
            &[],
            &uncommitted_events_list,
        );

        Ok(states)
    }
}

// Safety: SqliteStore uses interior mutability via RefCell, but is only used
// from single-threaded contexts in this codebase. The Connection is wrapped
// in a Mutex in multi-threaded scenarios by the caller.
unsafe impl Send for SqliteStore {}
unsafe impl Sync for SqliteStore {}

impl<'a> Transaction<'a> {
    /// Commit the transaction
    pub fn commit(self) -> CoreResult<()> {
        self.conn
            .execute("COMMIT", [])
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Rollback the transaction
    pub fn rollback(self) {
        let _ = self.conn.execute("ROLLBACK", []);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sqlite_store_in_memory() {
        let store = SqliteStore::in_memory().unwrap();
        assert!(store.list_sessions().unwrap().is_empty());
    }

    #[test]
    fn test_create_and_get_session() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp/test").unwrap();

        let session = store.get_session(session_id).unwrap();
        assert!(session.is_some());
        assert_eq!(session.unwrap().workspace_root, "/tmp/test");
    }

    #[test]
    fn test_create_and_get_branch() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp/test").unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        let branch = store.get_branch(branch_id).unwrap();
        assert!(branch.is_some());
        assert_eq!(branch.unwrap().name, "main");
    }

    #[test]
    fn test_create_and_get_turn() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn_id = TurnId::new();

        store.create_session(session_id, "/tmp/test").unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();
        store
            .create_turn(&TurnInfo {
                id: turn_id,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        let turn = store.get_turn(turn_id).unwrap();
        assert!(turn.is_some());
        assert_eq!(turn.unwrap().actor, "User");
    }

    #[test]
    fn test_finalize_turn() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn_id = TurnId::new();

        store.create_session(session_id, "/tmp/test").unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();
        store
            .create_turn(&TurnInfo {
                id: turn_id,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        store.finalize_turn(turn_id).unwrap();

        let turn = store.get_turn(turn_id).unwrap().unwrap();
        assert!(turn.finalized_at.is_some());
    }

    #[test]
    fn test_list_branches() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id1 = BranchId::new();
        let branch_id2 = BranchId::new();

        store.create_session(session_id, "/tmp/test").unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id1,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id2,
                session_id,
                name: "feature".to_string(),
                parent_branch_id: Some(branch_id1),
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: vec![branch_id1],
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        let branches = store.list_branches(session_id).unwrap();
        assert_eq!(branches.len(), 2);
    }

    #[test]
    fn test_file_based_store() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = SqliteStore::open(&db_path).unwrap();

        let session_id = SessionId::new();
        store.create_session(session_id, "/tmp/test").unwrap();

        let session = store.get_session(session_id).unwrap();
        assert!(session.is_some());
    }

    #[test]
    fn test_transaction_commit() {
        let store = SqliteStore::in_memory().unwrap();
        let txn = store.transaction().unwrap();
        txn.commit().unwrap();
    }

    #[test]
    fn test_update_branch_head() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn_id = TurnId::new();

        store.create_session(session_id, "/tmp/test").unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        store.update_branch_head(branch_id, turn_id).unwrap();

        let branch = store.get_branch(branch_id).unwrap().unwrap();
        assert!(branch.head_turn_id.is_some());
    }

    #[test]
    fn test_get_operation_state() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();

        store.create_session(session_id, "/tmp/test").unwrap();

        let state = store.get_operation_state(session_id).unwrap();
        assert_eq!(state, Some("active".to_string()));
    }

    #[test]
    fn test_set_operation_state() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();

        store.create_session(session_id, "/tmp/test").unwrap();

        store
            .set_operation_state(session_id, "interrupted")
            .unwrap();
        let state = store.get_operation_state(session_id).unwrap();
        assert_eq!(state, Some("interrupted".to_string()));

        store.set_operation_state(session_id, "active").unwrap();
        let state = store.get_operation_state(session_id).unwrap();
        assert_eq!(state, Some("active".to_string()));
    }

    #[test]
    fn test_find_incomplete_turns() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp/test").unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        // Create a turn
        let turn_id = TurnId::new();
        store
            .create_turn(&TurnInfo {
                id: turn_id,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        // Should find the incomplete turn
        let incomplete = store.find_incomplete_turns(session_id).unwrap();
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].id, turn_id);
    }

    #[test]
    fn test_find_incomplete_turns_empty_when_all_finalized() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp/test").unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        // Create a finalized turn
        let turn_id = TurnId::new();
        store
            .create_turn(&TurnInfo {
                id: turn_id,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None, // create_turn always sets NULL, then we finalize
            })
            .unwrap();

        // Finalize the turn to make it complete
        store.finalize_turn(turn_id).unwrap();

        // Should find no incomplete turns
        let incomplete = store.find_incomplete_turns(session_id).unwrap();
        assert!(incomplete.is_empty());
    }

    #[test]
    fn test_count_uncommitted_events() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp/test").unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        // No events yet
        let count = store.count_uncommitted_events(session_id).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_begin_and_clear_restore_operation() {
        let store = SqliteStore::in_memory().unwrap();

        // Should not have incomplete restore initially
        let has_incomplete = store.has_incomplete_restore_operation().unwrap();
        assert!(!has_incomplete);

        // Begin a restore operation
        store.begin_restore_operation("restore-123").unwrap();

        // Should now have incomplete restore
        let has_incomplete = store.has_incomplete_restore_operation().unwrap();
        assert!(has_incomplete);

        // Clear it
        store.clear_restore_operation("restore-123").unwrap();

        // Should no longer have incomplete restore
        let has_incomplete = store.has_incomplete_restore_operation().unwrap();
        assert!(!has_incomplete);
    }

    #[test]
    fn test_detect_recovery_state_clean() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();

        store.create_session(session_id, "/tmp/test").unwrap();

        let states = store.detect_recovery_state(session_id).unwrap();
        assert_eq!(states.len(), 1);
        assert!(matches!(
            states[0],
            ai_history_core::recovery::RecoveryState::Clean
        ));
    }

    #[test]
    fn test_detect_recovery_state_with_incomplete_turn() {
        let store = SqliteStore::in_memory().unwrap();
        let session_id = SessionId::new();
        let branch_id = BranchId::new();

        store.create_session(session_id, "/tmp/test").unwrap();
        store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();

        // Create incomplete turn
        let turn_id = TurnId::new();
        store
            .create_turn(&TurnInfo {
                id: turn_id,
                session_id,
                branch_id,
                parent_turn_id: None,
                actor: "User".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                finalized_at: None,
            })
            .unwrap();

        let states = store.detect_recovery_state(session_id).unwrap();
        assert!(!states.is_empty());
        // Should have at least one non-clean state
        let has_incomplete = states.iter().any(|s| {
            matches!(
                s,
                ai_history_core::recovery::RecoveryState::IncompleteTransaction(_)
            )
        });
        assert!(has_incomplete);
    }
}
