use crate::workspace_store::HistoryStore;
use ai_history_core::{CoreError, CoreResult};
use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;

const BACKUP_SNAPSHOT_KIND_KEY: &str = "backup_snapshot_kind";
const BACKUP_SNAPSHOT_KIND_METADATA: &str = "metadata-snapshot";

impl HistoryStore {
    pub fn write_metadata_snapshot<P: AsRef<Path>>(&self, destination: P) -> CoreResult<()> {
        let destination = destination.as_ref();
        self.migrate_all_legacy_inline_blobs()?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(io_err)?;
        }
        if destination.exists() {
            fs::remove_file(destination).map_err(io_err)?;
        }

        let _ = Self::open(destination)?;
        let destination_conn = Connection::open(destination).map_err(storage_err)?;
        let source_path = self.db_path().display().to_string();
        destination_conn
            .execute("ATTACH DATABASE ?1 AS source", [source_path.as_str()])
            .map_err(storage_err)?;
        destination_conn
            .execute_batch(
                "BEGIN;
                 DELETE FROM sessions;
                 DELETE FROM branches;
                 DELETE FROM turns;
                 DELETE FROM events;
                 DELETE FROM checkpoints;
                 DELETE FROM manifests;
                 DELETE FROM blobs;
                 DELETE FROM materialized_views;
                 DELETE FROM cursor_position;
                 DELETE FROM app_state;
                 DELETE FROM messages;
                 DELETE FROM context_snapshots;
                 DELETE FROM blob_objects;
                 DELETE FROM blob_refs;
                 DELETE FROM store_settings;
                 INSERT INTO sessions SELECT * FROM source.sessions;
                 INSERT INTO branches SELECT * FROM source.branches;
                 INSERT INTO turns SELECT * FROM source.turns;
                 INSERT INTO events SELECT * FROM source.events;
                 INSERT INTO checkpoints SELECT * FROM source.checkpoints;
                 INSERT INTO manifests SELECT * FROM source.manifests;
                 INSERT INTO blobs (id, session_id, hash, size, content, created_at)
                 SELECT id, session_id, hash, size, X'', created_at FROM source.blobs;
                 INSERT INTO materialized_views SELECT * FROM source.materialized_views;
                 INSERT INTO cursor_position SELECT * FROM source.cursor_position;
                 INSERT INTO app_state SELECT * FROM source.app_state;
                 INSERT INTO messages SELECT * FROM source.messages;
                 INSERT INTO context_snapshots SELECT * FROM source.context_snapshots;
                 INSERT INTO blob_objects SELECT * FROM source.blob_objects;
                 INSERT INTO blob_refs SELECT * FROM source.blob_refs;
                 INSERT INTO store_settings SELECT * FROM source.store_settings;
                 COMMIT;",
            )
            .map_err(storage_err)?;
        destination_conn
            .execute("DETACH DATABASE source", [])
            .map_err(storage_err)?;
        destination_conn
            .execute(
                "INSERT INTO store_settings (key, value, updated_at)
                 VALUES (?1, ?2, datetime('now'))
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                params![BACKUP_SNAPSHOT_KIND_KEY, BACKUP_SNAPSHOT_KIND_METADATA],
            )
            .map_err(storage_err)?;
        destination_conn
            .execute_batch("VACUUM")
            .map_err(storage_err)?;
        Ok(())
    }
}

fn io_err(error: std::io::Error) -> CoreError {
    CoreError::Storage(error.to_string())
}

fn storage_err(error: rusqlite::Error) -> CoreError {
    CoreError::Storage(error.to_string())
}
