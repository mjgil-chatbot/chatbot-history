//! Fake implementations for testing

use ai_history_core::*;
use ai_history_events::Event;
use ai_history_store::traits::BlobStore;
use std::collections::HashMap;
use std::sync::Mutex;

/// Fake blob store for testing
pub struct FakeBlobStore {
    blobs: Mutex<HashMap<String, Vec<u8>>>,
    hashes: Mutex<HashMap<String, u64>>,
}

impl FakeBlobStore {
    pub fn new() -> Self {
        Self {
            blobs: Mutex::new(HashMap::new()),
            hashes: Mutex::new(HashMap::new()),
        }
    }

    pub fn write(&self, hash: &[u8], data: &[u8]) -> CoreResult<()> {
        let mut blobs = self.blobs.lock().unwrap();
        let mut hashes = self.hashes.lock().unwrap();
        blobs.insert(hex::encode(hash), data.to_vec());
        hashes.insert(hex::encode(hash), data.len() as u64);
        Ok(())
    }

    pub fn read(&self, hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
        let blobs = self.blobs.lock().unwrap();
        Ok(blobs.get(&hex::encode(hash)).cloned())
    }

    pub fn has(&self, hash: &[u8]) -> bool {
        let blobs = self.blobs.lock().unwrap();
        blobs.contains_key(&hex::encode(hash))
    }

    pub fn size(&self, hash: &[u8]) -> Option<u64> {
        let hashes = self.hashes.lock().unwrap();
        hashes.get(&hex::encode(hash)).copied()
    }

    pub fn clear(&self) {
        let mut blobs = self.blobs.lock().unwrap();
        let mut hashes = self.hashes.lock().unwrap();
        blobs.clear();
        hashes.clear();
    }
}

impl Default for FakeBlobStore {
    fn default() -> Self {
        Self::new()
    }
}

impl BlobStore for FakeBlobStore {
    fn write_blob(&mut self, hash: &[u8], data: &[u8]) -> CoreResult<()> {
        let mut blobs = self.blobs.lock().unwrap();
        let mut hashes = self.hashes.lock().unwrap();
        blobs.insert(hex::encode(hash), data.to_vec());
        hashes.insert(hex::encode(hash), data.len() as u64);
        Ok(())
    }

    fn read_blob(&self, hash: &[u8]) -> CoreResult<Option<Vec<u8>>> {
        let blobs = self.blobs.lock().unwrap();
        Ok(blobs.get(&hex::encode(hash)).cloned())
    }

    fn has_blob(&self, hash: &[u8]) -> bool {
        let blobs = self.blobs.lock().unwrap();
        blobs.contains_key(&hex::encode(hash))
    }

    fn delete_blob(&mut self, hash: &[u8]) -> CoreResult<()> {
        let mut blobs = self.blobs.lock().unwrap();
        let mut hashes = self.hashes.lock().unwrap();
        let key = hex::encode(hash);
        blobs.remove(&key);
        hashes.remove(&key);
        Ok(())
    }

    fn list_blobs(
        &self,
        _session_id: SessionId,
    ) -> CoreResult<Vec<ai_history_store::traits::BlobInfo>> {
        let hashes = self.hashes.lock().unwrap();
        let blobs = self.blobs.lock().unwrap();
        let mut result = Vec::new();
        for (hash, size) in hashes.iter() {
            if blobs.get(hash).is_some() {
                result.push(ai_history_store::traits::BlobInfo {
                    id: hash.clone(),
                    hash: hash.clone(),
                    size: *size,
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                });
            }
        }
        Ok(result)
    }
}

/// Fake event store for testing
pub struct FakeEventStore {
    events: Mutex<Vec<(SessionId, Event)>>,
}

impl FakeEventStore {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    pub fn append(&self, session_id: SessionId, event: Event) {
        let mut events = self.events.lock().unwrap();
        events.push((session_id, event));
    }

    pub fn get_all(&self, session_id: SessionId) -> Vec<Event> {
        let events = self.events.lock().unwrap();
        events
            .iter()
            .filter(|(sid, _)| *sid == session_id)
            .map(|(_, e)| e.clone())
            .collect()
    }

    pub fn clear(&self) {
        let mut events = self.events.lock().unwrap();
        events.clear();
    }
}

impl Default for FakeEventStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministic clock for testing
pub struct DeterministicClock {
    time: Mutex<i64>,
}

impl DeterministicClock {
    pub fn new(start_time: i64) -> Self {
        Self {
            time: Mutex::new(start_time),
        }
    }

    pub fn advance(&self, seconds: i64) {
        let mut time = self.time.lock().unwrap();
        *time += seconds;
    }

    pub fn now(&self) -> Timestamp {
        let time = *self.time.lock().unwrap();
        // Convert Unix timestamp to DateTime<Utc>
        let dt = chrono::DateTime::from_timestamp(time, 0).unwrap_or_else(|| chrono::Utc::now());
        Timestamp::from_datetime(dt)
    }
}

impl Default for DeterministicClock {
    fn default() -> Self {
        Self::new(1704067200) // 2024-01-01 00:00:00 UTC
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_history_events::SessionCreatedData;

    #[test]
    fn test_fake_blob_store() {
        let store = FakeBlobStore::new();
        let data = b"hello world";
        let hash = ContentHash::new(data);

        store.write(hash.as_bytes(), data).unwrap();
        assert!(store.has(hash.as_bytes()));

        let read = store.read(hash.as_bytes()).unwrap().unwrap();
        assert_eq!(read, data);
    }

    #[test]
    fn test_fake_event_store() {
        let store = FakeEventStore::new();
        let session_id = SessionId::new();

        let event = Event::SessionCreated(SessionCreatedData {
            session_id,
            workspace_root: "/tmp".to_string(),
            timestamp: Timestamp::now(),
        });

        store.append(session_id, event.clone());
        let events = store.get_all(session_id);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_deterministic_clock() {
        let clock = DeterministicClock::new(1000);
        let ts1 = clock.now();
        clock.advance(3600);
        let ts2 = clock.now();
        // The timestamps should be different
        assert_ne!(ts1.to_string(), ts2.to_string());
    }
}
