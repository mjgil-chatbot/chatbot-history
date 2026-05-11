# AI History System - Design Specification

## Overview

The AI History System provides persistent, branching, rollback-capable history for AI coding assistant interactions. It combines event sourcing with content-addressed storage to maintain complete audit trails of conversations, file changes, and tool executions.

## Goals

1. **Complete History Preservation** - Never lose conversation history or file changes
2. **Branch and Fork** - Support parallel exploration of different approaches
3. **Safe Rollback** - Restore to any previous checkpoint without data loss
4. **Conflict Detection** - Identify and resolve workspace conflicts before restore
5. **Garbage Collection** - Clean up unreachable blobs while protecting live data
6. **Export/Import** - Portable archives for backup and migration
7. **Observability** - Structured logging and metrics for operations

## Non-Goals

1. **Real-time Collaboration** - This is a single-user local system
2. **Cloud Sync** - Data stays on local filesystem
3. **AI Model Hosting** - The system stores history, not model weights
4. **Full Text Search** - Only structural indexing, not content search
5. **Concurrent Writes** - Single-writer semantics per session

## Architecture

The system consists of 11 independent Rust crates:

```
┌─────────────────────────────────────────────────────────────┐
│                     ai-history-cli                          │
│                  (command-line interface)                   │
└─────────────────────────┬───────────────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
        ▼                 ▼                 ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│ ai-history-fsm│ │ai-history-   │ │ai-history-    │
│ (state        │ │events        │ │restore        │
│  machine)      │ │(event         │ │(conflict       │
│               │ │ sourcing)     │ │ detection)    │
└───────┬───────┘ └───────┬───────┘ └───────┬───────┘
        │                 │                 │
        └─────────────────┼─────────────────┘
                          │
┌─────────────────────────┼───────────────────────────────────┐
│                   ai-history-core                           │
│  (typed IDs, errors, timestamps, actors, path safety,      │
│   invariants, shared traits, GC, export/import, design     │
│   review, recovery, rollback modes, dirty workspace,       │
│   conflict resolution, materialized views, retention,      │
│   security, accessibility, observability)                   │
└─────────────────────────┬───────────────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
        ▼                 ▼                 ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│ai-history-store│ │ai-history-   │ │ai-history-    │
│ (SQLite        │ │snapshot       │ │diff           │
│  persistence)   │ │(workspace     │ │(patch         │
│               │ │ snapshots)    │ │ application)  │
└───────────────┘ └───────────────┘ └───────────────┘
                          │
┌─────────────────────────┼───────────────────────────────────┐
│              ai-history-context                             │
│        (context management, reproducibility)                │
└─────────────────────────┬───────────────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
        ▼                 ▼                 ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│ai-history-ui  │ │ai-history-   │ │ai-history-    │
│-model         │ │testkit        │ │cli            │
│(UI DTOs only) │ │(testing utils)│ │(commands)     │
└───────────────┘ └───────────────┘ └───────────────┘
```

## State Machine

The system uses a finite-state transducer to control the interaction lifecycle:

