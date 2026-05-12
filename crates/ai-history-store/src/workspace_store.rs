use crate::sqlite_store::SqliteStore;
use crate::traits::{BlobInfo, BlobStore, CheckpointInfo, ManifestInfo};
use ai_history_core::gc::{BranchInfo as GcBranchInfo, CheckpointInfo as GcCheckpointInfo};
use ai_history_core::gc::{GcStore, ManifestInfo as GcManifestInfo};
use ai_history_core::{
    BlobId, BranchId, CheckpointId, CoreError, CoreResult, ManifestId, SessionId, TurnId,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[path = "workspace_store_support.rs"]
mod workspace_store_support;
use self::workspace_store_support::{
    collect_mapped_rows, extract_manifest_blob_hashes, is_older_than_grace,
    prune_empty_object_dirs, query_count, query_sum, storage_err,
};
const DEFAULT_MAX_TOTAL_BLOB_BYTES: u64 = 5 * 1024 * 1024 * 1024;
const DEFAULT_FRESH_WRITE_GRACE_HOURS: u64 = 24;
const EXTERNAL_BLOB_MODE_KEY: &str = "external_blob_mode";
const MAX_TOTAL_BLOB_BYTES_KEY: &str = "max_total_blob_bytes";
const FRESH_WRITE_GRACE_HOURS_KEY: &str = "fresh_write_grace_hours";
const OBJECTS_DIR: &str = "objects";
const HASH_ALGO_DIR: &str = "sha256";
#[derive(Debug, Clone, Default)]
pub struct BlobQuotaReport {
    pub object_count: usize,
    pub stored_bytes: u64,
    pub inline_blob_count: usize,
    pub inline_blob_bytes: u64,
    pub max_total_blob_bytes: u64,
}
impl BlobQuotaReport {
    pub fn over_quota(&self) -> bool {
        self.stored_bytes > self.max_total_blob_bytes
    }}
#[derive(Debug, Clone, Default)]
pub struct BlobMigrationReport {
    pub migrated_blobs: usize,
    pub migrated_bytes: u64,
    pub remaining_inline_blobs: usize,
    pub vacuum_performed: bool,
}
#[derive(Debug, Clone, Default)]
pub struct BlobGcReport {
    pub deleted_objects: usize,
    pub deleted_rows: usize,
    pub reclaimed_bytes: u64,
    pub missing_object_files: usize,
    pub protected_recent_objects: usize,
}
pub struct HistoryStore {
    inner: SqliteStore,
    db_path: PathBuf,
}
impl HistoryStore {
    pub fn open<P: AsRef<Path>>(path: P) -> CoreResult<Self> {
        let db_path = path.as_ref().to_path_buf();
        let inner = SqliteStore::open(&db_path)?;
        let store = Self { inner, db_path };
        store.run_external_blob_migrations()?;
        Ok(store)
    }}
impl HistoryStore {
    pub fn in_memory() -> CoreResult<Self> {
        let db_path = std::env::temp_dir().join(format!("ai-history-{}.db", uuid::Uuid::new_v4()));
        Self::open(db_path)
    }}
impl HistoryStore {
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }}
impl HistoryStore {
    pub fn object_store_dir(&self) -> PathBuf {
        self.db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(OBJECTS_DIR)
            .join(HASH_ALGO_DIR)
    }}
impl HistoryStore {
    pub fn uses_metadata_snapshots(&self) -> CoreResult<bool> {
        Ok(self.external_blob_mode_enabled()?)
    }}
impl HistoryStore {
    pub fn set_max_total_blob_bytes(&self, max_bytes: u64) -> CoreResult<()> {
        self.set_setting(MAX_TOTAL_BLOB_BYTES_KEY, max_bytes.to_string().as_str())
    }}
