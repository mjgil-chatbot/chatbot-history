use super::HistoryStore;
use crate::{BlobStore, CheckpointInfo, ManifestInfo};
use ai_history_core::{BranchId, CheckpointId, ContentHash, ManifestId, SessionId, TurnId};
use rusqlite::Connection;

fn create_store() -> (HistoryStore, tempfile::TempDir) {
    let temp = tempfile::tempdir().unwrap();
    let store = HistoryStore::open(temp.path().join("store.db")).unwrap();
    (store, temp)
}

fn seed_session(store: &mut HistoryStore) -> (SessionId, BranchId, CheckpointId, ManifestId) {
    let session_id = SessionId::new();
    let branch_id = BranchId::new();
    let checkpoint_id = CheckpointId::new();
    let manifest_id = ManifestId::new();
    store.create_session(session_id, "/tmp").unwrap();
    store
        .create_branch(&crate::BranchInfo {
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
            created_at: "2026-05-21T00:00:00Z".to_string(),
        })
        .unwrap();
    store
        .create_turn(&crate::TurnInfo {
            id: TurnId::new(),
            session_id,
            branch_id,
            parent_turn_id: None,
            actor: "User".to_string(),
            created_at: "2026-05-21T00:00:00Z".to_string(),
            finalized_at: None,
        })
        .unwrap();
    store
        .create_checkpoint(&CheckpointInfo {
            id: checkpoint_id,
            session_id,
            branch_id,
            turn_id: None,
            manifest_id,
            reason: "test".to_string(),
            created_at: "2026-05-21T00:00:00Z".to_string(),
        })
        .unwrap();
    (session_id, branch_id, checkpoint_id, manifest_id)
}

#[test]
fn write_blob_externalizes_content_and_tracks_metadata() {
    let (mut store, _temp) = create_store();
    let contents = b"hello blob";
    let hash = ContentHash::new(contents);

    store.write_blob(hash.as_bytes(), contents).unwrap();

    let quota = store.quota_report().unwrap();
    assert_eq!(quota.object_count, 1);
    assert_eq!(quota.inline_blob_count, 0);
    assert_eq!(
        store.read_blob(hash.as_bytes()).unwrap().as_deref(),
        Some(contents.as_slice())
    );
    assert!(store.object_path_for_hash(&hash.to_hex()).is_file());
}

#[test]
fn write_blob_migrates_matching_inline_blob() {
    let (mut store, _temp) = create_store();
    let contents = b"legacy inline blob";
    let hash_hex = ContentHash::new(contents).to_hex();
    Connection::open(store.db_path())
        .unwrap()
        .execute(
            "INSERT INTO blobs (id, session_id, hash, size, content, created_at)
             VALUES ('legacy', NULL, ?1, ?2, ?3, datetime('now'))",
            rusqlite::params![hash_hex, contents.len() as i64, contents],
        )
        .unwrap();

    store
        .write_blob(ContentHash::new(contents).as_bytes(), contents)
        .unwrap();

    let quota = store.quota_report().unwrap();
    assert_eq!(quota.inline_blob_count, 0);
    assert_eq!(quota.object_count, 1);
}

#[test]
fn metadata_snapshot_omits_inline_blob_content() {
    let (mut store, temp) = create_store();
    let contents = b"checkpoint payload";
    let hash = ContentHash::new(contents);
    let (session_id, _branch_id, checkpoint_id, manifest_id) = seed_session(&mut store);
    store.write_blob(hash.as_bytes(), contents).unwrap();
    store
        .create_manifest(&ManifestInfo {
            id: manifest_id,
            session_id,
            checkpoint_id,
            manifest_data: format!(
                "{{\"entries\":{{\"file.txt\":{{\"blob_hash\":\"{}\"}}}}}}",
                hash.to_hex()
            ),
            created_at: "2026-05-21T00:00:00Z".to_string(),
        })
        .unwrap();

    let snapshot_path = temp.path().join("backup.db");
    store.write_metadata_snapshot(&snapshot_path).unwrap();
    let snapshot = HistoryStore::open(&snapshot_path).unwrap();
    let quota = snapshot.quota_report().unwrap();
    assert_eq!(quota.inline_blob_count, 0);
    assert_eq!(quota.object_count, 1);
    assert_eq!(
        snapshot.read_blob(hash.as_bytes()).unwrap().as_deref(),
        Some(contents.as_slice())
    );
}

#[test]
fn quota_enforcement_rejects_new_blob_over_limit() {
    let (mut store, _temp) = create_store();
    store.set_max_total_blob_bytes(4).unwrap();

    let contents = b"too large";
    let error = store
        .write_blob(ContentHash::new(contents).as_bytes(), contents)
        .unwrap_err();

    assert!(error.to_string().contains("blob quota exceeded"));
}

#[test]
fn garbage_collect_objects_removes_unreferenced_rows_and_files() {
    let (mut store, _temp) = create_store();
    let contents = b"orphan";
    let hash = ContentHash::new(contents);
    let (session_id, _branch_id, checkpoint_id, manifest_id) = seed_session(&mut store);
    store.write_blob(hash.as_bytes(), contents).unwrap();
    store
        .create_manifest(&ManifestInfo {
            id: manifest_id,
            session_id,
            checkpoint_id,
            manifest_data: format!(
                "{{\"entries\":{{\"file.txt\":{{\"blob_hash\":\"{}\"}}}}}}",
                hash.to_hex()
            ),
            created_at: "2026-05-21T00:00:00Z".to_string(),
        })
        .unwrap();
    let path = store.object_path_for_hash(&hash.to_hex());
    assert!(path.is_file());
    store.delete_checkpoint(checkpoint_id).unwrap();
    Connection::open(store.db_path())
        .unwrap()
        .execute(
            "INSERT INTO store_settings (key, value, updated_at)
             VALUES ('fresh_write_grace_hours', '0', datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            [],
        )
        .unwrap();

    let report = store.garbage_collect_objects(true).unwrap();

    assert_eq!(report.deleted_rows, 1);
    assert!(!path.exists());
}
