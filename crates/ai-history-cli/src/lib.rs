// AI History CLI - Command-line interface
//
// This crate provides a thin command surface that delegates to public workspace APIs.
// CLI commands call public APIs and persist through the real store rather than
// bypassing into direct storage shortcuts.
//
// Commands:
// - init: Initialize a new session (delegates to ai-history-store)
// - checkpoint: Create a checkpoint (uses ai-history-snapshot scanner + store)
// - list-checkpoints: List available checkpoints (queries store)
// - restore: Restore from a checkpoint (uses ai-history-restore executor)
// - fork: Create a new branch from a checkpoint (uses store branch API)
// - branches: List branches (queries store)
// - diff: Show diff between current and checkpoint (uses ai-history-snapshot comparison)
// - status: Show session status (queries store + snapshot)
// - gc: Garbage collection (delegates to ai-history-core gc)
// - doctor: Integrity verification (uses ai-history-store integrity scanner)
// - verify: Session/checkpoint verification
// - export/import: Session data export/import
// - navigate-back/forward: Cursor navigation (store-only operation)
// - fork-current/fork-from-cursor: Branch forking operations
// - continue-from-cursor: Continue from historical position
//
// CLI Non-Responsibilities:
// - Business logic lives in ai-history-core, ai-history-fsm, and orchestrators
// - Blob storage is owned by ai-history-snapshot
// - Event schema is owned by ai-history-events
// - Restore planning and execution is owned by ai-history-restore
// - Store persistence is owned by ai-history-store
//
// Integration: Commands use real SQLite persistence via SqliteStore.
// Tests use TempDir workspaces to verify checkpoint/list/restore/diff/status/branches/fork behavior.

pub mod commands;
pub mod output;

pub use commands::*;
pub use output::*;