impl HistoryStore {
    pub fn migrate_all_legacy_inline_blobs(&self) -> CoreResult<BlobMigrationReport> {
        let mut summary = BlobMigrationReport::default();
        loop {
            let report = self.migrate_legacy_inline_blobs(512)?;
            summary.migrated_blobs += report.migrated_blobs;
            summary.migrated_bytes += report.migrated_bytes;
            summary.remaining_inline_blobs = report.remaining_inline_blobs;
            summary.vacuum_performed |= report.vacuum_performed;
            if report.remaining_inline_blobs == 0 || report.migrated_blobs == 0 {
                return Ok(summary);
            }
        }
    }}
impl HistoryStore {
    pub fn quota_report(&self) -> CoreResult<BlobQuotaReport> {
        let conn = self.metadata_connection()?;
        let object_count = query_count(&conn, "SELECT COUNT(*) FROM blob_objects")?;
        let stored_bytes = query_sum(
            &conn,
            "SELECT COALESCE(SUM(stored_size), 0) FROM blob_objects",
        )?;
        let inline_blob_count = query_count(
            &conn,
            "SELECT COUNT(*) FROM blobs WHERE length(content) > 0",
        )?;
        let inline_blob_bytes = query_sum(
            &conn,
            "SELECT COALESCE(SUM(length(content)), 0) FROM blobs WHERE length(content) > 0",
        )?;
        Ok(BlobQuotaReport {
            object_count,
            stored_bytes,
            inline_blob_count,
            inline_blob_bytes,
            max_total_blob_bytes: self.max_total_blob_bytes()?,
        })
    }}
impl HistoryStore {
    pub fn migrate_legacy_inline_blobs(&self, limit: usize) -> CoreResult<BlobMigrationReport> {
        self.rebuild_manifest_blob_refs()?;
        let conn = self.metadata_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT hash, content FROM blobs
                 WHERE length(content) > 0
                 ORDER BY created_at
                 LIMIT ?1",
            )
            .map_err(storage_err)?;
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let rows = collect_mapped_rows(
            stmt.query_map([limit], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })
            .map_err(storage_err)?,
        )?;
        drop(stmt);

        let mut report = BlobMigrationReport::default();
        for (hash_hex, contents) in rows {
            let stored_size = self.write_object_if_missing(&hash_hex, &contents)?;
            conn.execute(
                "INSERT OR REPLACE INTO blob_objects
                 (hash, codec, logical_size, stored_size, path, created_at, last_verified_at)
                 VALUES (?1, 'identity', ?2, ?3, ?4, datetime('now'), datetime('now'))",
                params![
                    hash_hex,
                    contents.len() as i64,
                    stored_size as i64,
                    self.object_path_for_hash(&hash_hex).display().to_string(),
                ],
            )
            .map_err(storage_err)?;
            conn.execute(
                "UPDATE blobs SET content = X'' WHERE hash = ?1",
                [hash_hex.as_str()],
            )
            .map_err(storage_err)?;
            report.migrated_blobs += 1;
            report.migrated_bytes += contents.len() as u64;
        }

        report.remaining_inline_blobs = query_count(
            &conn,
            "SELECT COUNT(*) FROM blobs WHERE length(content) > 0",
        )?;
        if report.migrated_blobs > 0 && report.remaining_inline_blobs == 0 {
            conn.execute_batch("VACUUM").map_err(storage_err)?;
            report.vacuum_performed = true;
        }
        Ok(report)
    }}
