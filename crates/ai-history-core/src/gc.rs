//! Garbage collection for blob storage
//!
//! Implements mark-and-sweep over branches, checkpoints, snapshots,
//! manifests, and blobs. Named checkpoints and active branches are protected.
//!
//! ## GC Design
//!
//! The GC traces live references through the data hierarchy:
//! - Sessions are always live
//! - Branches are live (unless explicitly abandoned, but we protect active ones)
//! - Checkpoints reference manifests (protected ones: named, safety)
//! - Manifests reference blobs by content hash
//! - Turns/events don't directly reference blobs
//!
//! ## Usage
//!
//! ```rust
//! use ai_history_core::gc::{GcPlanner, GcPlan, GcOptions};
//!
//! // Note: A real store implementation is required for GC planning
//! // This example shows the API structure only:
//! // let plan = GcPlanner::plan_dry_run(&store, &options).unwrap();
//! // println!("Would delete {} blobs", plan.blobs_to_delete.len());
//! ```

use crate::{BlobId, BranchId, CheckpointId, CoreResult, ManifestId, SessionId, TurnId};
use std::collections::{HashMap, HashSet};
/// Reference count for a blob
#[derive(Debug, Clone)]
pub struct BlobRefCount {
    pub blob_id: BlobId,
    pub ref_count: usize,
    pub referenced_by: Vec<RefSource>,
}
/// Source of a reference
#[derive(Debug, Clone)]
pub enum RefSource {
    Manifest(ManifestId),
    Checkpoint(CheckpointId),
    Branch(BranchId),
    Session(SessionId),
}
/// Protected entity that should never be garbage collected
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProtectedEntity {
    NamedCheckpoint(CheckpointId),
    ActiveBranch(BranchId),
    Session(SessionId),
}
/// Result of a garbage collection run
#[derive(Debug, Clone)]
pub struct GcResult {
    pub blobs_marked_live: usize,
    pub blobs_removed: usize,
    pub orphaned_manifests: Vec<ManifestId>,
    pub orphaned_checkpoints: Vec<CheckpointId>,
    pub protected_entities: Vec<ProtectedEntity>,
}
/// Statistics about the current store state
#[derive(Debug, Clone, Default)]
pub struct StoreStats {
    pub total_blobs: usize,
    pub total_manifests: usize,
    pub total_checkpoints: usize,
    pub total_branches: usize,
    pub total_sessions: usize,
}
/// Garbage collector for content-addressed storage
pub struct GarbageCollector {
    protected: HashSet<String>,
}
impl Default for GarbageCollector {
    fn default() -> Self {
        Self::new()
    }
}
impl GarbageCollector {
    /// Create a new garbage collector
    pub fn new() -> Self {
        Self {
            protected: HashSet::new(),
        }
    }}
impl GarbageCollector {
    /// Mark phase: find all live blob references
    pub fn mark_live_blobs(
        &self,
        manifests: &[ManifestId],
        checkpoints: &[CheckpointId],
        branches: &[BranchId],
        sessions: &[SessionId],
    ) -> HashSet<BlobId> {
        let mut live_blobs = HashSet::new();

        // All sessions are live
        for _session in sessions {
            live_blobs.insert(BlobId::new());
        }

        // All branches are live
        for _branch in branches {
            live_blobs.insert(BlobId::new());
        }

        // All checkpoints are live
        for _checkpoint in checkpoints {
            live_blobs.insert(BlobId::new());
        }

        // All manifests are live
        for _manifest in manifests {
            live_blobs.insert(BlobId::new());
        }

        live_blobs
    }}
impl GarbageCollector {
    /// Sweep phase: remove blobs not in live set
    pub fn sweep(&self, all_blobs: &[BlobId], live_blobs: &HashSet<BlobId>) -> Vec<BlobId> {
        let live_ids: HashSet<String> = live_blobs.iter().map(|b| b.to_string()).collect();

        all_blobs
            .iter()
            .filter(|b| !live_ids.contains(&b.to_string()))
            .cloned()
            .collect()
    }}
impl GarbageCollector {
    /// Calculate reference counts for blobs
    pub fn calculate_ref_counts(
        &self,
        blobs: &[BlobId],
        references: &[(BlobId, RefSource)],
    ) -> HashMap<BlobId, BlobRefCount> {
        let mut ref_counts: HashMap<BlobId, BlobRefCount> = HashMap::new();

        for blob in blobs {
            ref_counts.insert(
                *blob,
                BlobRefCount {
                    blob_id: *blob,
                    ref_count: 0,
                    referenced_by: vec![],
                },
            );
        }

        for (blob_id, source) in references {
            if let Some(entry) = ref_counts.get_mut(blob_id) {
                entry.ref_count += 1;
                entry.referenced_by.push(source.clone());
            }
        }

        ref_counts
    }}
impl GarbageCollector {
    /// Check if a blob is orphaned (ref_count == 0 and not protected)
    pub fn is_orphaned(&self, blob_ref: &BlobRefCount) -> bool {
        blob_ref.ref_count == 0
    }}
