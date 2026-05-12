use ai_history_core::{CoreError, CoreResult};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rusqlite::Connection;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::time::Duration;

pub(super) fn extract_manifest_blob_hashes(manifest_data: &str) -> Vec<String> {
    let mut hashes = Vec::new();
    let Ok(json) = serde_json::from_str::<Value>(manifest_data) else {
        return hashes;
    };
    let Some(entries) = json.get("entries").and_then(|entries| entries.as_object()) else {
        return hashes;
    };
    for entry in entries.values() {
        if let Some(hash) = entry.get("blob_hash").and_then(|value| value.as_str()) {
            hashes.push(hash.trim_start_matches("blob:").to_string());
        }
    }
    hashes
}

pub(super) fn is_older_than_grace(raw: &str, grace: Duration) -> bool {
    parse_store_timestamp(raw)
        .and_then(|created_at| Utc::now().signed_duration_since(created_at).to_std().ok())
        .map(|age| age >= grace)
        .unwrap_or(false)
}

pub(super) fn prune_empty_object_dirs(root: &Path, mut current: Option<&Path>) {
    while let Some(path) = current {
        if path == root {
            break;
        }
        match fs::remove_dir(path) {
            Ok(()) => current = path.parent(),
            Err(_) => break,
        }
    }
}

pub(super) fn query_count(conn: &Connection, sql: &str) -> CoreResult<usize> {
    conn.query_row(sql, [], |row| row.get::<_, i64>(0))
        .map(|count| count as usize)
        .map_err(storage_err)
}

pub(super) fn query_sum(conn: &Connection, sql: &str) -> CoreResult<u64> {
    conn.query_row(sql, [], |row| row.get::<_, i64>(0))
        .map(|sum| sum.max(0) as u64)
        .map_err(storage_err)
}

pub(super) fn collect_mapped_rows<T, F>(mapped: rusqlite::MappedRows<'_, F>) -> CoreResult<Vec<T>>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    mapped.collect::<Result<Vec<_>, _>>().map_err(storage_err)
}

pub(super) fn storage_err(error: rusqlite::Error) -> CoreError {
    CoreError::Storage(error.to_string())
}

fn parse_store_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
        return Some(parsed.with_timezone(&Utc));
    }
    if let Ok(parsed) = NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S%.f") {
        return Some(parsed.and_utc());
    }
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .ok()?
        .and_hms_opt(0, 0, 0)
        .map(|naive| naive.and_utc())
}
