//! Blob store configuration and traits
//!
//! Provides configuration types for content-addressed blob storage.
//! The concrete BlobStore implementation lives in ai-history-snapshot.

/// Configuration for blob store behavior
#[derive(Debug, Clone)]
pub struct BlobStoreConfig {
    /// Enable atomic writes using temp file + rename
    pub atomic_writes: bool,
}

impl Default for BlobStoreConfig {
    fn default() -> Self {
        Self {
            atomic_writes: true,
        }
    }
}

impl BlobStoreConfig {
    pub fn with_atomic_writes(mut self, enabled: bool) -> Self {
        self.atomic_writes = enabled;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_store_config_default() {
        let config = BlobStoreConfig::default();
        assert!(config.atomic_writes);
    }

    #[test]
    fn test_blob_store_config_accessors() {
        let config = BlobStoreConfig::default().with_atomic_writes(false);
        assert!(!config.atomic_writes);
    }

    #[test]
    fn test_blob_store_config_debug() {
        let config = BlobStoreConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("BlobStoreConfig"));
    }
}