impl HistoryStore {
    pub fn garbage_collect_objects(&self, execute: bool) -> CoreResult<BlobGcReport> {
        self.rebuild_manifest_blob_refs()?;
        let conn = self.metadata_connection()?;
        let live_hashes = self.live_manifest_hashes(&conn)?;
        let grace = Duration::from_secs(self.fresh_write_grace_hours()? * 60 * 60);
        let mut stmt = conn
            .prepare(
                "SELECT hash, stored_size, path, created_at FROM blob_objects ORDER BY created_at",
            )
            .map_err(storage_err)?;
        let rows = collect_mapped_rows(
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(storage_err)?,
        )?;
        drop(stmt);

        let mut report = BlobGcReport::default();
        for (hash_hex, stored_size, path_str, created_at) in rows {
            if live_hashes.contains(&hash_hex) {
                continue;
            }
            let path = PathBuf::from(path_str);
            let old_enough = fs::metadata(&path)
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                .map(|age| age >= grace)
                .unwrap_or_else(|| is_older_than_grace(&created_at, grace));
            if !old_enough {
                report.protected_recent_objects += 1;
                continue;
            }
            if !path.is_file() {
                report.missing_object_files += 1;
            }
            if execute {
                if path.is_file() {
                    fs::remove_file(&path).map_err(|e| {
                        CoreError::Storage(format!("remove object {}: {}", path.display(), e))
                    })?;
                    prune_empty_object_dirs(&self.object_store_dir(), path.parent());
                    report.reclaimed_bytes += stored_size.max(0) as u64;
                    report.deleted_objects += 1;
                }
                conn.execute(
                    "DELETE FROM blob_objects WHERE hash = ?1",
                    [hash_hex.as_str()],
                )
                .map_err(storage_err)?;
                conn.execute(
                    "DELETE FROM blobs WHERE hash = ?1 AND length(content) = 0",
                    [hash_hex.as_str()],
                )
                .map_err(storage_err)?;
                conn.execute("DELETE FROM blob_refs WHERE hash = ?1", [hash_hex.as_str()])
                    .map_err(storage_err)?;
                report.deleted_rows += 1;
            }
        }
        Ok(report)
    }}
impl HistoryStore {
    pub fn create_manifest(&self, manifest: &ManifestInfo) -> CoreResult<()> {
        self.inner.create_manifest(manifest)?;
        self.record_manifest_blob_refs(manifest.id, &manifest.manifest_data)
    }}
impl HistoryStore {
    pub fn delete_manifest(&self, manifest_id: ManifestId) -> CoreResult<()> {
        let conn = self.metadata_connection()?;
        conn.execute(
            "DELETE FROM blob_refs WHERE owner_kind = 'manifest' AND owner_id = ?1",
            [manifest_id.to_string()],
        )
        .map_err(storage_err)?;
        self.inner.delete_manifest(&manifest_id)
    }}
impl HistoryStore {
    pub fn create_checkpoint(&self, checkpoint: &CheckpointInfo) -> CoreResult<()> {
        self.inner.create_checkpoint(checkpoint)
    }}
impl HistoryStore {
    pub fn delete_checkpoint(&self, checkpoint_id: CheckpointId) -> CoreResult<()> {
        let conn = self.metadata_connection()?;
        let manifest_ids = {
            let mut stmt = conn
                .prepare("SELECT id FROM manifests WHERE checkpoint_id = ?1")
                .map_err(storage_err)?;
            let manifest_ids = collect_mapped_rows(
                stmt.query_map([checkpoint_id.to_string()], |row| row.get::<_, String>(0))
                    .map_err(storage_err)?,
            )?;
            manifest_ids
        };
        for manifest_id in manifest_ids {
            self.delete_manifest(
                manifest_id
                    .parse()
                    .map_err(|_| CoreError::Storage("invalid manifest id".to_string()))?,
            )?;
        }
        self.inner.delete_checkpoint(checkpoint_id)
    }}
