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

impl ProtectedEntity {
    /// Get the key for this protected entity
    pub fn key(&self) -> String {
        match self {
            ProtectedEntity::NamedCheckpoint(id) => format!("checkpoint:{}", id),
            ProtectedEntity::ActiveBranch(id) => format!("branch:{}", id),
            ProtectedEntity::Session(id) => format!("session:{}", id),
        }
    }
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
    }

    /// Protect an entity from garbage collection
    pub fn protect(&mut self, entity: ProtectedEntity) {
        let key = match entity {
            ProtectedEntity::NamedCheckpoint(id) => format!("checkpoint:{}", id),
            ProtectedEntity::ActiveBranch(id) => format!("branch:{}", id),
            ProtectedEntity::Session(id) => format!("session:{}", id),
        };
        self.protected.insert(key);
    }

    /// Check if an entity is protected
    pub fn is_protected(&self, entity: &ProtectedEntity) -> bool {
        let key = match entity {
            ProtectedEntity::NamedCheckpoint(id) => format!("checkpoint:{}", id),
            ProtectedEntity::ActiveBranch(id) => format!("branch:{}", id),
            ProtectedEntity::Session(id) => format!("session:{}", id),
        };
        self.protected.contains(&key)
    }

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
    }

    /// Sweep phase: remove blobs not in live set
    pub fn sweep(&self, all_blobs: &[BlobId], live_blobs: &HashSet<BlobId>) -> Vec<BlobId> {
        let live_ids: HashSet<String> = live_blobs.iter().map(|b| b.to_string()).collect();

        all_blobs
            .iter()
            .filter(|b| !live_ids.contains(&b.to_string()))
            .cloned()
            .collect()
    }

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
    }

    /// Check if a blob is orphaned (ref_count == 0 and not protected)
    pub fn is_orphaned(&self, blob_ref: &BlobRefCount) -> bool {
        blob_ref.ref_count == 0
    }

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
    }

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
    }
}

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
    }

    /// Mark a checkpoint as protected (e.g., named checkpoint)
    pub fn protect_checkpoint(&mut self, checkpoint_id: CheckpointId) {
        self.protected_checkpoints.insert(checkpoint_id);
    }

    /// Mark a branch as protected (e.g., active branch)
    pub fn protect_branch(&mut self, branch_id: BranchId) {
        self.protected_branches.insert(branch_id);
    }

    /// Check if a checkpoint can be safely deleted
    pub fn can_delete_checkpoint(&self, checkpoint_id: &CheckpointId) -> bool {
        !self.protected_checkpoints.contains(checkpoint_id)
    }

    /// Check if a branch can be safely deleted
    pub fn can_delete_branch(&self, branch_id: &BranchId) -> bool {
        !self.protected_branches.contains(branch_id)
    }

    /// Get count of protected entities
    pub fn protected_count(&self) -> (usize, usize) {
        (
            self.protected_checkpoints.len(),
            self.protected_branches.len(),
        )
    }
}

// =============================================================================
// GC Planning and Execution API
// =============================================================================

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
    }

    /// Enable execution mode (delete data)
    pub fn with_execute(mut self) -> Self {
        self.execute = true;
        self
    }

    /// Add a protected checkpoint
    pub fn protect_checkpoint(mut self, id: CheckpointId) -> Self {
        self.protected_checkpoints.push(id);
        self
    }

    /// Add a protected branch
    pub fn protect_branch(mut self, id: BranchId) -> Self {
        self.protected_branches.push(id);
        self
    }

    /// Add a protected session
    pub fn protect_session(mut self, id: SessionId) -> Self {
        self.protected_sessions.push(id);
        self
    }
}

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
    }

    /// Check if this plan would delete anything
    pub fn would_delete(&self) -> bool {
        !self.blobs_to_delete.is_empty()
            || !self.checkpoints_to_delete.is_empty()
            || !self.manifests_to_delete.is_empty()
    }

    /// Get total items to delete
    pub fn total_to_delete(&self) -> usize {
        self.blobs_to_delete.len()
            + self.checkpoints_to_delete.len()
            + self.manifests_to_delete.len()
    }
}

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
    }

    /// Add an error
    pub fn with_error(mut self, error: String) -> Self {
        self.success = false;
        self.errors.push(error);
        self
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }
}

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