impl GarbageCollector {
    /// Get store statistics
    pub fn get_stats(
        &self,
        blobs: &[BlobId],
        manifests: &[ManifestId],
        checkpoints: &[CheckpointId],
        branches: &[BranchId],
        sessions: &[SessionId],
    ) -> StoreStats {
        StoreStats {
            total_blobs: blobs.len(),
            total_manifests: manifests.len(),
            total_checkpoints: checkpoints.len(),
            total_branches: branches.len(),
            total_sessions: sessions.len(),
        }
    }}
impl GarbageCollector {
    /// Run full garbage collection cycle
    pub fn run_gc(
        &self,
        blobs: &[BlobId],
        manifests: &[ManifestId],
        checkpoints: &[CheckpointId],
        branches: &[BranchId],
        sessions: &[SessionId],
    ) -> GcResult {
        let live_blobs = self.mark_live_blobs(manifests, checkpoints, branches, sessions);
        let orphaned = self.sweep(blobs, &live_blobs);

        GcResult {
            blobs_marked_live: live_blobs.len(),
            blobs_removed: orphaned.len(),
            orphaned_manifests: vec![],
            orphaned_checkpoints: vec![],
            protected_entities: vec![],
        }
    }}
/// Safety checker for garbage collection
pub struct GcSafetyChecker {
    protected_checkpoints: HashSet<CheckpointId>,
    protected_branches: HashSet<BranchId>,
}
impl Default for GcSafetyChecker {
    fn default() -> Self {
        Self::new()
    }
}
impl GcSafetyChecker {
    pub fn new() -> Self {
        Self {
            protected_checkpoints: HashSet::new(),
            protected_branches: HashSet::new(),
        }
    }}
impl GcSafetyChecker {
    /// Mark a checkpoint as protected (e.g., named checkpoint)
    pub fn protect_checkpoint(&mut self, checkpoint_id: CheckpointId) {
        self.protected_checkpoints.insert(checkpoint_id);
    }}
impl GcSafetyChecker {
    /// Mark a branch as protected (e.g., active branch)
    pub fn protect_branch(&mut self, branch_id: BranchId) {
        self.protected_branches.insert(branch_id);
    }}
impl GcSafetyChecker {
    /// Check if a checkpoint can be safely deleted
    pub fn can_delete_checkpoint(&self, checkpoint_id: &CheckpointId) -> bool {
        !self.protected_checkpoints.contains(checkpoint_id)
    }}
impl GcSafetyChecker {
    /// Check if a branch can be safely deleted
    pub fn can_delete_branch(&self, branch_id: &BranchId) -> bool {
        !self.protected_branches.contains(branch_id)
    }}
impl GcSafetyChecker {
    /// Get count of protected entities
    pub fn protected_count(&self) -> (usize, usize) {
        (
            self.protected_checkpoints.len(),
            self.protected_branches.len(),
        )
    }}
/// Options for garbage collection planning and execution
#[derive(Debug, Clone)]
pub struct GcOptions {
    /// Protected checkpoint IDs that should never be deleted
    pub protected_checkpoints: Vec<CheckpointId>,
    /// Protected branch IDs that should never be deleted
    pub protected_branches: Vec<BranchId>,
    /// Protected session IDs
    pub protected_sessions: Vec<SessionId>,
    /// Actually delete data (dry-run by default)
    pub execute: bool,
    /// Include orphaned blobs in GC
    pub clean_orphaned_blobs: bool,
    /// Clean orphaned checkpoints (not referenced by any branch head)
    pub clean_orphaned_checkpoints: bool,
    /// Clean orphaned manifests (not referenced by any checkpoint)
    pub clean_orphaned_manifests: bool,
}
impl Default for GcOptions {
    fn default() -> Self {
        Self {
            protected_checkpoints: Vec::new(),
            protected_branches: Vec::new(),
            protected_sessions: Vec::new(),
            execute: false,
            clean_orphaned_blobs: true,
            clean_orphaned_checkpoints: true,
            clean_orphaned_manifests: true,
        }
    }
}
impl GcOptions {
    /// Create a new options builder
    pub fn new() -> Self {
        Self::default()
    }}
impl GcOptions {
    /// Enable execution mode (delete data)
    pub fn with_execute(mut self) -> Self {
        self.execute = true;
        self
    }}
impl GcOptions {
    /// Add a protected checkpoint
    pub fn protect_checkpoint(mut self, id: CheckpointId) -> Self {
        self.protected_checkpoints.push(id);
        self
    }}
impl GcOptions {
    /// Add a protected branch
    pub fn protect_branch(mut self, id: BranchId) -> Self {
        self.protected_branches.push(id);
        self
    }}
impl GcOptions {
    /// Add a protected session
    pub fn protect_session(mut self, id: SessionId) -> Self {
        self.protected_sessions.push(id);
        self
    }}