impl HistoryStore {
    fn run_external_blob_migrations(&self) -> CoreResult<()> {
        let conn = self.metadata_connection()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS blob_objects (
                hash TEXT PRIMARY KEY,
                codec TEXT NOT NULL,
                logical_size INTEGER NOT NULL,
                stored_size INTEGER NOT NULL,
                path TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_verified_at TEXT
             );
             CREATE TABLE IF NOT EXISTS blob_refs (
                owner_kind TEXT NOT NULL,
                owner_id TEXT NOT NULL,
                hash TEXT NOT NULL,
                ref_kind TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (owner_kind, owner_id, hash, ref_kind)
             );
             CREATE TABLE IF NOT EXISTS store_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_blob_refs_hash ON blob_refs(hash);
             CREATE INDEX IF NOT EXISTS idx_blob_refs_owner ON blob_refs(owner_kind, owner_id);
             DELETE FROM schema_version WHERE version <> 2;
             INSERT OR REPLACE INTO schema_version (version, applied_at)
             VALUES (2, datetime('now'));",
        )
        .map_err(storage_err)?;
        self.set_setting(EXTERNAL_BLOB_MODE_KEY, "1")?;
        if self.get_setting(MAX_TOTAL_BLOB_BYTES_KEY)?.is_none() {
            self.set_max_total_blob_bytes(DEFAULT_MAX_TOTAL_BLOB_BYTES)?;
        }
        if self.get_setting(FRESH_WRITE_GRACE_HOURS_KEY)?.is_none() {
            self.set_setting(
                FRESH_WRITE_GRACE_HOURS_KEY,
                DEFAULT_FRESH_WRITE_GRACE_HOURS.to_string().as_str(),
            )?;
        }
        Ok(())
    }}
impl HistoryStore {
    fn metadata_connection(&self) -> CoreResult<Connection> {
        let conn = Connection::open(&self.db_path).map_err(storage_err)?;
        conn.execute("PRAGMA foreign_keys = ON", [])
            .map_err(storage_err)?;
        Ok(conn)
    }}
impl HistoryStore {
    fn external_blob_mode_enabled(&self) -> CoreResult<bool> {
        Ok(self
            .get_setting(EXTERNAL_BLOB_MODE_KEY)?
            .as_deref()
            .unwrap_or("0")
            == "1")
    }}
impl HistoryStore {
    fn max_total_blob_bytes(&self) -> CoreResult<u64> {
        Ok(self
            .get_setting(MAX_TOTAL_BLOB_BYTES_KEY)?
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_MAX_TOTAL_BLOB_BYTES))
    }}
impl HistoryStore {
    fn fresh_write_grace_hours(&self) -> CoreResult<u64> {
        Ok(self
            .get_setting(FRESH_WRITE_GRACE_HOURS_KEY)?
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_FRESH_WRITE_GRACE_HOURS))
    }}
impl HistoryStore {
    fn get_setting(&self, key: &str) -> CoreResult<Option<String>> {
        let conn = self.metadata_connection()?;
        conn.query_row(
            "SELECT value FROM store_settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage_err)
    }}
impl HistoryStore {
    fn set_setting(&self, key: &str, value: &str) -> CoreResult<()> {
        let conn = self.metadata_connection()?;
        conn.execute(
            "INSERT INTO store_settings (key, value, updated_at)
             VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value],
        )
        .map_err(storage_err)?;
        Ok(())
    }}
impl HistoryStore {
    fn record_manifest_blob_refs(
        &self,
        manifest_id: ManifestId,
        manifest_data: &str,
    ) -> CoreResult<()> {
        let conn = self.metadata_connection()?;
        conn.execute(
            "DELETE FROM blob_refs WHERE owner_kind = 'manifest' AND owner_id = ?1",
            [manifest_id.to_string()],
        )
        .map_err(storage_err)?;
        for hash_hex in extract_manifest_blob_hashes(manifest_data) {
            conn.execute(
                "INSERT OR IGNORE INTO blob_refs (owner_kind, owner_id, hash, ref_kind, created_at)
                 VALUES ('manifest', ?1, ?2, 'entry', datetime('now'))",
                params![manifest_id.to_string(), hash_hex],
            )
            .map_err(storage_err)?;
        }
        Ok(())
    }}
