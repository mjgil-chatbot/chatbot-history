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
    }}
impl ContentHash {
    /// Alias for from_data for backwards compatibility
    pub fn new(data: &[u8]) -> Self {
        Self::from_data(data)
    }}
impl Default for ContentHash {
    fn default() -> Self {
        Self([0u8; 32])
    }
}
