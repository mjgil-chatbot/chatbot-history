//! Hash types for content-addressed storage

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{Display, Formatter};

/// SHA-256 content hash
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub fn from_data(data: &[u8]) -> Self {
        let result = Sha256::digest(data);
        Self(result.into())
    }

    /// Alias for from_data for backwards compatibility
    pub fn new(data: &[u8]) -> Self {
        Self::from_data(data)
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, &'static str> {
        let bytes = hex::decode(s).map_err(|_| "invalid hex")?;
        if bytes.len() != 32 {
            return Err("expected 32 bytes");
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl Default for ContentHash {
    fn default() -> Self {
        Self([0u8; 32])
    }
}

impl Display for ContentHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl From<[u8; 32]> for ContentHash {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_new() {
        let hash = ContentHash::new(b"hello world");
        assert_eq!(
            hash.to_hex(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_content_hash_roundtrip() {
        let hash = ContentHash::new(b"test data");
        let hex = hash.to_hex();
        let parsed = ContentHash::from_hex(&hex).unwrap();
        assert_eq!(hash, parsed);
    }

    #[test]
    fn test_content_hash_display() {
        let hash = ContentHash::new(b"");
        let s = hash.to_string();
        assert_eq!(s.len(), 64);
    }
}
