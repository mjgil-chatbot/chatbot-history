//! Blob store for content-addressed storage
//!
//! Provides atomic writes, deduplication, corruption detection,
//! and configurable storage location.
//!
//! NOTE: BlobStoreConfig is defined in ai-history-core to allow configuration
//! without depending on concrete storage backends.

use ai_history_core::{BlobStoreConfig, ContentHash, CoreError, CoreResult};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

/// Content-addressed blob store
pub struct BlobStore {
    root: PathBuf,
    cache: HashMap<String, Vec<u8>>,
    config: BlobStoreConfig,
}

impl BlobStore {
    pub fn new(root: PathBuf) -> CoreResult<Self> {
        Self::with_config(root, BlobStoreConfig::default())
    }

    pub fn with_config(root: PathBuf, config: BlobStoreConfig) -> CoreResult<Self> {
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            cache: HashMap::new(),
            config,
        })
    }

    /// Get the root path
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Get the configuration
    pub fn config(&self) -> &BlobStoreConfig {
        &self.config
    }

    /// Write a blob, returns the hash
    pub fn write(&mut self, data: &[u8]) -> CoreResult<ContentHash> {
        let hash = ContentHash::new(data);
        let key = hash.to_hex();

        // Check if already exists (deduplication)
        if self.has(&hash) {
            return Ok(hash);
        }

        // Write to disk using atomic write if enabled
        let path = self.blob_path(&hash);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        if self.config.atomic_writes {
            // Atomic write: temp file + rename
            let temp_path = self.temp_path(&hash);
            let mut file = fs::File::create(&temp_path).map_err(|e| {
                CoreError::Filesystem(format!("Failed to create temp blob file: {}", e))
            })?;
            file.write_all(data)
                .map_err(|e| CoreError::Filesystem(format!("Failed to write temp blob: {}", e)))?;
            file.sync_all().map_err(|e| {
                CoreError::Filesystem(format!("Filesystem error: Failed to sync temp blob: {}", e))
            })?;
            // Atomic rename
            fs::rename(&temp_path, &path).map_err(|e| {
                CoreError::Filesystem(format!(
                    "Filesystem error: Failed to rename temp blob: {}",
                    e
                ))
            })?;
        } else {
            // Direct write
            let mut file = fs::File::create(&path)
                .map_err(|e| CoreError::Filesystem(format!("Failed to create blob file: {}", e)))?;
            file.write_all(data)
                .map_err(|e| CoreError::Filesystem(format!("Failed to write blob: {}", e)))?;
        }

        // Cache it
        self.cache.insert(key, data.to_vec());

        Ok(hash)
    }

    fn temp_path(&self, hash: &ContentHash) -> PathBuf {
        let hex = hash.to_hex();
        let prefix = &hex[0..2];
        self.root.join(format!("{}/.tmp_{}", prefix, hex))
    }

    /// Read a blob by hash
    pub fn read(&self, hash: &ContentHash) -> CoreResult<Option<Vec<u8>>> {
        let key = hash.to_hex();

        // Check cache first
        if let Some(data) = self.cache.get(&key) {
            return Ok(Some(data.clone()));
        }

        // Read from disk
        let path = self.blob_path(hash);
        if !path.exists() {
            return Ok(None);
        }

        let mut file = fs::File::open(&path).map_err(|e| {
            CoreError::Filesystem(format!("Filesystem error: Failed to open blob: {}", e))
        })?;

        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(|e| {
            CoreError::Filesystem(format!("Filesystem error: Failed to read blob: {}", e))
        })?;

        // Verify integrity
        let computed_hash = ContentHash::new(&data);
        if computed_hash != *hash {
            return Err(CoreError::Filesystem(
                "Blob corrupted: hash mismatch".to_string(),
            ));
        }

        Ok(Some(data))
    }

    /// Check if a blob exists
    pub fn has(&self, hash: &ContentHash) -> bool {
        let key = hash.to_hex();
        if self.cache.contains_key(&key) {
            return true;
        }
        self.blob_path(hash).exists()
    }

    /// Get blob size
    pub fn size(&self, hash: &ContentHash) -> CoreResult<Option<u64>> {
        if let Some(data) = self.cache.get(&hash.to_hex()) {
            return Ok(Some(data.len() as u64));
        }

        let path = self.blob_path(hash);
        if !path.exists() {
            return Ok(None);
        }

        let metadata = fs::metadata(&path).map_err(|e| {
            CoreError::Filesystem(format!(
                "Filesystem error: Failed to get blob metadata: {}",
                e
            ))
        })?;
        Ok(Some(metadata.len()))
    }

    /// Delete a blob
    pub fn delete(&mut self, hash: &ContentHash) -> CoreResult<()> {
        let key = hash.to_hex();
        self.cache.remove(&key);

        let path = self.blob_path(hash);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                CoreError::Filesystem(format!("Filesystem error: Failed to delete blob: {}", e))
            })?;
        }

        Ok(())
    }

    /// Verify blob integrity
    pub fn verify(&self, hash: &ContentHash) -> CoreResult<bool> {
        let path = self.blob_path(hash);
        if !path.exists() {
            return Ok(false);
        }

        let mut file = fs::File::open(&path).map_err(|e| {
            CoreError::Filesystem(format!("Filesystem error: Failed to open blob: {}", e))
        })?;

        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(|e| {
            CoreError::Filesystem(format!("Filesystem error: Failed to read blob: {}", e))
        })?;

        let computed_hash = ContentHash::new(&data);
        Ok(computed_hash == *hash)
    }

    fn blob_path(&self, hash: &ContentHash) -> PathBuf {
        let hex = hash.to_hex();
        let prefix = &hex[0..2];
        let root = &self.root;
        root.join(format!("{}/{}", prefix, hex))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_blob_store_write_read() {
        let temp = TempDir::new().unwrap();
        let mut store = BlobStore::new(temp.path().to_path_buf()).unwrap();

        let data = b"hello world";
        let hash = store.write(data).unwrap();

        let read = store.read(&hash).unwrap().unwrap();
        assert_eq!(read, data);
    }

    #[test]
    fn test_blob_store_deduplication() {
        let temp = TempDir::new().unwrap();
        let mut store = BlobStore::new(temp.path().to_path_buf()).unwrap();

        let data = b"hello world";
        let hash1 = store.write(data).unwrap();
        let hash2 = store.write(data).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_blob_store_has() {
        let temp = TempDir::new().unwrap();
        let mut store = BlobStore::new(temp.path().to_path_buf()).unwrap();

        let data = b"test data";
        let hash = store.write(data).unwrap();

        assert!(store.has(&hash));
        assert!(!store.has(&ContentHash::new(b"other")));
    }

    #[test]
    fn test_blob_store_size() {
        let temp = TempDir::new().unwrap();
        let mut store = BlobStore::new(temp.path().to_path_buf()).unwrap();

        let data = b"hello world";
        let hash = store.write(data).unwrap();

        let size = store.size(&hash).unwrap().unwrap();
        assert_eq!(size, 11);
    }

    #[test]
    fn test_blob_store_delete() {
        let temp = TempDir::new().unwrap();
        let mut store = BlobStore::new(temp.path().to_path_buf()).unwrap();

        let data = b"hello world";
        let hash = store.write(data).unwrap();

        store.delete(&hash).unwrap();
        assert!(!store.has(&hash));
    }

    #[test]
    fn test_blob_store_verify() {
        let temp = TempDir::new().unwrap();
        let mut store = BlobStore::new(temp.path().to_path_buf()).unwrap();

        let data = b"test content";
        let hash = store.write(data).unwrap();

        assert!(store.verify(&hash).unwrap());
    }

    #[test]
    fn test_blob_store_corruption_detection() {
        let temp = TempDir::new().unwrap();
        let mut store = BlobStore::new(temp.path().to_path_buf()).unwrap();

        let data = b"hello world";
        let hash = store.write(data).unwrap();

        // Corrupt the blob
        let path = store.blob_path(&hash);
        fs::write(&path, b"corrupted data").unwrap();

        // Verify should fail
        assert!(!store.verify(&hash).unwrap());
    }

    #[test]
    fn test_blob_store_atomic_writes_enabled() {
        let temp = TempDir::new().unwrap();
        let config = BlobStoreConfig::default().with_atomic_writes(true);
        let mut store = BlobStore::with_config(temp.path().to_path_buf(), config).unwrap();

        let data = b"hello world";
        let hash = store.write(data).unwrap();

        // Verify write succeeded
        assert!(store.has(&hash));
        assert!(store.verify(&hash).unwrap());
    }

    #[test]
    fn test_blob_store_atomic_writes_disabled() {
        let temp = TempDir::new().unwrap();
        let config = BlobStoreConfig::default().with_atomic_writes(false);
        let mut store = BlobStore::with_config(temp.path().to_path_buf(), config).unwrap();

        let data = b"hello world";
        let hash = store.write(data).unwrap();

        // Verify write succeeded
        assert!(store.has(&hash));
        assert!(store.verify(&hash).unwrap());
    }

    #[test]
    fn test_blob_store_config_accessors() {
        let config = BlobStoreConfig::default();
        assert!(config.atomic_writes);

        let config = config.with_atomic_writes(false);
        assert!(!config.atomic_writes);
    }

    #[test]
    fn test_blob_store_deduplication_across_caches() {
        let temp = TempDir::new().unwrap();
        let mut store = BlobStore::new(temp.path().to_path_buf()).unwrap();

        let data = b"unique content";
        let hash1 = store.write(data).unwrap();
        let hash2 = store.write(data).unwrap();

        // Same hash means deduplication works
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_blob_store_missing_blob_returns_none() {
        let temp = TempDir::new().unwrap();
        let store = BlobStore::new(temp.path().to_path_buf()).unwrap();

        let missing_hash = ContentHash::new(b"does not exist");
        assert!(!store.has(&missing_hash));
        assert!(store.read(&missing_hash).unwrap().is_none());
    }

    #[test]
    fn test_blob_store_config_debug() {
        let config = BlobStoreConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("BlobStoreConfig"));
    }

    #[test]
    fn test_blob_store_root_path() {
        let temp = TempDir::new().unwrap();
        let store = BlobStore::new(temp.path().to_path_buf()).unwrap();

        assert_eq!(store.root(), temp.path());
    }

    #[test]
    fn test_blob_store_with_custom_config() {
        let temp = TempDir::new().unwrap();
        let config = BlobStoreConfig::default().with_atomic_writes(false);
        let store = BlobStore::with_config(temp.path().to_path_buf(), config).unwrap();

        assert!(!store.config().atomic_writes);
    }
}