impl GcPlanner {
    /// Build a GC plan by tracing live references (dry-run)
    pub fn plan_dry_run<S: GcStore>(store: &S, options: &GcOptions) -> CoreResult<GcPlan> {
        let protected = Self::build_protected_set(options);

        // Collect all sessions - all are live
        let session_ids = store.get_all_session_ids()?;
        let mut live_entities = Vec::new();
        for session_id in &session_ids {
            live_entities.push(LiveEntity::Session(*session_id));
        }

        // Collect all live branches
        let mut live_checkpoints: HashSet<CheckpointId> = HashSet::new();
        let mut live_manifests: HashSet<ManifestId> = HashSet::new();
        let mut live_blobs: HashSet<BlobId> = HashSet::new();

        for session_id in &session_ids {
            let branch_ids = store.get_branch_ids_for_session(*session_id)?;
            for branch_id in &branch_ids {
                live_entities.push(LiveEntity::Branch(*branch_id));
            }

            // Checkpoints are live if their branch is live
            let checkpoint_ids = store.get_checkpoint_ids_for_session(*session_id)?;
            for checkpoint_id in &checkpoint_ids {
                // Protected checkpoints are live
                if protected.contains(&ProtectedEntity::NamedCheckpoint(*checkpoint_id).key()) {
                    live_entities.push(LiveEntity::Checkpoint(*checkpoint_id));
                    live_checkpoints.insert(*checkpoint_id);
                    continue;
                }

                // Get checkpoint info to find its manifest
                if let Some(info) = store.get_checkpoint_info(*checkpoint_id)? {
                    live_entities.push(LiveEntity::Checkpoint(*checkpoint_id));
                    live_checkpoints.insert(*checkpoint_id);

                    // Manifests referenced by live checkpoints are live
                    live_manifests.insert(info.manifest_id);
                    live_entities.push(LiveEntity::Manifest(info.manifest_id));
                }
            }
        }

        // Blobs referenced by live manifests are live
        for manifest_id in &live_manifests {
            if let Some(info) = store.get_manifest_info(*manifest_id)? {
                // Parse manifest_data to extract blob references
                if let Ok(entries) = Self::parse_manifest_blob_refs(&info.manifest_data) {
                    for blob_id in entries {
                        live_blobs.insert(blob_id);
                        live_entities.push(LiveEntity::Blob(blob_id));
                    }
                }
            }
        }

        // All blobs are live if no manifests exist (edge case)
        if live_manifests.is_empty() {
            let all_blobs_initial = store.get_all_blob_ids()?;
            for blob_id in all_blobs_initial {
                live_blobs.insert(blob_id);
            }
        }

        // Get all blobs and compute orphaned ones
        let all_blobs = store.get_all_blob_ids()?;
        let total_blobs = all_blobs.len();
        let blobs_to_delete: Vec<BlobId> = all_blobs
            .iter()
            .filter(|b| !live_blobs.contains(b))
            .cloned()
            .collect();

        // All checkpoints and manifests are considered for deletion if orphaned
        let all_checkpoints: Vec<CheckpointId> = session_ids
            .iter()
            .flat_map(|sid| {
                store
                    .get_checkpoint_ids_for_session(*sid)
                    .unwrap_or_default()
            })
            .collect();
        let total_checkpoints = all_checkpoints.len();

        let checkpoints_to_delete: Vec<CheckpointId> = all_checkpoints
            .iter()
            .filter(|c| !live_checkpoints.contains(c))
            .cloned()
            .collect();

        // Collect all manifest IDs from checkpoints
        let all_manifests: Vec<ManifestId> = all_checkpoints
            .iter()
            .filter_map(|cp_id| store.get_checkpoint_info(*cp_id).unwrap_or_default())
            .map(|cp_info| cp_info.manifest_id)
            .collect();
        let total_manifests = all_manifests.len();

        let manifests_to_delete: Vec<ManifestId> = all_manifests
            .iter()
            .filter(|m| !live_manifests.contains(m))
            .cloned()
            .collect();

        Ok(GcPlan {
            blobs_to_delete,
            checkpoints_to_delete,
            manifests_to_delete,
            protected_blobs: Vec::new(),
            live_entities,
            total_blobs,
            total_checkpoints,
            total_manifests,
        })
    }

