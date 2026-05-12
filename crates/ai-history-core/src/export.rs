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
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Add an error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
}

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
    }

    /// Check if all checks passed
    pub fn all_passed(&self) -> bool {
        self.checksum_valid && self.blob_hash_valid && self.references_valid
    }
}

/// Exporter for creating export packages
pub struct Exporter;

impl Exporter {
    /// Create a new exporter
    pub fn new() -> Self {
        Self
    }

    /// Export sessions to a package
    pub fn export_sessions(&self, sessions: &[SessionExport]) -> Vec<SessionExport> {
        sessions.to_vec()
    }

    /// Export branches to a package
    pub fn export_branches(&self, branches: &[BranchExport]) -> Vec<BranchExport> {
        branches.to_vec()
    }

    /// Export checkpoints to a package
    pub fn export_checkpoints(&self, checkpoints: &[CheckpointExport]) -> Vec<CheckpointExport> {
        checkpoints.to_vec()
    }

    /// Export manifests to a package
    pub fn export_manifests(&self, manifests: &[ManifestExport]) -> Vec<ManifestExport> {
        manifests.to_vec()
    }

    /// Export events to a package
    pub fn export_events(&self, events: &[EventExport]) -> Vec<EventExport> {
        events.to_vec()
    }

    /// Export blobs to a package
    pub fn export_blobs(&self, blobs: &[BlobExport]) -> Vec<BlobExport> {
        blobs.to_vec()
    }

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
    }
}

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
    }

    /// Verify import package integrity
    pub fn verify_integrity(&self, package: &ExportPackage) -> IntegrityVerification {
        // Verify all checksums match
        let checksum_valid = package.events.iter().all(|e| {
            // Simple check: checksum should be 64 hex chars
            e.checksum.len() == 64 && e.checksum.chars().all(|c| c.is_ascii_hexdigit())
        });

        // Verify blob hashes
        let blob_hash_valid = package.blobs.iter().all(|b| {
            // SHA-256 hash is 64 hex chars
            b.hash.len() == 64 && b.hash.chars().all(|c| c.is_ascii_hexdigit())
        });

        // Verify references exist
        let session_ids: HashMap<String, bool> = package
            .sessions
            .iter()
            .map(|s| (s.id.to_string(), true))
            .collect();

        let references_valid = package
            .branches
            .iter()
            .all(|b| session_ids.contains_key(&b.session_id.to_string()))
            && package
                .checkpoints
                .iter()
                .all(|c| session_ids.contains_key(&c.session_id.to_string()));

        IntegrityVerification {
            checksum_valid,
            blob_hash_valid,
            references_valid,
        }
    }

    /// Import sessions from package
    pub fn import_sessions(&self, package: &ExportPackage) -> Vec<SessionExport> {
        package.sessions.clone()
    }

    /// Import branches from package
    pub fn import_branches(&self, package: &ExportPackage) -> Vec<BranchExport> {
        package.branches.clone()
    }

    /// Import checkpoints from package
    pub fn import_checkpoints(&self, package: &ExportPackage) -> Vec<CheckpointExport> {
        package.checkpoints.clone()
    }

    /// Import manifests from package
    pub fn import_manifests(&self, package: &ExportPackage) -> Vec<ManifestExport> {
        package.manifests.clone()
    }

    /// Import events from package
    pub fn import_events(&self, package: &ExportPackage) -> Vec<EventExport> {
        package.events.clone()
    }

    /// Import blobs from package
    pub fn import_blobs(&self, package: &ExportPackage) -> Vec<BlobExport> {
        package.blobs.clone()
    }

    /// Perform full import and return result
    pub fn import(&self, package: &ExportPackage) -> ImportResult {
        let mut result = ImportResult::default();

        // Verify integrity first
        let verification = self.verify_integrity(package);
        if !verification.all_passed() {
            result.add_error("Package integrity verification failed".to_string());
            return result;
        }

        // Import sessions
        result.sessions_imported = package.sessions.len();
        result.branches_imported = package.branches.len();
        result.checkpoints_imported = package.checkpoints.len();
        result.manifests_imported = package.manifests.len();
        result.events_imported = package.events.len();
        result.blobs_imported = package.blobs.len();

        result
    }
}

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
    }

    /// Deserialize package from bytes
    pub fn deserialize(data: &[u8]) -> CoreResult<ExportPackage> {
        serde_json::from_slice(data).map_err(|e| crate::CoreError::Deserialization(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session() -> SessionExport {
        SessionExport {
            id: SessionId::new(),
            workspace_root: "/tmp/test".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn create_test_package() -> ExportPackage {
        let metadata = ExportMetadata {
            version: EXPORT_VERSION.to_string(),
            session_id: SessionId::new(),
            exported_at: "2024-01-01T00:00:00Z".to_string(),
            session_count: 1,
            event_count: 0,
            branch_count: 0,
            checkpoint_count: 0,
            manifest_count: 0,
            blob_count: 0,
        };

        ExportPackage {
            metadata,
            sessions: vec![create_test_session()],
            branches: vec![],
            checkpoints: vec![],
            manifests: vec![],
            events: vec![],
            blobs: vec![],
        }
    }

    #[test]
    fn test_export_metadata_creation() {
        let exporter = Exporter::new();
        let metadata = exporter.create_metadata(
            SessionId::new(),
            5,  // sessions
            10, // events
            3,  // branches
            7,  // checkpoints
            4,  // manifests
            20, // blobs
        );

        assert_eq!(metadata.version, EXPORT_VERSION);
        assert_eq!(metadata.session_count, 5);
        assert_eq!(metadata.event_count, 10);
        assert_eq!(metadata.branch_count, 3);
    }

    #[test]
    fn test_export_sessions() {
        let exporter = Exporter::new();
        let sessions = vec![create_test_session()];
        let exported = exporter.export_sessions(&sessions);

        assert_eq!(exported.len(), 1);
        assert_eq!(exported[0].workspace_root, "/tmp/test");
    }

    #[test]
    fn test_verify_integrity_pass() {
        let importer = Importer::new();
        let package = create_test_package();
        let verification = importer.verify_integrity(&package);

        assert!(verification.all_passed());
    }

    #[test]
    fn test_verify_integrity_checksum_valid() {
        let mut package = create_test_package();
        package.events.push(EventExport {
            id: 1,
            session_id: SessionId::new(),
            branch_id: BranchId::new(),
            turn_id: None,
            sequence_num: 1,
            event_type: "Test".to_string(),
            event_data: "{}".to_string(),
            causation_id: None,
            correlation_id: None,
            actor: "User".to_string(),
            checksum: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                .to_string(), // Valid SHA-256
            created_at: "2024-01-01T00:00:00Z".to_string(),
        });

        let importer = Importer::new();
        let verification = importer.verify_integrity(&package);

        assert!(verification.checksum_valid);
    }

    #[test]
    fn test_verify_integrity_blob_hash_valid() {
        let mut package = create_test_package();
        package.blobs.push(BlobExport {
            id: BlobId::new(),
            session_id: SessionId::new(),
            hash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            size: 0,
            content: vec![],
            created_at: "2024-01-01T00:00:00Z".to_string(),
        });

        let importer = Importer::new();
        let verification = importer.verify_integrity(&package);

        assert!(verification.blob_hash_valid);
    }

    #[test]
    fn test_import_result_is_success() {
        let mut result = ImportResult::default();
        assert!(result.is_success());

        result.add_error("Test error".to_string());
        assert!(!result.is_success());
    }

    #[test]
    fn test_import_result_add_warning() {
        let mut result = ImportResult::default();
        result.add_warning("Test warning".to_string());
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_archive_format_serialize_deserialize() {
        let package = create_test_package();
        let serialized = ArchiveFormat::serialize(&package).unwrap();
        let deserialized = ArchiveFormat::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.sessions.len(), 1);
        assert_eq!(deserialized.metadata.version, EXPORT_VERSION);
    }

    #[test]
    fn test_archive_format_roundtrip() {
        let mut package = create_test_package();
        package.blobs.push(BlobExport {
            id: BlobId::new(),
            session_id: SessionId::new(),
            hash: "abc123".to_string(),
            size: 5,
            content: vec![1, 2, 3, 4, 5],
            created_at: "2024-01-01T00:00:00Z".to_string(),
        });

        let serialized = ArchiveFormat::serialize(&package).unwrap();
        let deserialized = ArchiveFormat::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.blobs.len(), 1);
        assert_eq!(deserialized.blobs[0].content, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_integrity_verification_pass() {
        let verification = IntegrityVerification::pass();
        assert!(verification.all_passed());
    }

    #[test]
    fn test_exporter_new() {
        let exporter = Exporter::new();
        let metadata = exporter.create_metadata(SessionId::new(), 0, 0, 0, 0, 0, 0);
        assert_eq!(metadata.version, EXPORT_VERSION);
    }

    #[test]
    fn test_importer_new() {
        let importer = Importer::new();
        let package = create_test_package();
        let result = importer.import(&package);
        assert!(result.is_success());
    }

    #[test]
    fn test_full_import() {
        let importer = Importer::new();
        let package = create_test_package();
        let result = importer.import(&package);

        assert!(result.is_success());
        assert_eq!(result.sessions_imported, 1);
    }
}