impl HistoryStore {
    fn rebuild_manifest_blob_refs(&self) -> CoreResult<()> {
        let conn = self.metadata_connection()?;
        conn.execute("DELETE FROM blob_refs WHERE owner_kind = 'manifest'", [])
            .map_err(storage_err)?;
        let mut stmt = conn
            .prepare("SELECT id, manifest_data FROM manifests")
            .map_err(storage_err)?;
        let manifests = collect_mapped_rows(
            stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage_err)?,
        )?;
        drop(stmt);
        for (id, manifest_data) in manifests {
            self.record_manifest_blob_refs(
                id.parse()
                    .map_err(|_| CoreError::Storage("invalid manifest id".to_string()))?,
                &manifest_data,
            )?;
        }
        Ok(())
    }}
impl HistoryStore {
    fn live_manifest_hashes(
        &self,
        conn: &Connection,
    ) -> CoreResult<std::collections::HashSet<String>> {
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT r.hash
                 FROM blob_refs r
                 INNER JOIN manifests m ON m.id = r.owner_id
                 WHERE r.owner_kind = 'manifest'",
            )
            .map_err(storage_err)?;
        let hashes = collect_mapped_rows(
            stmt.query_map([], |row| row.get::<_, String>(0))
                .map_err(storage_err)?,
        )?;
        Ok(hashes.into_iter().collect())
    }}
impl HistoryStore {
    fn write_object_if_missing(&self, hash_hex: &str, contents: &[u8]) -> CoreResult<u64> {
        let final_path = self.object_path_for_hash(hash_hex);
        if final_path.is_file() {
            return Ok(fs::metadata(final_path)
                .map_err(|e| CoreError::Storage(e.to_string()))?
                .len());
        }
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| CoreError::Storage(format!("create object dir: {}", e)))?;
        }
        let temp_path = final_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!(
                ".tmp-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|e| CoreError::Storage(e.to_string()))?
                    .as_nanos(),
                hash_hex
            ));
        let mut file = fs::File::create(&temp_path)
            .map_err(|e| CoreError::Storage(format!("create temp object: {}", e)))?;
        file.write_all(contents)
            .map_err(|e| CoreError::Storage(format!("write temp object: {}", e)))?;
        file.sync_all()
            .map_err(|e| CoreError::Storage(format!("sync temp object: {}", e)))?;
        fs::rename(&temp_path, &final_path)
            .map_err(|e| CoreError::Storage(format!("rename temp object: {}", e)))?;
        Ok(contents.len() as u64)
    }}
impl HistoryStore {
    fn migrate_inline_blob_hash(&self, hash_hex: &str) -> CoreResult<bool> {
        let Some(contents) = self.read_inline_blob(hash_hex)? else {
            return Ok(false);
        };
        let stored_size = self.write_object_if_missing(hash_hex, &contents)?;
        let conn = self.metadata_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO blob_objects
             (hash, codec, logical_size, stored_size, path, created_at, last_verified_at)
             VALUES (?1, 'identity', ?2, ?3, ?4, datetime('now'), datetime('now'))",
            params![
                hash_hex,
                contents.len() as i64,
                stored_size as i64,
                self.object_path_for_hash(hash_hex).display().to_string(),
            ],
        )
        .map_err(storage_err)?;
        conn.execute("UPDATE blobs SET content = X'' WHERE hash = ?1", [hash_hex])
            .map_err(storage_err)?;
        Ok(true)
    }}
impl HistoryStore {
    fn object_path_for_hash(&self, hash_hex: &str) -> PathBuf {
        let prefix_a = &hash_hex[0..2];
        let prefix_b = &hash_hex[2..4];
        self.object_store_dir()
            .join(prefix_a)
            .join(prefix_b)
            .join(hash_hex)
    }}
impl HistoryStore {
    fn read_inline_blob(&self, hash_hex: &str) -> CoreResult<Option<Vec<u8>>> {
        let conn = self.metadata_connection()?;
        conn.query_row(
            "SELECT content FROM blobs WHERE hash = ?1 AND length(content) > 0",
            [hash_hex],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_err)
    }}