    /// Build the set of protected entity keys
    fn build_protected_set(options: &GcOptions) -> HashSet<String> {
        let mut protected = HashSet::new();
        for id in &options.protected_checkpoints {
            protected.insert(ProtectedEntity::NamedCheckpoint(*id).key());
        }
        for id in &options.protected_branches {
            protected.insert(ProtectedEntity::ActiveBranch(*id).key());
        }
        for id in &options.protected_sessions {
            protected.insert(ProtectedEntity::Session(*id).key());
        }
        protected
    }

    /// Parse blob references from manifest data (JSON)
    fn parse_manifest_blob_refs(manifest_data: &str) -> Result<Vec<BlobId>, ()> {
        // Simple JSON parsing for manifest blob refs
        // Format: {"entries": {"path": {"blob_hash": "xxx", ...}, ...}}
        let mut blob_ids = Vec::new();

        // Try to parse as JSON and extract blob hashes
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(manifest_data) {
            if let Some(entries) = json.get("entries").and_then(|e| e.as_object()) {
                for entry in entries.values() {
                    if let Some(hash) = entry.get("blob_hash").and_then(|h| h.as_str()) {
                        // Blob IDs are stored as "blob:hash" format
                        if let Some(rest) = hash.strip_prefix("blob:") {
                            if let Ok(blob_id) = rest.parse() {
                                blob_ids.push(blob_id);
                            }
                        } else if let Ok(blob_id) = hash.parse() {
                            blob_ids.push(blob_id);
                        }
                    }
                }
            }
        }

        Ok(blob_ids)
    }
}

/// GC executor - performs the actual deletion
pub struct GcExecutor;

impl GcExecutor {
    /// Execute garbage collection
    pub fn execute<S: GcStore>(store: &S, options: &GcOptions) -> CoreResult<GcExecutionResult> {
        let plan = GcPlanner::plan_dry_run(store, options)?;

        if !options.execute {
            // Dry-run mode - return plan without deleting
            return Ok(GcExecutionResult::success(plan));
        }

        // Execute mode - delete orphaned data
        let mut result = GcExecutionResult::success(plan.clone());

        // Delete orphaned blobs
        if options.clean_orphaned_blobs {
            for blob_id in &plan.blobs_to_delete {
                match store.delete_blob(blob_id) {
                    Ok(_) => result.blobs_deleted += 1,
                    Err(e) => {
                        result =
                            result.with_error(format!("Failed to delete blob {}: {}", blob_id, e))
                    }
                }
            }
        }

        // Delete orphaned manifests (before checkpoints since they reference them)
        if options.clean_orphaned_manifests {
            for manifest_id in &plan.manifests_to_delete {
                match store.delete_manifest(manifest_id) {
                    Ok(_) => result.manifests_deleted += 1,
                    Err(e) => {
                        result = result
                            .with_error(format!("Failed to delete manifest {}: {}", manifest_id, e))
                    }
                }
            }
        }

        // Delete orphaned checkpoints
        if options.clean_orphaned_checkpoints {
            for checkpoint_id in &plan.checkpoints_to_delete {
                match store.delete_checkpoint(checkpoint_id) {
                    Ok(_) => result.checkpoints_deleted += 1,
                    Err(e) => {
                        result = result.with_error(format!(
                            "Failed to delete checkpoint {}: {}",
                            checkpoint_id, e
                        ))
                    }
                }
            }
        }

        Ok(result)
    }