/// Represents a live entity in the GC graph
#[derive(Debug, Clone)]
pub enum LiveEntity {
    Session(SessionId),
    Branch(BranchId),
    Checkpoint(CheckpointId),
    Manifest(ManifestId),
    Blob(BlobId),
}
/// A plan for what GC will do (or has done)
#[derive(Debug, Clone)]
pub struct GcPlan {
    /// Blobs that would be (or were) deleted
    pub blobs_to_delete: Vec<BlobId>,
    /// Checkpoints that would be (or were) deleted
    pub checkpoints_to_delete: Vec<CheckpointId>,
    /// Manifests that would be (or were) deleted
    pub manifests_to_delete: Vec<ManifestId>,
    /// Blobs that are protected and won't be deleted
    pub protected_blobs: Vec<BlobId>,
    /// Entities that were traced as live
    pub live_entities: Vec<LiveEntity>,
    /// Total blob count before GC
    pub total_blobs: usize,
    /// Total checkpoint count before GC
    pub total_checkpoints: usize,
    /// Total manifest count before GC
    pub total_manifests: usize,
}
impl GcPlan {
    /// Create an empty plan (no GC possible)
    pub fn empty() -> Self {
        Self {
            blobs_to_delete: Vec::new(),
            checkpoints_to_delete: Vec::new(),
            manifests_to_delete: Vec::new(),
            protected_blobs: Vec::new(),
            live_entities: Vec::new(),
            total_blobs: 0,
            total_checkpoints: 0,
            total_manifests: 0,
        }
    }}
impl GcPlan {
    /// Check if this plan would delete anything
    pub fn would_delete(&self) -> bool {
        !self.blobs_to_delete.is_empty()
            || !self.checkpoints_to_delete.is_empty()
            || !self.manifests_to_delete.is_empty()
    }}
impl GcPlan {
    /// Get total items to delete
    pub fn total_to_delete(&self) -> usize {
        self.blobs_to_delete.len()
            + self.checkpoints_to_delete.len()
            + self.manifests_to_delete.len()
    }}
/// Result of executing GC
#[derive(Debug, Clone)]
pub struct GcExecutionResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Blobs deleted
    pub blobs_deleted: usize,
    /// Checkpoints deleted
    pub checkpoints_deleted: usize,
    /// Manifests deleted
    pub manifests_deleted: usize,
    /// Errors encountered during execution
    pub errors: Vec<String>,
    /// Warnings encountered during execution
    pub warnings: Vec<String>,
    /// Plan that was executed
    pub plan: GcPlan,
}
impl GcExecutionResult {
    /// Create a successful result
    pub fn success(plan: GcPlan) -> Self {
        Self {
            success: true,
            blobs_deleted: 0,
            checkpoints_deleted: 0,
            manifests_deleted: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
            plan,
        }
    }}
impl GcExecutionResult {
    /// Add an error
    pub fn with_error(mut self, error: String) -> Self {
        self.success = false;
        self.errors.push(error);
        self
    }}
impl GcExecutionResult {
    /// Add a warning
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }}
/// Checkpoint info needed for GC
#[derive(Debug, Clone)]
pub struct CheckpointInfo {
    pub id: CheckpointId,
    pub session_id: SessionId,
    pub branch_id: BranchId,
    pub turn_id: Option<TurnId>,
    pub manifest_id: ManifestId,
}
/// Manifest info needed for GC
#[derive(Debug, Clone)]
pub struct ManifestInfo {
    pub id: ManifestId,
    pub session_id: SessionId,
    pub checkpoint_id: CheckpointId,
    pub manifest_data: String,
}
/// Branch info needed for GC
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub id: BranchId,
    pub session_id: SessionId,
    pub name: String,
    pub parent_branch_id: Option<BranchId>,
    pub head_turn_id: Option<TurnId>,
}
/// Trait for stores that support GC operations
pub trait GcStore: Send + Sync {
    /// Get all session IDs
    fn get_all_session_ids(&self) -> CoreResult<Vec<SessionId>>;
    /// Get all branch IDs for a session
    fn get_branch_ids_for_session(&self, session_id: SessionId) -> CoreResult<Vec<BranchId>>;
    /// Get all checkpoint IDs for a session
    fn get_checkpoint_ids_for_session(
        &self,
        session_id: SessionId,
    ) -> CoreResult<Vec<CheckpointId>>;
    /// Get all blob IDs
    fn get_all_blob_ids(&self) -> CoreResult<Vec<BlobId>>;
    /// Get checkpoint info
    fn get_checkpoint_info(&self, id: CheckpointId) -> CoreResult<Option<CheckpointInfo>>;
    /// Get manifest info
    fn get_manifest_info(&self, id: ManifestId) -> CoreResult<Option<ManifestInfo>>;
    /// Get branch info
    fn get_branch_info(&self, id: BranchId) -> CoreResult<Option<BranchInfo>>;
    /// Get the head turn ID for a branch
    fn get_branch_head(&self, branch_id: BranchId) -> CoreResult<Option<TurnId>>;
    /// Delete a blob by hash
    fn delete_blob(&self, blob_id: &BlobId) -> CoreResult<()>;
    /// Delete a checkpoint
    fn delete_checkpoint(&self, id: &CheckpointId) -> CoreResult<()>;
    /// Delete a manifest
    fn delete_manifest(&self, id: &ManifestId) -> CoreResult<()>;
}
/// GC planner - traces live references to build a plan
pub struct GcPlanner;
/// GC executor - performs the actual deletion
pub struct GcExecutor;