impl HistoryStore {
    fn external_blob_row(&self, hash_hex: &str) -> CoreResult<Option<(String, String)>> {
        let conn = self.metadata_connection()?;
        conn.query_row(
            "SELECT path, codec FROM blob_objects WHERE hash = ?1",
            [hash_hex],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_err)
    }}
impl BlobStore for HistoryStore {
    fn write_blob(&mut self, hash: &[u8], data: &[u8]) -> CoreResult<()> {
        let expected_hash = hex::encode(hash);
        let actual_hash = ai_history_core::ContentHash::new(data).to_hex();
        if expected_hash != actual_hash {
            return Err(CoreError::Storage(
                "blob hash does not match content".to_string(),
            ));
        }
        if self.external_blob_row(&expected_hash)?.is_some() {
            return Ok(());
        }
        if self.read_inline_blob(&expected_hash)?.is_some() {
            self.migrate_inline_blob_hash(&expected_hash)?;
            return Ok(());
        }
        let quota = self.quota_report()?;
        if quota.stored_bytes + data.len() as u64 > quota.max_total_blob_bytes {
            return Err(CoreError::Storage(format!(
                "blob quota exceeded: {} + {} > {}",
                quota.stored_bytes,
                data.len(),
                quota.max_total_blob_bytes
            )));
        }
        let stored_size = self.write_object_if_missing(&expected_hash, data)?;
        let conn = self.metadata_connection()?;
        conn.execute(
            "INSERT OR IGNORE INTO blobs (id, session_id, hash, size, content, created_at)
             VALUES (lower(hex(randomblob(16))), NULL, ?1, ?2, X'', datetime('now'))",
            params![expected_hash, data.len() as i64],
        )
        .map_err(storage_err)?;
        conn.execute(
            "INSERT OR REPLACE INTO blob_objects
             (hash, codec, logical_size, stored_size, path, created_at, last_verified_at)
             VALUES (?1, 'identity', ?2, ?3, ?4, datetime('now'), datetime('now'))",
            params![
                expected_hash,
                data.len() as i64,
                stored_size as i64,
                self.object_path_for_hash(&actual_hash)
                    .display()
                    .to_string(),
            ],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    fn read_blob(&self, hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
        let hash_hex = hex::encode(hash);
        if let Some((path, codec)) = self.external_blob_row(&hash_hex)? {
            if codec != "identity" {
                return Err(CoreError::Storage(format!(
                    "unsupported blob codec: {}",
                    codec
                )));
            }
            let path = PathBuf::from(path);
            if !path.is_file() {
                return Err(CoreError::Storage(format!(
                    "external blob metadata exists but object is missing: {}",
                    hash_hex
                )));
            }
            return fs::read(path)
                .map(Some)
                .map_err(|e| CoreError::Storage(format!("read external blob: {}", e)));
        }
        self.read_inline_blob(&hash_hex)
    }

    fn has_blob(&self, hash: &[u8]) -> bool {
        let hash_hex = hex::encode(hash);
        self.external_blob_row(&hash_hex)
            .map(|row| row.is_some())
            .unwrap_or(false)
            || self
                .read_inline_blob(&hash_hex)
                .map(|row| row.is_some())
                .unwrap_or(false)
    }

    fn delete_blob(&mut self, hash: &[u8]) -> CoreResult<()> {
        let hash_hex = hex::encode(hash);
        let conn = self.metadata_connection()?;
        if let Some((path, _)) = self.external_blob_row(&hash_hex)? {
            let path = PathBuf::from(path);
            if path.is_file() {
                fs::remove_file(&path)
                    .map_err(|e| CoreError::Storage(format!("delete external blob: {}", e)))?;
                prune_empty_object_dirs(&self.object_store_dir(), path.parent());
            }
        }
        conn.execute(
            "DELETE FROM blob_objects WHERE hash = ?1",
            [hash_hex.as_str()],
        )
        .map_err(storage_err)?;
        conn.execute("DELETE FROM blob_refs WHERE hash = ?1", [hash_hex.as_str()])
            .map_err(storage_err)?;
        conn.execute("DELETE FROM blobs WHERE hash = ?1", [hash_hex.as_str()])
            .map_err(storage_err)?;
        Ok(())
    }

    fn list_blobs(&self, _session_id: SessionId) -> CoreResult<Vec<BlobInfo>> {
        let conn = self.metadata_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT b.id, b.hash, COALESCE(o.logical_size, b.size), b.created_at
                 FROM blobs b
                 LEFT JOIN blob_objects o ON o.hash = b.hash
                 ORDER BY b.created_at",
            )
            .map_err(storage_err)?;
        let blobs = collect_mapped_rows(
            stmt.query_map([], |row| {
                Ok(BlobInfo {
                    id: row.get(0)?,
                    hash: row.get(1)?,
                    size: row.get::<_, i64>(2)? as u64,
                    created_at: row.get(3)?,
                })
            })
            .map_err(storage_err)?,
        )?;
        Ok(blobs)
    }
}
impl GcStore for HistoryStore {
    fn get_all_session_ids(&self) -> CoreResult<Vec<SessionId>> {
        self.inner.get_all_session_ids()
    }