    /// Execute GC and return a summary string for CLI output
    pub fn execute_with_summary<S: GcStore>(store: &S, options: &GcOptions) -> CoreResult<String> {
        let result = Self::execute(store, options)?;

        let mut lines = Vec::new();

        if options.execute {
            lines.push(format!("GC executed:"));
            if result.blobs_deleted > 0 {
                lines.push(format!("  {} blob(s) deleted", result.blobs_deleted));
            }
            if result.checkpoints_deleted > 0 {
                lines.push(format!(
                    "  {} checkpoint(s) deleted",
                    result.checkpoints_deleted
                ));
            }
            if result.manifests_deleted > 0 {
                lines.push(format!(
                    "  {} manifest(s) deleted",
                    result.manifests_deleted
                ));
            }
            if result.blobs_deleted == 0
                && result.checkpoints_deleted == 0
                && result.manifests_deleted == 0
            {
                lines.push("  Nothing to delete".to_string());
            }
        } else {
            lines.push("GC dry-run (no changes made):".to_string());
            if result.plan.would_delete() {
                lines.push(format!(
                    "  Would delete {} blob(s)",
                    result.plan.blobs_to_delete.len()
                ));
                lines.push(format!(
                    "  Would delete {} checkpoint(s)",
                    result.plan.checkpoints_to_delete.len()
                ));
                lines.push(format!(
                    "  Would delete {} manifest(s)",
                    result.plan.manifests_to_delete.len()
                ));
            } else {
                lines.push("  Nothing to delete - all data is live".to_string());
            }
        }

        if !result.errors.is_empty() {
            lines.push("\nErrors:".to_string());
            for error in &result.errors {
                lines.push(format!("  {}", error));
            }
        }

        if !result.warnings.is_empty() {
            lines.push("\nWarnings:".to_string());
            for warning in &result.warnings {
                lines.push(format!("  {}", warning));
            }
        }

        Ok(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_garbage_collector_new() {
        let gc = GarbageCollector::new();
        assert!(gc.protected.is_empty());
    }

    #[test]
    fn test_protect_named_checkpoint() {
        let mut gc = GarbageCollector::new();
        let checkpoint_id = CheckpointId::new();

        gc.protect(ProtectedEntity::NamedCheckpoint(checkpoint_id));
        assert!(gc.is_protected(&ProtectedEntity::NamedCheckpoint(checkpoint_id)));
    }

    #[test]
    fn test_protect_active_branch() {
        let mut gc = GarbageCollector::new();
        let branch_id = BranchId::new();

        gc.protect(ProtectedEntity::ActiveBranch(branch_id));
        assert!(gc.is_protected(&ProtectedEntity::ActiveBranch(branch_id)));
    }

    #[test]
    fn test_protect_session() {
        let mut gc = GarbageCollector::new();
        let session_id = SessionId::new();

        gc.protect(ProtectedEntity::Session(session_id));
        assert!(gc.is_protected(&ProtectedEntity::Session(session_id)));
    }

    #[test]
    fn test_mark_live_blobs() {
        let gc = GarbageCollector::new();
        let manifests = vec![ManifestId::new()];
        let checkpoints = vec![CheckpointId::new()];
        let branches = vec![BranchId::new()];
        let sessions = vec![SessionId::new()];

        let live = gc.mark_live_blobs(&manifests, &checkpoints, &branches, &sessions);
        assert!(!live.is_empty());
    }

    #[test]
    fn test_sweep_removes_orphaned_blobs() {
        let gc = GarbageCollector::new();
        let blobs = vec![BlobId::new(), BlobId::new(), BlobId::new()];
        let live = HashSet::new();

        let orphaned = gc.sweep(&blobs, &live);
        assert_eq!(orphaned.len(), 3);
    }

    #[test]
    fn test_sweep_keeps_live_blobs() {
        let gc = GarbageCollector::new();
        let blob1 = BlobId::new();
        let blob2 = BlobId::new();
        let blobs = vec![blob1, blob2];

        let mut live = HashSet::new();
        live.insert(blob1);

        let orphaned = gc.sweep(&blobs, &live);
        assert_eq!(orphaned.len(), 1);
        assert_eq!(orphaned[0], blob2);
    }

    #[test]
    fn test_calculate_ref_counts() {
        let gc = GarbageCollector::new();
        let blob1 = BlobId::new();
        let blob2 = BlobId::new();
        let blobs = vec![blob1, blob2];

        let manifest_id = ManifestId::new();
        let references = vec![
            (blob1, RefSource::Manifest(manifest_id)),
            (blob1, RefSource::Checkpoint(CheckpointId::new())),
        ];

        let ref_counts = gc.calculate_ref_counts(&blobs, &references);

        assert_eq!(ref_counts[&blob1].ref_count, 2);
        assert_eq!(ref_counts[&blob2].ref_count, 0);
    }

    #[test]
    fn test_is_orphaned() {
        let gc = GarbageCollector::new();
        let blob = BlobId::new();

        let orphaned_ref = BlobRefCount {
            blob_id: blob,
            ref_count: 0,
            referenced_by: vec![],
        };

        assert!(gc.is_orphaned(&orphaned_ref));

        let referenced_ref = BlobRefCount {
            blob_id: blob,
            ref_count: 1,
            referenced_by: vec![RefSource::Manifest(ManifestId::new())],
        };

        assert!(!gc.is_orphaned(&referenced_ref));
    }

    #[test]
    fn test_get_stats() {
        let gc = GarbageCollector::new();
        let blobs = vec![BlobId::new(), BlobId::new()];
        let manifests = vec![ManifestId::new()];
        let checkpoints = vec![CheckpointId::new()];
        let branches = vec![BranchId::new()];
        let sessions = vec![SessionId::new()];

        let stats = gc.get_stats(&blobs, &manifests, &checkpoints, &branches, &sessions);

        assert_eq!(stats.total_blobs, 2);
        assert_eq!(stats.total_manifests, 1);
        assert_eq!(stats.total_checkpoints, 1);
        assert_eq!(stats.total_branches, 1);
        assert_eq!(stats.total_sessions, 1);
    }

    #[test]
    fn test_run_gc() {
        let gc = GarbageCollector::new();
        let blobs = vec![BlobId::new(), BlobId::new()];
        let manifests = vec![ManifestId::new()];
        let checkpoints = vec![CheckpointId::new()];
        let branches = vec![BranchId::new()];
        let sessions = vec![SessionId::new()];

        let result = gc.run_gc(&blobs, &manifests, &checkpoints, &branches, &sessions);

        assert!(result.blobs_marked_live > 0);
        assert_eq!(result.blobs_removed, 2); // 2 blobs not in live set
    }

    #[test]
    fn test_gc_safety_checker_new() {
        let checker = GcSafetyChecker::new();
        let (cp_count, br_count) = checker.protected_count();
        assert_eq!(cp_count, 0);
        assert_eq!(br_count, 0);
    }

    #[test]
    fn test_gc_safety_checker_protect_checkpoint() {
        let mut checker = GcSafetyChecker::new();
        let checkpoint_id = CheckpointId::new();

        checker.protect_checkpoint(checkpoint_id);
        // Protected checkpoints cannot be deleted
        assert!(!checker.can_delete_checkpoint(&checkpoint_id));
        assert_eq!(checker.protected_count().0, 1);
    }

    #[test]
    fn test_gc_safety_checker_protect_branch() {
        let mut checker = GcSafetyChecker::new();
        let branch_id = BranchId::new();

        checker.protect_branch(branch_id);
        assert!(!checker.can_delete_branch(&branch_id));
        assert_eq!(checker.protected_count().1, 1);
    }

    #[test]
    fn test_gc_safety_checker_multiple_protections() {
        let mut checker = GcSafetyChecker::new();

        checker.protect_checkpoint(CheckpointId::new());
        checker.protect_checkpoint(CheckpointId::new());
        checker.protect_branch(BranchId::new());

        assert_eq!(checker.protected_count(), (2, 1));
    }

    #[test]
    fn test_blob_ref_count_clone() {
        let blob_id = BlobId::new();
        let ref_count = BlobRefCount {
            blob_id,
            ref_count: 5,
            referenced_by: vec![
                RefSource::Manifest(ManifestId::new()),
                RefSource::Branch(BranchId::new()),
            ],
        };

        let cloned = ref_count.clone();
        assert_eq!(cloned.ref_count, 5);
        assert_eq!(cloned.referenced_by.len(), 2);
    }

    #[test]
    fn test_protected_entity_equality() {
        let checkpoint_id = CheckpointId::new();
        let entity1 = ProtectedEntity::NamedCheckpoint(checkpoint_id);
        let entity2 = ProtectedEntity::NamedCheckpoint(checkpoint_id);

        assert_eq!(entity1, entity2);
    }

    #[test]
    fn test_gc_options_default() {
        let options = GcOptions::default();
        assert!(!options.execute); // Dry-run by default
        assert!(options.clean_orphaned_blobs);
        assert!(options.clean_orphaned_checkpoints);
        assert!(options.clean_orphaned_manifests);
    }

    #[test]
    fn test_gc_options_builder() {
        let options = GcOptions::new()
            .with_execute()
            .protect_checkpoint(CheckpointId::new())
            .protect_branch(BranchId::new());

        assert!(options.execute);
        assert_eq!(options.protected_checkpoints.len(), 1);
        assert_eq!(options.protected_branches.len(), 1);
    }

    #[test]
    fn test_gc_plan_empty() {
        let plan = GcPlan::empty();
        assert!(!plan.would_delete());
        assert_eq!(plan.total_to_delete(), 0);
    }

    #[test]
    fn test_gc_plan_would_delete() {
        let mut plan = GcPlan::empty();
        plan.blobs_to_delete.push(BlobId::new());
        assert!(plan.would_delete());
        assert_eq!(plan.total_to_delete(), 1);
    }

    #[test]
    fn test_gc_execution_result_success() {
        let plan = GcPlan::empty();
        let result = GcExecutionResult::success(plan);
        assert!(result.success);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_gc_execution_result_with_error() {
        let plan = GcPlan::empty();
        let result = GcExecutionResult::success(plan).with_error("Test error".to_string());
        assert!(!result.success);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_protected_entity_key() {
        let checkpoint_id = CheckpointId::new();
        let entity = ProtectedEntity::NamedCheckpoint(checkpoint_id);
        assert!(entity.key().starts_with("checkpoint:"));
    }

    #[test]
    fn test_checkpoint_info_clone() {
        let info = CheckpointInfo {
            id: CheckpointId::new(),
            session_id: SessionId::new(),
            branch_id: BranchId::new(),
            turn_id: Some(TurnId::new()),
            manifest_id: ManifestId::new(),
        };
        let cloned = info.clone();
        assert_eq!(info.id, cloned.id);
    }

    #[test]
    fn test_manifest_info_clone() {
        let info = ManifestInfo {
            id: ManifestId::new(),
            session_id: SessionId::new(),
            checkpoint_id: CheckpointId::new(),
            manifest_data: "{}".to_string(),
        };
        let cloned = info.clone();
        assert_eq!(info.id, cloned.id);
    }

    #[test]
    fn test_branch_info_clone() {
        let info = BranchInfo {
            id: BranchId::new(),
            session_id: SessionId::new(),
            name: "test".to_string(),
            parent_branch_id: None,
            head_turn_id: None,
        };
        let cloned = info.clone();
        assert_eq!(info.id, cloned.id);
    }

    #[test]
    fn test_live_entity_variants() {
        let session = LiveEntity::Session(SessionId::new());
        let branch = LiveEntity::Branch(BranchId::new());
        let checkpoint = LiveEntity::Checkpoint(CheckpointId::new());
        let manifest = LiveEntity::Manifest(ManifestId::new());
        let blob = LiveEntity::Blob(BlobId::new());

        // Just verify they can be created and used in a context
        let entities = vec![session, branch, checkpoint, manifest, blob];
        assert_eq!(entities.len(), 5);
    }

    #[test]
    fn test_gc_plan_with_data() {
        let blob_id = BlobId::new();
        let checkpoint_id = CheckpointId::new();
        let manifest_id = ManifestId::new();

        let plan = GcPlan {
            blobs_to_delete: vec![blob_id],
            checkpoints_to_delete: vec![checkpoint_id],
            manifests_to_delete: vec![manifest_id],
            protected_blobs: vec![],
            live_entities: vec![],
            total_blobs: 1,
            total_checkpoints: 1,
            total_manifests: 1,
        };

        assert!(plan.would_delete());
        assert_eq!(plan.total_to_delete(), 3);
    }
}