```
┌──────────────────────────────────────────────────────────────┐
│                         State Diagram                         │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│   ┌──────┐                                                    │
│   │ Idle │◄─────────────────────────┐                        │
│   └──┬───┘                          │                        │
│      │ +UserPrompt                   │                        │
│      ▼                              │                        │
│   ┌─────────────────┐               │                        │
│   │StreamingAssistant│──────────────┤                        │
│   └────────┬────────┘               │                        │
│            │ +ToolWriteRequested    │                        │
│            ▼                        │                        │
│   ┌─────────────────┐               │                        │
│   │ ApplyingTool    │───────────────┤                        │
│   └────────┬────────┘               │                        │
│            │ +ToolCompleted         │                        │
│            ▼                        │                        │
│   ┌──────┴───────┐                  │                        │
│   │   Idle       │◄─────────────────┤                        │
│   └──┬────────┬──┘                  │                        │
│      │        │ +RestoreRequested   │                        │
│      │        ▼                      │                        │
│      │   ┌────────────────┐           │                        │
│      │   │  Restoring    │───────────┘                        │
│      │   └───────┬────────┘                                   │
│      │           │ +ConflictDetected                          │
│      │           ▼                                            │
│      │   ┌─────────────────────┐                              │
│      └──►│ConflictResolution    │                              │
│          └─────────────────────┘                              │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### State Transitions

| Current State | Input | Next State | Output Actions |
|---------------|-------|------------|----------------|
| Idle | UserPrompt | StreamingAssistant | CreateTurn, CaptureBeforeCheckpoint |
| StreamingAssistant | AssistantDelta | StreamingAssistant | AppendAssistantDelta |
| StreamingAssistant | ToolWriteRequested | ApplyingTool | CapturePreWriteCheckpoint |
| ApplyingTool | ToolWriteSucceeded | Idle | AppendToolResult |
| ApplyingTool | ToolWriteFailed | Idle | LogError |
| Idle | RestoreRequested | Restoring | CaptureSafetySnapshot |
| Restoring | ConflictDetected | ConflictResolution | ShowConflictUI |
| Restoring | RestoreComplete | Idle | FinalizeRestore |
| ConflictResolution | ResolutionChosen | Restoring | ApplyResolution |

## Event Schema

Events are the core immutable record of all state changes:

```rust
// Event types (30+ variants)
enum EventType {
    SessionCreated,
    SessionUpdated,
    TurnStarted,
    TurnFinalized,
    UserMessageReceived,
    AssistantMessageStarted,
    AssistantMessageDelta,
    AssistantMessageFinalized,
    ToolCallRequested,
    ToolCallStarted,
    ToolCallCompleted,
    ToolCallFailed,
    FileSnapshotCreated,
    FileSnapshotRestored,
    BranchCreated,
    BranchSwitched,
    CheckpointCreated,
    WorkspaceRestored,
    ConflictDetected,
    ConflictResolved,
    RollbackPerformed,
    RedoPerformed,
    GcStarted,
    GcCompleted,
    ExportPerformed,
    ImportPerformed,
}
```

### Event Structure

```rust
struct Event {
    id: usize,                    // Monotonic sequence number
    session_id: SessionId,         // Parent session
    branch_id: BranchId,           // Current branch
    turn_id: Option<TurnId>,       // Associated turn
    sequence_num: u64,             // Global sequence
    event_type: EventType,         // Event variant
    event_data: serde_json::Value, // Type-specific payload
    causation_id: Option<String>,  // Parent event ID
    correlation_id: Option<String>,// Request correlation
    actor: Actor,                 // User or Assistant
    checksum: String,              // SHA-256 integrity hash
    created_at: String,            // RFC3339 timestamp
}
```

## Storage

### Content-Addressed Blob Store

All content is stored with SHA-256 hashes:

```
BlobId = SHA-256(content)
```

Benefits:
- Deduplication automatic
- Integrity verification built-in
- Parallel access safe

### SQLite Metadata Store

```sql
-- Sessions table
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    workspace_root TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Branches table (DAG nodes)
CREATE TABLE branches (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    name TEXT NOT NULL,
    parent_branch_id TEXT,
    head_turn_id TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- Checkpoints table
CREATE TABLE checkpoints (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    turn_id TEXT,
    manifest_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- Events table (journal)
CREATE TABLE events (
    id INTEGER PRIMARY KEY,
    session_id TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    turn_id TEXT,
    sequence_num INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL,
    checksum TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Manifests table (blob references)
CREATE TABLE manifests (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    checkpoint_id TEXT NOT NULL,
    manifest_data TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Blobs table (content address)
CREATE TABLE blobs (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    hash TEXT NOT NULL UNIQUE,
    size INTEGER NOT NULL,
    created_at TEXT NOT NULL
);
```

### Directory Structure

```
$AI_HISTORY_HOME/
├── store.db           # SQLite metadata
├── blobs/             # Content-addressed blobs
│   ├── ab/
│   │   └── c123...    # SHA-256 prefix directories
│   └── de/
│       └── f456...
└── sessions/          # Per-session workspace roots
    └── {session-id}/
        └── .ai-history-state  # Symlink to store
```

`$AI_HISTORY_HOME` represents the chosen application data directory for the
deployment environment.

## Canonical Type Ownership

Each domain type has exactly one owning crate to prevent duplicate definitions and ambiguous imports.

| Type | Canonical Owner | Notes |
|------|-----------------|-------|
| SessionId, BranchId, TurnId, CheckpointId, BlobId, ManifestId | `ai-history-core` (id.rs) | Typed ID wrappers around Uuid |
| Session | `ai-history-core` (session.rs) | Domain session with retention policy |
| Branch | `ai-history-core` (branch.rs) | Branch DAG node with metadata |
| Turn | `ai-history-core` (turn.rs) | Turn with message list and metadata |
| Message | `ai-history-core` (message.rs) | User/Assistant/System/Tool message |
| Checkpoint | `ai-history-core` (checkpoint.rs) | Checkpoint with manifest linkage |
| Manifest | `ai-history-snapshot` (manifest.rs) | Content-addressed workspace snapshot |
| BlobStore trait | `ai-history-store` (traits.rs) | Trait for blob storage backends |
| BlobStore impl | `ai-history-snapshot` (blob_store.rs) | Concrete content-addressed blob storage |
| BlobStoreConfig | `ai-history-core` (blob_store.rs) | Configuration types for blob storage |
| Event | `ai-history-events` (event.rs) | All event enums and variant data |
| EventId, SequenceNumber, PayloadVersion | `ai-history-core` (event_log.rs) | Event log infrastructure types |
| SessionInfo, BranchInfo, TurnInfo, CheckpointInfo, BlobInfo, ManifestInfo | `ai-history-store` (traits.rs) | Storage tier info types |
| All UI DTOs | `ai-history-ui-model` | TimelineItem, BranchItem, CheckpointItem, etc. |
| GcStore trait, GcPlanner, GcExecutor | `ai-history-core` (gc.rs) | GC infrastructure |
| RestorePlanner, RestoreExecutor | `ai-history-restore` | Restore planning and execution |
| MaterializedViews | `ai-history-core` (materialized_views.rs) | Read model projections |
| Exporter, Importer | `ai-history-core` (export.rs) | Archive import/export |

### Cross-Crate Type Usage

Types are re-exported as needed but the owning crate is the single source of truth:
- `ai-history-core` exports Session, Branch, Turn, Message, Checkpoint, BlobStoreConfig
- `ai-history-snapshot` exports BlobStore (concrete implementation), Manifest, Scanner
- `ai-history-events` exports Event and all event data variants
- `ai-history-store` exports storage traits and info types
- `ai-history-ui-model` exports all UI DTOs only
- `ai-history-snapshot` exports Manifest and related snapshot types

## ai-history-events Responsibilities

`ai-history-events` is the sole source for all event definitions and event-related types. No other crate may define event enums or event variant data.

### Explicit Responsibilities
- **Event enum**: The `Event` enum with 30+ variants covering turns, messages, tools, patches, checkpoints, restore, branches, conflicts, and system events
- **Event data variants**: TurnCreatedData, UserMessageData, AssistantMessageDeltaData, ToolCallRequestedData, FilePatchProposedData, CheckpointCreatedData, BranchCreatedData, SessionCreatedData, and all other event data types
- **Payload versions**: Payload versioning for event schema evolution
- **Sequence numbers**: Per-session monotonic sequence assignment
- **Actor**: Actor enum is defined in core but used by events
- **Causation ID**: Tracking what caused an event
- **Correlation ID**: Tracking related events in a flow
- **Checksums**: Event content integrity via SHA-256
- **Hash-chain fields**: Linking events for replay integrity
- **Replay schema**: EventStore trait, EventReplay trait, replay interfaces
- **Event validation**: EventSchema validation rules
- **EventMetadata**: Causation/correlation/sequence tracking struct

### Explicit Non-Responsibilities
- **No persistence**: Event storage is in `ai-history-store`
- **No session/turn CRUD**: Those domain models are in `ai-history-core`
- **No restore execution**: Restore lives in `ai-history-restore`
- **No FSM logic**: FSM lives in `ai-history-fsm`

### Event Schema Versioning Rules
1. Events are serialized with a version tag
2. Unknown fields are ignored (forward compatibility)
3. Missing required fields cause validation failures
4. Schema version is checked on deserialization

### Event Checksum and Hash-Chain Algorithm
- **Checksum**: SHA-256 of canonical JSON serialization (payload only, excludes chain linkage)
- **Hash-Chain**: SHA-256 of canonical JSON including previous hash for chain linkage
- **Format**: 64 hex characters (SHA-256 output)
- **Determinism**: Canonical JSON serialization ensures identical checksum for identical content
- **Tamper Detection**: Any content modification produces different checksum

### Store Persistence Boundary
Events are persisted by `ai-history-store` but the schema ownership remains in `ai-history-events`. Store implementations must not redefine event types - they consume what's defined in events. The store handles SQLite schema, migrations, and the actual persistence, while events owns the canonical event enum, payload versions, checksums, causation/correlation metadata, sequence numbers, and replay-facing contracts.

## ai-history-fsm Responsibilities

`ai-history-fsm` is the pure finite-state transducer that controls the interaction lifecycle. It contains no side effects.

### Explicit Responsibilities
- **States (Phases)**: Idle, CreatingTurn, StreamingAssistant, AwaitingToolResult, ApplyingPatch, Checkpointing, Restoring, ConflictResolution, Cancelled, Error
- **Input events**: UserPromptSubmitted, AssistantDelta, ToolResult, PatchProposed, PatchApproved, CheckpointRequested, RestoreRequested, NavigateBackward, NavigateForward, ForkCurrent, ForkFromCursor, ContinueFromCursor, etc.
- **Output actions**: CreateTurn, AppendEvent, CaptureWorkspaceSnapshot, ExecuteRestore, ShowConflictUI, etc.
- **Transition logic**: Pure function `transition(phase, runtime, input) -> TransitionResult`
- **Runtime state**: TurnId, BranchId, SessionId, event sequence tracking
- **Invalid transition detection**: Rejecting invalid state/input combinations

### Workflows Covered
- Prompt → turn → stream → tool call → patch → checkpoint
- Restore → conflict detection → conflict resolution
- Branch switch → cursor navigation
- Fork current, fork from cursor, continue from cursor
- Import/Export, GC, doctor

### Explicit Non-Responsibilities
- **No side effects**: FSM makes no filesystem, database, or network calls
- **No command execution**: Orchestrator executes OutputAction commands
- **No storage**: Event store, blob store, manifest store are external

### Transition Table Testing
Exhaustive transition tests verify:
- All valid state+input combinations produce expected next state and actions
- Invalid combinations are rejected
- State machine is deterministic

### Invariant Enforcement
The FSM enforces operation invariants through transition rules:
- **Safety checkpoint before restore**: Any `RestoreRequested` captures a safety checkpoint before proceeding, enabling rollback on failure
- **Checkpoint capture for destructive operations**: `FilePatchProposed` captures a workspace snapshot before applying changes
- **Invalid transition rejection**: Events invalid in a given state (e.g., `ToolCallCompleted` in `Idle`) transition to `Error` rather than being silently ignored
- **Turn finalization immutability**: `TurnFinalized` events transition to `Idle` and finalize the turn, preventing subsequent deltas

### Checkpoint Trigger and Restore Safety Wiring
Checkpoint creation and restore flows are wired through public API events:
- `CheckpointRequested` → `CaptureWorkspaceSnapshot` action for checkpoint creation
- `RestoreRequested` / `RestoreWorkspaceOnly` / `RestoreConversationOnly` → pre-restore safety checkpoint + restore execution
- `CheckpointCreated` / `RestoreCompleted` complete the respective flows
- `WorkspaceConflictDetected` → `ConflictResolution` state for interactive conflict handling
- Safety invariants enforced in `ai-history-restore::executor` require `safety_checkpoint_id` for destructive restore modes

## ai-history-store Responsibilities

`ai-history-store` is the persistence layer providing SQLite storage and related abstractions.

### Explicit Responsibilities
- **SQLite adapter**: SqliteStore implementing all storage traits
- **Migrations**: Schema version table, foreign keys, indexes, idempotent migrations
- **Storage traits**: SessionStore, BranchStore, TurnStore, CheckpointStore, BlobStore, ManifestStore
- **Unit of work**: Atomic operations for sessions, branches, turns, events, checkpoints, manifests, blobs
- **Transaction support**: Transaction guard with commit/rollback
- **Materialized views**: ChatTranscriptView, BranchTimelineView, CheckpointListView, RestoreHistoryView
- **Message persistence**: First-class message records with sequence ordering for replay-safe message storage
- **Cursor navigation**: Persistent cursor state per (session, branch) for back/forward navigation
- **Orchestrator**: FSM command execution against storage
- **Integrity scanning**: StoreIntegrityScanner for event checksums, branch refs, checkpoint reachability
- **GC support**: GcStore trait implementation for GC planning
- **Import/export persistence**: Archive import/export through store APIs

### Storage Schema
Tables: sessions, branches, turns, events, checkpoints, manifests, blobs, materialized_views, cursor_position, app_state, messages
Indexes on: events(session_id), events(branch_id), events(sequence_num), checkpoints(session_id), blobs(hash), messages(turn_id), messages(sequence_num)
Materialized views: ChatTranscriptView, BranchTimelineView, CheckpointListView, FileChangeSummaryView, RestoreHistoryView, WorkspaceStateView

### Cascade Deletion and Lifecycle Invariants
Foreign key constraints with `ON DELETE CASCADE` enforce referential integrity:
- `branches.session_id → sessions(id) ON DELETE CASCADE` — deleting a session removes all branches
- `turns.session_id → sessions(id) ON DELETE CASCADE` — deleting a session removes all turns
- `turns.branch_id → branches(id) ON DELETE CASCADE` — deleting a branch removes all turns
- `events.session_id → sessions(id) ON DELETE CASCADE` — deleting a session removes all events
- `events.branch_id → branches(id) ON DELETE CASCADE` — deleting a branch removes all events
- `checkpoints.session_id → sessions(id) ON DELETE CASCADE` — cascade on session delete
- `checkpoints.branch_id → branches(id) ON DELETE CASCADE` — cascade on branch delete
- `messages.turn_id → turns(id) ON DELETE CASCADE` — deleting a turn removes all messages
- `context_snapshots.turn_id → turns(id) ON DELETE CASCADE` — deleting a turn removes all context snapshots
- `manifests.checkpoint_id → checkpoints(id) ON DELETE CASCADE` — deleting a checkpoint removes its manifest

Turn immutability: Once `finalized_at` is set, the turn enters immutable state. Subsequent message creation is rejected at the application layer. Cursor-only navigation (back/forward) does not affect the branch head.

Checkpoint/manifest ordering: Due to circular FK between checkpoints and manifests (manifest.checkpoint_id → checkpoints), checkpoint must be created before its manifest.

### Backend Extension Points
Trait-based design allows alternative backends (Postgres, cloud). The trait definitions in traits.rs define the contract.

### Explicit Non-Responsibilities
- **No CLI logic**: CLI lives in `ai-history-cli`
- **No restore execution**: Restore executor lives in `ai-history-restore`
- **No blob filesystem ops**: Concrete blob I/O uses core traits, actual filesystem in snapshot crate
- **No UI models**: UI DTOs live in `ai-history-ui-model`

## ai-history-snapshot Responsibilities

`ai-history-snapshot` owns workspace scanning and manifest management.

### Explicit Responsibilities
- **Workspace scanning**: WalkDir-based traversal with ignore rule evaluation
- **Ignore rules**: .gitignore, product ignore file, default excludes, generated-file excludes
- **Path validation integration**: SafePath validation during scanning
- **Blob store implementation**: Concrete content-addressed blob storage with atomic writes, deduplication, and corruption detection
- **Blob writes**: File content written to blob storage before manifest creation
- **Manifest creation**: Manifest with entries mapped to blob hashes
- **Manifest canonicalization**: Deterministic serialization for content-addressed deduplication
- **Parent linkage**: Parent manifest ID tracking for snapshot chains
- **Manifest comparison**: Three-way base/current/target with added/modified/deleted/renamed/binary/conflict classification
- **Platform metadata**: Cross-platform file metadata (Unix permissions, executable bit, symlinks)
- **Binary policy**: Explicit handling of binary vs text files

### Ignore Precedence
1. Product ignore file (highest priority)
2. .gitignore patterns
3. Default excludes (node_modules, target, etc.)
4. Generated file excludes
5. Max size limits (configurable)

### Manifest Format
```json
{
  "id": "manifest:abc123...",
  "content_hash": "sha256:...",
  "parent_id": "manifest:xyz...",
  "entries": {
    "src/main.rs": {
      "blob_hash": "sha256:...",
      "size": 1234,
      "mode": 0o644,
      "executable": false
    }
  },
  "total_size": 50000,
  "file_count": 15
}
```

### Platform Rules
- Unix: preserves mode bits, executable bit, symlink target
- Non-Unix: default 0o644 mode, no executable bit detection
- Symlinks: detected via symlink_metadata without following into target

### Symlink Policy
- **External symlinks**: When `follow_symlinks=false`, symlinks pointing outside the workspace are rejected before file content is read
- **Internal symlinks**: Symlinks with targets inside the workspace are included; the target path is recorded in `symlink_target`
- **Follow mode**: When `follow_symlinks=true`, symlinks are followed and target content is included

### Scanner Policy Fields
The scanner captures or controls these behaviors:
- **binary_policy**: Controls binary file handling (`Skip`, `Include`, `HashOnly`)
- **ignore_provenance**: Tracked via ignore rule sources (.gitignore, .ai-history-ignore, default rules)
- **parent linkage**: Parent manifest ID preserved through serialization
- **mtime**: File modification time recorded in RFC3339 format

### Manifest Format
```json
{
  "id": "manifest:abc123...",
  "content_hash": "sha256:...",
  "parent_id": "manifest:xyz...",
  "entries": {
    "src/main.rs": {
      "blob_hash": "sha256:...",
      "size": 1234,
      "mode": 0o644,
      "executable": false,
      "symlink_target": null,
      "modified": "2026-05-16T10:30:00Z",
      "ownership": "AiOwned"
    }
  },
  "total_size": 50000,
  "file_count": 15
}

## ai-history-diff Responsibilities

`ai-history-diff` owns unified diff generation, patch application, and merge operations.

### Explicit Responsibilities
- **Unified diff generation**: Text diff with Delete/Insert/Equal change tracking
- **Inverse patches**: Reverse diff for undo operations
- **Binary fallback**: Refusal to diff binary content (returns error)
- **Patch precondition validation**: Verify base content matches before applying
- **CRLF handling**: Line ending normalization or rejection
- **Multi-hunk offsets**: Correct handling when edits shift line numbers
- **Empty files**: Special handling for empty file diffs
- **Stale base checks**: Reject patches when base has been modified
- **Three-way merge**: Merge with conflict detection and structured MergeResult

### Patch Grammar
Patches follow standard unified diff format:
```diff
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,5 @@
 fn main() {
-    old line
+    new line
 }
```
Multi-hunk patches include multiple @@ regions with offset corrections.

### Inverse Guarantees
- `inverse()` of a diff produces a diff that undoes the original
- Double-inverse does not necessarily equal original (round-trip property)
- Inverse requires same base content (stale base rejection)

### Merge Limits
- Binary content cannot be merged (BinaryContent error)
- Conflict threshold: too many conflicts returns TooManyConflicts error
- Merge produces conflict markers when automatic merge fails

## ai-history-context Responsibilities

`ai-history-context` owns context snapshots for reproducibility tracking.

### Explicit Responsibilities
- **Context snapshots**: ContextSnapshot capturing turn creation state
- **Selected files**: SelectedFileRef with path, blob_hash, line_range, relevance_score
- **Retrieved chunks**: RetrievedChunkRef with chunk_id, source, blob_hash, relevance_score
- **Tool outputs**: ToolOutputRef with tool_call_id, tool_name, output_hash, timestamp
- **Summary references**: SummaryRef with summary_id, summary_hash, SummaryType
- **Memory references**: MemoryRef with memory_id, memory_hash, MemoryType
- **Rule references**: RuleRef with rule_set_id, rules_hash
- **System prompt hash**: ContentHash of the system prompt at turn creation
- **Model config**: ModelConfig with provider, model, temperature, top_p, max_tokens, stop_sequences, feature_flags
- **Reproducibility tracking**: ReproducibilityManager for dependency verification

### Captured Context Fields
| Field | Type | Description |
|-------|------|-------------|
| selected_files | Vec<SelectedFileRef> | Files opened by user/assistant |
| retrieved_chunks | Vec<RetrievedChunkRef> | RAG chunks retrieved for context |
| tool_outputs | Vec<ToolOutputRef> | Tool execution results |
| summary_refs | Vec<SummaryRef> | Conversation summaries |
| memory_refs | Vec<MemoryRef> | Memory system entries |
| rule_refs | Vec<RuleRef> | Rule set references |
| system_prompt_hash | ContentHash | Hash of system prompt |
| model_config | ModelConfig | Model parameters |

### Excluded Context
Large binary content, full file contents (only references stored), sensitive data.

### Persistence Hook
Context snapshots are persisted via `ai-history-store` (`ContextSnapshotRecord`) with:
- Foreign key to `turns(id)` for turn association
- JSON-serialized snapshot data for full context reconstruction
- Count fields for quick indexing (selected_files_count, etc.)
- System prompt hash and model config data as separate indexed fields
- Created timestamp for ordering and replay planning

### Reproducibility Limits
- Dependencies must exist at replay time (missing blob returns error)
- Stale references detected via content hash mismatch

## ai-history-ui-model Responsibilities

`ai-history-ui-model` owns all UI-ready data transfer objects (DTOs). No business logic lives here.

### Explicit Responsibilities
- **Timeline views**: TimelineItem, TimelineView for conversation history
- **Checkpoint list views**: CheckpointItem, CheckpointListView for checkpoint navigation
- **Branch timeline views**: BranchItem, BranchTimelineView for branch DAG display
- **File change summaries**: FileChangeSummaryView, FileChangeView for diff display
- **Restore modal states**: RestoreModalState, RestoreHistoryItem, RestoreHistoryView
- **Conflict screen states**: ConflictItem, ConflictScreenState for conflict resolution UI
- **Accessibility**: AccessibilityLabels, KeyboardNavigation, ScreenReaderAnnouncement, DestructiveConfirmState, ConflictExplanation, UxCopy, AccessibilityContract
- **DTO-only**: All types are plain data shapes, no methods that mutate state

### Frontend API Contracts
All types serialize to JSON for frontend consumption. TimelineItem example:
```json
{
  "turn_id": "turn:abc123",
  "actor": "User",
  "content": "Hello",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

### Explicit Non-Responsibilities
- **No business logic**: Pure DTOs, no decision-making
- **No storage**: UI model doesn't call store directly
- **No network calls**: Data flows through orchestrator/CLI layer

## ai-history-cli Responsibilities

`ai-history-cli` is the command-line interface entry point.

### Explicit Responsibilities
- **Commands**: init, checkpoint, list-checkpoints, restore, fork, branches, diff, status, navigate-back, navigate-forward, gc, doctor, export, import, verify
- **Session management**: Create/manage session and workspace binding
- **Real API calls**: All commands call real store/snapshot/restore/FSM/orchestrator APIs (no placeholder behavior)
- **Output formatting**: Human-readable output for all commands
- **Error reporting**: Clear error messages with exit codes
- **Thin entrypoint**: `src/bin/main.rs` only parses CLI input, opens the store, and delegates command execution into `src/bin/command_runner/` modules
- **Deterministic bin tests**: `src/bin/bin_tests/` uses `TempDir` workspaces so checkpoint/list/restore/status/fork coverage does not depend on shared machine state

### Command Semantics
| Command | Behavior |
|---------|----------|
| init | Create durable session, main branch, metadata, workspace binding, storage dirs |
| checkpoint | Scan workspace with real blob persistence (scan_with_blob_store), persist manifest with checkpoint-created-before-manifest ordering, create checkpoint metadata |
| list-checkpoints | Query persistent store, support branch/session filters, show checkpoint metadata |
| restore | Capture safety checkpoint (checkpoint before manifest), load target manifest, build plan, execute transactional restore with preflight validation, verify |
| fork | Create branch from checkpoint/turn, set head, optionally restore workspace |
| branches | List branch DAG with active marker, parent, head, fork point, protected state |
| diff | Compare current manifest against checkpoint manifest |
| status | Show session, active branch, cursor, dirty workspace, checkpoint/branch counts |
| gc | Dry-run and execute modes for mark-and-sweep GC |
| doctor | Integrity checks for event checksums, branch refs, checkpoint reachability, blobs |
| export/import | Archive operations with version/checksum metadata |

### Checkpoint Artifact Semantics
- **Manifest entries** reference blob hashes computed from file content during scan
- **Blob persistence** occurs during scan via Scanner::scan_with_blob_store (not scan())
- **Checkpoint-manifest ordering**: checkpoint created first, then manifest referencing it (respects FK constraint)
- **Safety checkpoint for restore**: created before manifest in same order, enables rollback on failure

### Restore Safety Behavior
- **Safety checkpoint requirement**: destructive restore modes require pre-existing safety checkpoint
- **Preflight validation**: all blob references verified to exist before any file operations execute
- **Transactional execution**: all-or-nothing semantics; failure triggers rollback
- **Verification failure**: if any file content mismatch post-restore, operation fails

### Explicit Non-Responsibilities
- **No business logic**: Commands delegate to orchestrator/store/snapshot layers
- **No direct storage access**: Uses public APIs only

## ai-history-testkit Responsibilities

`ai-history-testkit` provides test fixtures, builders, fakes, and acceptance scenarios.

### Explicit Responsibilities
- **Builders**: SessionBuilder, BranchBuilder, TurnBuilder, MessageBuilder, CheckpointBuilder, ManifestBuilder
- **Fakes**: FakeBlobStore, FakeEventStore, FakeSessionStore for trait conformance testing
- **Generators**: Deterministic content generation for repeatable tests
- **Acceptance scenarios**: Full end-to-end test scenarios
- **Completion evidence gate**: Verify implementation completeness
- **Design tests**: Verify design decisions are implemented correctly

### Fixture Patterns
```rust
// Builder pattern for creating test data
let session = SessionBuilder::new()
    .title("test session")
    .with_branch("main")
    .with_turn()
    .with_message("user", "Hello")
    .build();

// Fake store for trait conformance
let store = FakeBlobStore::new()
    .with_blob(hash, data);
```

### Fake vs Real Store Guidance
- Use fakes for isolated unit tests
- Use real SQLite stores for integration tests
- Acceptance tests use temp workspaces with real stores

## Completion Scanning System

The completion scanning system verifies implementation status by examining evidence from multiple sources.

### Evidence Providers

| Provider | Type | Purpose |
|---------|------|---------|
| `FilesystemEvidenceProvider` | Filesystem | Scans repository for feature markers |
| `docs_gate` | Documentation | Verifies design.md sections exist |
| `implementation_matrix` | Matrix | Maps tasks to required files/evidence |
| `completion_gate` | Gate | Passes only when all criteria met |
| `doc_completeness` | Composite | Aggregates all documentation checks |

### Unified Scanner Rules

The `UnifiedScanner` combines multiple evidence sources:
- Repository markers (file existence, code patterns)
- Documentation coverage (design.md sections, API docs)
- Test evidence (test file existence, test names)
- Build verification (compile success, no warnings)

```rust
pub struct UnifiedScanner {
    providers: Vec<Box<dyn EvidenceProvider>>,
    rules: Vec<Box<dyn ScanRule>>,
}
```

### Documentation Requirements

For a task to be considered complete:
1. Design decision must be recorded in `design.md`
2. Implementation must have tests in appropriate crate
3. Feature must be documented via doc comments
4. Status must be updated in the release sign-off checklist when the feature is part of a release claim

### Audit Generation

The audit system generates implementation reports:
- Task completion percentage by crate
- Missing evidence items per task
- Recommendations for incomplete features
- Repository health metrics

```
Audit Output:
- Total Tasks: 104
- Complete: N (%)
- Incomplete: M (%)
- Crate Breakdown: [...]
```

### Evidence Schema

The repository-backed completion gate derives status from filesystem evidence
rather than trusting static checklist labels. The five evidence statuses are:

| Status | Meaning | Repository Evidence Required |
|--------|---------|------------------------------|
| `Complete` | Fully implemented with real behavior | Source files AND tests AND docs all found |
| `Partial` | Core works but edge cases missing | Source OR tests found, but not all required |
| `Placeholder` | Stub implementation returns hardcoded values | Source found but contains TODO/FIXME/stub keywords |
| `Missing` | Not yet implemented | No source files found for the claimed feature |
| `Overclaimed` | Task claims Complete but has no evidence | Status column says Complete but no source links exist |

**Downgrade Rules (derived from repository evidence):**
- If no source links found → `Missing`
- If source links point to nonexistent files → `Missing`
- If source exists but has placeholder keywords (TODO, FIXME, stub) → `Placeholder`
- If source found but tests missing → `Partial`
- If source found but docs missing → `Partial`
- If task claims Complete but no source links → `Overclaimed`

**Audit Output States:**
- `Implemented`: Source + tests + docs + no placeholder keywords
- `Deferred`: Explicitly marked deferred with rationale in code
- `Non-goal`: Feature explicitly listed as non-goal in design.md

### Release Sign-Off Checklist

Before a feature can be marked Complete and included in a release:

1. **Source Evidence**: Implementation files exist in repository and contain real behavior (not TODO/FIXME/stub)
2. **Test Evidence**: Tests exist in appropriate crate and pass
3. **Documentation Evidence**: Feature mentioned in design.md or API docs in source
4. **Design Feature Mapping**: Feature mapped to code, tests, docs, and explicit status evidence

**Release gate fails if any of these are true:**
- Evidence gate detects `Overclaimed` tasks (Complete status with no source links)
- Evidence gate detects `Missing` source files for any Complete task
- Evidence gate detects `Placeholder` implementations for any Complete task
- Release-checklist tests fail when required design-feature mappings lack evidence

**Sign-off authority:** Each crate owns its evidence. The `ai-history-testkit` crate provides `validate_tasks_evidence_gate()` for CI validation. The `ci_all_pass()` function aggregates all CI checks including the evidence gate.

Open-source release automation runs `cargo fmt --all --check`,
`cargo check --workspace`, `cargo test --workspace --release`, and
`cargo doc --release --workspace --no-deps` from `.github/workflows/ci.yml`.

## Runtime Invariants

The implementation requires enforcing runtime invariants through public APIs.

### Invariant Categories

| Category | Purpose |
|----------|---------|
| Workspace | File write and restore safety invariants |
| Branch | Branch head mutation without events |
| Turn | Turn immutability and delta ordering |
| Event | Sequence monotonicity and checksum validation |
| Checkpoint | Turn reference and manifest validity |

### Enforced Invariants

1. **No file write without checkpoint**: Pre-write checkpoint required before destructive file operations
2. **No destructive restore without safety**: Safety snapshot required before restore operations
3. **No branch head mutation without event**: Branch head changes must be recorded as events
4. **No assistant delta after completion**: Deltas rejected after message finalization
5. **No checkpoint without turn reference**: Checkpoints must reference associated turn

### Invariant Checker API

```rust
InvariantChecker::check_file_write_invariant(has_checkpoint, is_destructive)
InvariantChecker::check_restore_safety_invariant(has_safety_snapshot)
InvariantChecker::check_branch_head_mvariant(has_event)
InvariantChecker::check_delta_after_completion(is_completed, has_pending_deltas)
InvariantChecker::check_checkpoint_turn_reference(has_turn_ref)
```

## Session and Branch Persistence

Tasks 23 and 24 require complete session and branch record persistence.

### SessionInfo Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | SessionId | Primary key |
| `title` | String | Session title |
| `active_branch` | BranchId | Current active branch |
| `view_cursor` | Option<TurnId> | Current view cursor position |
| `workspace_root` | String | Path to workspace |
| `retention_config` | String | Retention policy |
| `project_identity` | String | Project identifier |
| `operation_state` | String | active/archived state |
| `created_at` | String | RFC3339 timestamp |
| `updated_at` | String | RFC3339 timestamp |
| `archival_state` | bool | Whether session is archived |

### BranchInfo Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | BranchId | Primary key |
| `session_id` | SessionId | Parent session |
| `name` | String | Branch name |
| `parent_branch_id` | Option<BranchId> | Parent branch for fork |
| `created_from_turn_id` | Option<TurnId> | Turn cursor was at when forked |
| `head_turn_id` | Option<TurnId> | Current head turn |
| `ancestry` | Vec<BranchId> | Immutable ancestor chain |
| `protected` | bool | Protected from GC |
| `archived` | bool | Archived branch |
| `abandoned_future` | bool | Abandoned future indicator |
| `created_at` | String | RFC3339 timestamp |

## Fork Semantics

Tasks 25-29 implement fork conversation semantics.

### fork_current_conversation

Creates a child branch with:
- `parent_branch_id` = active branch
- `head_turn_id` = active branch head turn
- `created_from_turn_id` = head turn (same as head when forking from head)

### fork_from_cursor

Creates a branch from cursor position:
- `parent_branch_id` = active branch
- `head_turn_id` = cursor turn (not necessarily the branch head)
- `created_from_turn_id` = cursor turn

Options:
- `restore_workspace`: Whether to restore workspace to cursor's snapshot

### continue_from_cursor

User submits new prompt while viewing earlier turn:
- Creates new branch from cursor position
- Fork point is the cursor turn
- New turn is appended to the new branch

### edit_and_rerun

Editing a prior prompt:
- Creates branch from the edited turn's parent
- Original turn becomes abandoned future
- Cannot edit root turns (no parent to fork from)

### redo_target_discovery

Discover abandoned futures:
- After fork, original branch head becomes "abandoned future"
- User can navigate back to continue from abandoned point
- BranchGraph tracks `abandoned_futures` flag

## Branch Switching

Task 30: Complete branch switching with two modes:
- **Conversation-only switch**: Changes active branch without workspace restore
- **Workspace-restore switch**: Changes active branch and restores workspace to branch head

Dirty state checking:
- Prompt user if workspace has uncommitted changes before switching
- Safety checkpoint before destructive operations

Conflict policy:
- Detect if branch head has conflicting changes
- Provide resolution UI for conflicts

## CLI Navigation

Navigation commands use active session/branch:
- `navigate-back`: Move cursor to parent turn
- `navigate-forward`: Move cursor to forward child

Key behaviors:
- Cursor navigation does NOT mutate branch heads
- Cursor navigation does NOT create events/checkpoints
- Ambiguous forward paths are reported to user
- Active branch selection preserved during navigation

## Turn Immutability

Turns are immutable after finalization through:
- Store APIs (no update method for finalized turns)
- Unit-of-work transactions
- Replay system
- CLI paths

Allowed metadata exceptions:
- `finalized_at` timestamp (set once)
- Stream state markers (started, completed, etc.)

## Event Lifecycles (Tasks 35-37)

### Assistant Stream Lifecycle
Events persisted:
- `AssistantStarted` - Stream began
- `AssistantDelta` - Content chunk received
- `AssistantInterrupted` - Stream paused
- `AssistantResumed` - Stream continued
- `AssistantCompleted` - Stream finished successfully
- `AssistantCancelled` - Stream cancelled by user
- `AssistantFailed` - Stream failed with error
- `AssistantRetried` - Stream restarted after failure

### Tool Call Lifecycle
Events persisted:
- `ToolRequested` - Tool invocation requested
- `ToolApproved` - User approved tool execution
- `ToolRejected` - User rejected tool execution
- `ToolStarted` - Tool execution began
- `ToolCompleted` - Tool execution finished
- `ToolFailed` - Tool execution failed
- `ToolRetried` - Tool execution retried
- `ToolCancelled` - Tool execution cancelled

### Patch Lifecycle Events
Events persisted:
- `PatchProposed` - Patch suggested for application
- `PatchValidated` - Patch validated for correctness
- `PatchApproved` - Patch approved for application
- `PatchRejected` - Patch rejected
- `PatchPreWriteCheckpointed` - Checkpoint before patch
- `PatchApplied` - Patch applied to workspace
- `PatchVerified` - Patch verified successful
- `PatchFailed` - Patch application failed
- `PatchPostWriteSnapshotted` - Checkpoint after patch

## Event Integrity (Tasks 38-40)

### Event Checksums
Canonical SHA-256 checksums computed from:
- Serialized payload (canonical form)
- Metadata (schema version, sequence number)
- Previous event hash (hash chain)

Properties:
- Deterministic (same event always same checksum)
- Tamper-evident (modification breaks checksum)
- Hash chain links events together

### Monotonic Event Append
- Per-session/per-branch sequence numbers
- Transactional assignment
- Rejects: duplicates, gaps, out-of-order appends

### Durable Event Replay
Rebuild from events:
- Transcripts (ChatTranscript)
- Branch timelines (BranchTimeline)
- Checkpoint history
- Restore history
- File summaries
- Redo targets
- Cursor state
- Workspace views

## ai-history-core Responsibilities

`ai-history-core` is the foundational crate that all other crates depend on. It contains:

### Explicit Responsibilities
- **Typed IDs**: SessionId, BranchId, TurnId, CheckpointId, BlobId, ManifestId
- **Hash types**: ContentHash with SHA-256 implementation
- **Timestamp types**: Timestamp, DateTime<Utc> wrappers
- **Actor types**: Actor enum (User, Assistant, System)
- **Error types**: CoreError enum with structured error variants
- **Path types**: SafePath wrapper with traversal protection
- **Invariants**: Runtime invariant checks and enforcement
- **Shared traits**: BlobStore trait, GcStore trait, EventStore trait
- **GC infrastructure**: GcPlanner, GcExecutor, GcOptions, GcPlan
- **Export/Import**: Exporter, Importer, ArchiveFormat for portable archives
- **Design review**: ReviewCategory, design review infrastructure
- **Recovery**: Startup recovery decisions and detection
- **Rollback scope**: RollbackScope and related types
- **Rollback modes**: RollbackMode enum and configuration
- **Dirty workspace**: WorkspaceClassifier, ChangeOwnership, ChangeType
- **Conflict resolution**: ConflictOutcome, FileConflict types
- **Redo**: Redo target discovery and management
- **Prompt edit**: PromptEdit, PromptEditResult, PromptEditRule
- **Reproducibility**: ReproducibilityManager for dependency tracking
- **Materialized views**: TurnView, MessageView, BranchView, CheckpointView, etc.
- **Retention**: RetentionPolicy and retention enforcement
- **Security**: Secret redaction, path boundary enforcement
- **Accessibility**: AccessibilityLabels, KeyboardNavigation, DestructiveConfirmState
- **Observability**: BlobStoreMetrics, tracing and metrics infrastructure
- **Concurrency**: Concurrency primitives and locking

### Explicit Non-Responsibilities (enforced by dependency rules)
- **No concrete persistence**: Does not implement SQLite or any storage backend
- **No concrete blob storage**: BlobStore implementation lives in `ai-history-snapshot`
- **No CLI logic**: No command-line argument parsing or I/O
- **No restore execution**: RestoreExecutor lives in `ai-history-restore`
- **No UI models**: UI DTOs live in `ai-history-ui-model`
- **No FSM state machine**: FSM lives in `ai-history-fsm`
- **No event definitions**: Event enums live in `ai-history-events`
- **No blob filesystem operations**: Concrete blob storage is in store/snapshot crates

This boundary is enforced by the dependency rules in `dependency_rules.rs`. The `core` crate may NOT depend on `store`, `restore`, `cli`, or `ui-model` crates.

## Rollback Flows

### Safe Rollback Protocol

1. **Capture Safety Snapshot** - Before any destructive operation
2. **Detect Conflicts** - Compare workspace state with checkpoint
3. **User Confirmation** - Require explicit confirmation for destructive restores
4. **Atomic Execution** - All-or-nothing restore semantics
5. **Preserve Undo History** - Branch DAG maintains redo capability

### Rollback Decision Tree

```
Restore Requested
       │
       ▼
┌─────────────────┐
│ Has Conflicts?  │
└────────┬────────┘
         │
    ┌────┴────┐
    │ Yes     │ No
    ▼         ▼
┌─────────┐  ┌────────────────┐
│ Show    │  │ User selects   │
│ Conflict│  │ Restore Mode:  │
│ Screen  │  │ - Workspace    │
└────┬────┘  │ - Conversation │
     │       │ - Both         │
     ▼       │ - Fork         │
┌─────────┐  └───────┬────────┘
│ Choose  │          │
│ Resolu- │          ▼
│ tion    │    ┌───────────┐
└────┬────┘    │ Capture   │
     │        │ Safety CP │
     │        └─────┬─────┘
     │              │
     └──────┬───────┘
            ▼
     ┌───────────┐
     │ Execute   │
     │ Restore   │
     └───────────┘
```

## Rust Crates

| Crate | Purpose | Public API |
|-------|---------|------------|
| `ai-history-core` | Typed IDs, errors, traits | `SessionId`, `BranchId`, `TurnId`, `CoreResult` |
| `ai-history-events` | Event sourcing | `Event`, `EventStore`, `EventReplay` |
| `ai-history-store` | SQLite persistence | `SqliteStore`, `Store traits` |
| `ai-history-snapshot` | Workspace snapshots | `SnapshotManager`, `Manifest` |
| `ai-history-diff` | Patch application | `DiffEngine`, `Patch` |
| `ai-history-restore` | Rollback execution | `RestoreExecutor`, `ConflictDetector` |
| `ai-history-fsm` | State machine | `Fsm`, `State`, `Input` |
| `ai-history-context` | Conversation context | `Context`, `ReproducibilityManager` |
| `ai-history-ui-model` | UI state types | `TimelineView`, `RestoreModalState` |
| `ai-history-cli` | CLI commands | `cli()`, `Commands` |
| `ai-history-testkit` | Testing utilities | `TestStore`, `EventGenerator` |

## Canonical Type Ownership

Each domain type has exactly one canonical owner crate. Cross-crate usage must import via the owner's public API.

| Type Category | Canonical Owner | Public Types |
|---------------|-----------------|--------------|
| Typed IDs | `ai-history-core` | `SessionId`, `BranchId`, `TurnId`, `CheckpointId`, `BlobId`, `ManifestId`, `PatchId`, `SequenceNum` |
| Session/Branch/Turn | `ai-history-core` | `Session`, `Branch`, `Turn`, `SessionId`, `BranchId`, `TurnId` |
| Checkpoint | `ai-history-core` | `Checkpoint`, `CheckpointType`, `CheckpointLinks`, `CheckpointTrigger` |
| Event sourcing | `ai-history-events` | `Event` enum + all `*Data` payload types |
| Store info DTOs | `ai-history-store` | `SessionInfo`, `BranchInfo`, `TurnInfo`, `CheckpointInfo`, `BlobInfo`, `ManifestInfo` |
| Blob storage | `ai-history-snapshot` | `BlobStore`, `SnapshotManager`, `Manifest` |
| Diff/patch | `ai-history-diff` | `Diff`, `DiffChange`, `ChangeKind`, `DiffEngine` |
| Restore planning | `ai-history-restore` | `RestorePlan`, `RestoreOperation`, `RestoreMode`, `ConflictPolicy`, `DirtyState` |
| FSM state | `ai-history-fsm` | `Phase`, `InputEvent`, `OutputAction`, `TransitionResult`, `RuntimeState` |
| Context/reproducibility | `ai-history-context` | `Context`, `ContextSnapshot`, `ReproducibilityManager` |
| UI DTOs | `ai-history-ui-model` | `TimelineView`, `RestoreModalState`, all View/State DTOs |
| Concurrency primitives | `ai-history-core` | `WorkspaceGuard`, `AppendGuard`, `WorkspaceLockManager`, `ConcurrencyConfig` |

**Ownership Rules:**
- `ai-history-core` owns: IDs, timestamps, actors, errors, path safety, invariants, domain enums, shared traits
- `ai-history-events` owns: All event enum variants and payload data structures
- `ai-history-store` owns: SQLite adapters, store traits, info DTOs, migrations
- `ai-history-snapshot` owns: Workspace scanning, manifest creation, blob storage
- `ai-history-diff` owns: Diff generation, patch application, merge semantics
- `ai-history-restore` owns: Restore planning, execution, conflict detection
- `ai-history-fsm` owns: State machine logic, transitions, no side effects
- `ai-history-context` owns: Context persistence, reproducibility tracking
- `ai-history-ui-model` owns: Serializable frontend DTOs only
- `ai-history-testkit` owns: Test utilities, builders, fakes, fixtures
- `ai-history-cli` owns: Command parsing, execution, user output

## Testing

### Test Categories

1. **Unit Tests** - Each crate has internal unit tests
2. **Integration Tests** - Cross-crate interaction tests
3. **Property Tests** - Hypothesis-based generative tests
4. **Migration Tests** - SQLite schema migration verification
5. **Concurrency Tests** - Multi-threaded operation safety

### Test Coverage Requirements

| Component | Minimum Coverage | Critical Paths |
|-----------|-----------------|----------------|
| Core | 90% | ID generation, serialization |
| Events | 95% | Event sourcing, replay |
| Store | 85% | CRUD, transactions |
| Restore | 90% | Conflict detection, rollback |
| Snapshot | 85% | Manifest creation, blob reference |
| Diff | 85% | Patch application, conflict resolution |

### Key Test Scenarios

1. **Event Sourcing Round-Trip** - Create events, replay, verify state
2. **Branch Fork Integrity** - Fork at any point, verify isolation
3. **Checkpoint GC Safety** - Protected entities never deleted
4. **Restore with Conflicts** - All resolution paths tested
5. **Concurrent Append** - Single-writer semantics enforced
6. **Import/Export Round-Trip** - Full serialization/deserialization
7. **Corruption Detection** - Checksum validation catches tampering

## Security

### Threat Model

1. **Data Tampering** - Checksums prevent undetected corruption
2. **Denial of Service** - GC never deletes protected entities
3. **Information Disclosure** - No sensitive content in logs
4. **Integrity Violation** - Import verification rejects bad archives

### Security Controls

- **Blob Integrity** - SHA-256 hashing on write and read
- **Event Checksums** - Every event has HMAC-like integrity hash
- **No Secret Logging** - Sensitive paths never logged
- **Import Verification** - Reject archives with invalid checksums
- **Foreign Key Enforcement** - SQLite FK constraints prevent orphans

## Operations

### Health Checks

```bash
# Run integrity verification
ai-history doctor

# Check blob store consistency
ai-history gc --dry-run

# Verify session integrity
ai-history verify --session <id>
```

### Metrics

- `ai_history_operations_total{type}` - Operation counts by type
- `ai_history_operation_duration_seconds{type}` - Latency histograms
- `ai_history_restore_total{outcome}` - Restore outcomes
- `ai_history_blob_store_bytes` - Storage usage
- `ai_history_gc_blobs_removed` - GC effectiveness

### Backup and Recovery

```bash
# Export session
ai-history export --session <id> --output archive.json

# Import session
ai-history import --input archive.json

# Verify import integrity
ai-history import --input archive.json --verify-only
```

## Open Questions

1. **Embedding Storage** - Should we store retrieval embeddings in blob store?
2. **Large File Handling** - What size threshold triggers external blob storage?
3. **Session Naming** - Human-readable names vs. UUID-only?
4. **Workspace Binding** - Should sessions be bound to specific directories?
5. **Concurrent Sessions** - Multiple CLI sessions to same store safe?

## Appendix: Module Map

```
ai-history/
├── Cargo.toml           # Workspace manifest
├── CLAUDE.md            # Claude Code instructions
├── README.md            # Project overview
├── docs/
│   ├── design.md        # This document
│   └── task.md          # Implementation tasks
└── crates/
    ├── ai-history-core/     # Shared types and traits
    ├── ai-history-events/   # Event sourcing
    ├── ai-history-store/    # SQLite persistence
    ├── ai-history-snapshot/ # Workspace snapshots
    ├── ai-history-diff/     # Patch/diff operations
    ├── ai-history-restore/  # Rollback execution
    ├── ai-history-fsm/      # State machine
    ├── ai-history-context/  # Context management
    ├── ai-history-ui-model/ # UI state types
    ├── ai-history-cli/      # CLI application
    └── ai-history-testkit/  # Testing utilities
```