    fn get_branch_ids_for_session(&self, session_id: SessionId) -> CoreResult<Vec<BranchId>> {
        self.inner.get_branch_ids_for_session(session_id)
    }

    fn get_checkpoint_ids_for_session(
        &self,
        session_id: SessionId,
    ) -> CoreResult<Vec<CheckpointId>> {
        self.inner.get_checkpoint_ids_for_session(session_id)
    }

    fn get_all_blob_ids(&self) -> CoreResult<Vec<BlobId>> {
        let conn = self.metadata_connection()?;
        let mut stmt = conn
            .prepare("SELECT hash FROM blobs UNION SELECT hash FROM blob_objects")
            .map_err(storage_err)?;
        let hashes = collect_mapped_rows(
            stmt.query_map([], |row| row.get::<_, String>(0))
                .map_err(storage_err)?,
        )?;
        hashes
            .into_iter()
            .map(|hash| {
                format!("blob:{}", hash)
                    .parse()
                    .map_err(|_| CoreError::Storage("invalid blob id".to_string()))
            })
            .collect()
    }

    fn get_checkpoint_info(&self, id: CheckpointId) -> CoreResult<Option<GcCheckpointInfo>> {
        self.inner.get_checkpoint_info(id)
    }

    fn get_manifest_info(&self, id: ManifestId) -> CoreResult<Option<GcManifestInfo>> {
        self.inner.get_manifest_info(id)
    }

    fn get_branch_info(&self, id: BranchId) -> CoreResult<Option<GcBranchInfo>> {
        self.inner.get_branch_info(id)
    }

    fn get_branch_head(&self, branch_id: BranchId) -> CoreResult<Option<TurnId>> {
        self.inner.get_branch_head(branch_id)
    }

    fn delete_blob(&self, blob_id: &BlobId) -> CoreResult<()> {
        let mut clone = self.clone_for_delete()?;
        BlobStore::delete_blob(
            &mut clone,
            blob_id.to_string().trim_start_matches("blob:").as_bytes(),
        )
    }

    fn delete_checkpoint(&self, id: &CheckpointId) -> CoreResult<()> {
        self.delete_checkpoint(*id)
    }

    fn delete_manifest(&self, id: &ManifestId) -> CoreResult<()> {
        self.delete_manifest(*id)
    }
}
impl Deref for HistoryStore {
    type Target = SqliteStore;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for HistoryStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl HistoryStore {
    fn clone_for_delete(&self) -> CoreResult<Self> {
        Self::open(&self.db_path)
    }}
