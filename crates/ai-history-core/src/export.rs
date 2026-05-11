//! Import/export functionality for sessions and data
//!
//! Supports exporting sessions, events, branches, checkpoints,
//! manifests, and blobs. Supports import with integrity verification.

use crate::{BlobId, BranchId, CheckpointId, CoreResult, ManifestId, SessionId, TurnId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
/// Archive version for compatibility
pub const EXPORT_VERSION: &str = "1.0";
/// Export metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub version: String,
    pub session_id: SessionId,
    pub exported_at: String,
    pub session_count: usize,
    pub event_count: usize,
    pub branch_count: usize,
    pub checkpoint_count: usize,
    pub manifest_count: usize,
    pub blob_count: usize,
}
/// A complete export package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPackage {
    pub metadata: ExportMetadata,
    pub sessions: Vec<SessionExport>,
    pub branches: Vec<BranchExport>,
    pub checkpoints: Vec<CheckpointExport>,
    pub manifests: Vec<ManifestExport>,
    pub events: Vec<EventExport>,
    pub blobs: Vec<BlobExport>,
}
/// Session data for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExport {
    pub id: SessionId,
    pub workspace_root: String,
    pub created_at: String,
    pub updated_at: String,
}
/// Branch data for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchExport {
    pub id: BranchId,
    pub session_id: SessionId,
    pub name: String,
    pub parent_branch_id: Option<BranchId>,
    pub head_turn_id: Option<TurnId>,
    pub created_at: String,
}
/// Checkpoint data for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointExport {
    pub id: CheckpointId,
    pub session_id: SessionId,
    pub branch_id: BranchId,
    pub turn_id: Option<TurnId>,
    pub manifest_id: ManifestId,
    pub reason: String,
    pub created_at: String,
}
/// Manifest data for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestExport {
    pub id: ManifestId,
    pub session_id: SessionId,
    pub checkpoint_id: CheckpointId,
    pub manifest_data: String, // JSON serialized
    pub created_at: String,
}
/// Event data for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventExport {
    pub id: usize, // Internal sequence number
    pub session_id: SessionId,
    pub branch_id: BranchId,
    pub turn_id: Option<TurnId>,
    pub sequence_num: u64,
    pub event_type: String,
    pub event_data: String, // JSON serialized
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
    pub actor: String,
    pub checksum: String,
    pub created_at: String,
}
/// Blob data for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobExport {
    pub id: BlobId,
    pub session_id: SessionId,
    pub hash: String,
    pub size: u64,
    pub content: Vec<u8>, // Binary content
    pub created_at: String,
}
/// Import result with statistics
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub sessions_imported: usize,
    pub branches_imported: usize,
    pub checkpoints_imported: usize,
    pub manifests_imported: usize,
    pub events_imported: usize,
    pub blobs_imported: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}
impl Default for ImportResult {
    fn default() -> Self {
        Self {
            sessions_imported: 0,
            branches_imported: 0,
            checkpoints_imported: 0,
            manifests_imported: 0,
            events_imported: 0,
            blobs_imported: 0,
            warnings: vec![],
            errors: vec![],
        }
    }
}
impl ImportResult {
    /// Check if import was successful
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }}
impl ImportResult {
    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }}
impl ImportResult {
    /// Add an error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }}
/// Integrity verification result for imports
#[derive(Debug, Clone)]
pub struct IntegrityVerification {
    pub checksum_valid: bool,
    pub blob_hash_valid: bool,
    pub references_valid: bool,
}
impl IntegrityVerification {
    /// Create a passing verification
    pub fn pass() -> Self {
        Self {
            checksum_valid: true,
            blob_hash_valid: true,
            references_valid: true,
        }
    }}
impl IntegrityVerification {
    /// Check if all checks passed
    pub fn all_passed(&self) -> bool {
        self.checksum_valid && self.blob_hash_valid && self.references_valid
    }}
/// Exporter for creating export packages
pub struct Exporter;
impl Exporter {
    /// Create a new exporter
    pub fn new() -> Self {
        Self
    }}
impl Exporter {
    /// Export sessions to a package
    pub fn export_sessions(&self, sessions: &[SessionExport]) -> Vec<SessionExport> {
        sessions.to_vec()
    }}
impl Exporter {
    /// Export branches to a package
    pub fn export_branches(&self, branches: &[BranchExport]) -> Vec<BranchExport> {
        branches.to_vec()
    }}
impl Exporter {
    /// Export checkpoints to a package
    pub fn export_checkpoints(&self, checkpoints: &[CheckpointExport]) -> Vec<CheckpointExport> {
        checkpoints.to_vec()
    }}
impl Exporter {
    /// Export manifests to a package
    pub fn export_manifests(&self, manifests: &[ManifestExport]) -> Vec<ManifestExport> {
        manifests.to_vec()
    }}
impl Exporter {
    /// Export events to a package
    pub fn export_events(&self, events: &[EventExport]) -> Vec<EventExport> {
        events.to_vec()
    }}
impl Exporter {
    /// Export blobs to a package
    pub fn export_blobs(&self, blobs: &[BlobExport]) -> Vec<BlobExport> {
        blobs.to_vec()
    }}
impl Exporter {
    /// Create metadata for export
    pub fn create_metadata(
        &self,
        session_id: SessionId,
        sessions: usize,
        events: usize,
        branches: usize,
        checkpoints: usize,
        manifests: usize,
        blobs: usize,
    ) -> ExportMetadata {
        ExportMetadata {
            version: EXPORT_VERSION.to_string(),
            session_id,
            exported_at: chrono::Utc::now().to_rfc3339(),
            session_count: sessions,
            event_count: events,
            branch_count: branches,
            checkpoint_count: checkpoints,
            manifest_count: manifests,
            blob_count: blobs,
        }
    }}
impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}
/// Importer for restoring from export packages
pub struct Importer;
impl Importer {
    /// Create a new importer
    pub fn new() -> Self {
        Self
    }}
impl Importer {
    /// Import sessions from package
    pub fn import_sessions(&self, package: &ExportPackage) -> Vec<SessionExport> {
        package.sessions.clone()
    }}
impl Importer {
    /// Import branches from package
    pub fn import_branches(&self, package: &ExportPackage) -> Vec<BranchExport> {
        package.branches.clone()
    }}
impl Importer {
    /// Import checkpoints from package
    pub fn import_checkpoints(&self, package: &ExportPackage) -> Vec<CheckpointExport> {
        package.checkpoints.clone()
    }}
impl Importer {
    /// Import manifests from package
    pub fn import_manifests(&self, package: &ExportPackage) -> Vec<ManifestExport> {
        package.manifests.clone()
    }}
impl Importer {
    /// Import events from package
    pub fn import_events(&self, package: &ExportPackage) -> Vec<EventExport> {
        package.events.clone()
    }}
impl Importer {
    /// Import blobs from package
    pub fn import_blobs(&self, package: &ExportPackage) -> Vec<BlobExport> {
        package.blobs.clone()
    }}
impl Default for Importer {
    fn default() -> Self {
        Self::new()
    }
}
/// Archive format utilities
pub struct ArchiveFormat;
impl ArchiveFormat {
    /// Serialize package to bytes
    pub fn serialize(package: &ExportPackage) -> CoreResult<Vec<u8>> {
        serde_json::to_vec(package).map_err(|e| crate::CoreError::Serialization(e.to_string()))
    }}
impl ArchiveFormat {
    /// Deserialize package from bytes
    pub fn deserialize(data: &[u8]) -> CoreResult<ExportPackage> {
        serde_json::from_slice(data).map_err(|e| crate::CoreError::Deserialization(e.to_string()))
    }}
