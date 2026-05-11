-- Initial schema migration for ai-history-store

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    active_branch TEXT NOT NULL,
    view_cursor TEXT,
    workspace_root TEXT NOT NULL,
    retention_config TEXT NOT NULL,
    project_identity TEXT NOT NULL,
    operation_state TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archival_state INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS branches (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    name TEXT NOT NULL,
    parent_branch_id TEXT,
    created_from_turn_id TEXT,
    head_turn_id TEXT,
    ancestry TEXT NOT NULL,
    protected INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    abandoned_future INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS turns (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    parent_turn_id TEXT,
    actor TEXT NOT NULL,
    created_at TEXT NOT NULL,
    finalized_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (branch_id) REFERENCES branches(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    turn_id TEXT,
    sequence_num INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL,
    causation_id TEXT,
    correlation_id TEXT,
    actor TEXT NOT NULL,
    checksum TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (branch_id) REFERENCES branches(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS checkpoints (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    turn_id TEXT,
    manifest_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (branch_id) REFERENCES branches(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS manifests (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    checkpoint_id TEXT NOT NULL,
    manifest_data TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS blobs (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    hash TEXT NOT NULL UNIQUE,
    size INTEGER NOT NULL,
    content BLOB NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);
CREATE INDEX IF NOT EXISTS idx_events_branch ON events(branch_id);
CREATE INDEX IF NOT EXISTS idx_events_sequence ON events(session_id, sequence_num);
CREATE INDEX IF NOT EXISTS idx_checkpoints_session ON checkpoints(session_id);
CREATE INDEX IF NOT EXISTS idx_blobs_hash ON blobs(hash);

-- Materialized views for read model caching
CREATE TABLE IF NOT EXISTS materialized_views (
    view_key TEXT PRIMARY KEY,
    view_type TEXT NOT NULL,
    session_id TEXT NOT NULL,
    branch_id TEXT,
    data BLOB NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

CREATE INDEX IF NOT EXISTS idx_views_session ON materialized_views(session_id);
CREATE INDEX IF NOT EXISTS idx_views_type ON materialized_views(view_type);

-- Cursor position tracking for timeline navigation
CREATE TABLE IF NOT EXISTS cursor_position (
    session_id TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (session_id, branch_id),
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    FOREIGN KEY (branch_id) REFERENCES branches(id)
);

-- App-level state for active session/branch tracking
CREATE TABLE IF NOT EXISTS app_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- First-class message records for replay-safe message ordering
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    turn_id TEXT NOT NULL,
    message_type TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata TEXT NOT NULL,
    sequence_num INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (turn_id) REFERENCES turns(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_messages_turn ON messages(turn_id);
CREATE INDEX IF NOT EXISTS idx_messages_sequence ON messages(sequence_num);

-- Context snapshots for reproducibility tracking (task 12)
CREATE TABLE IF NOT EXISTS context_snapshots (
    id TEXT PRIMARY KEY,
    turn_id TEXT NOT NULL,
    snapshot_data TEXT NOT NULL,
    selected_files_count INTEGER NOT NULL DEFAULT 0,
    retrieved_chunks_count INTEGER NOT NULL DEFAULT 0,
    tool_outputs_count INTEGER NOT NULL DEFAULT 0,
    summary_refs_count INTEGER NOT NULL DEFAULT 0,
    memory_refs_count INTEGER NOT NULL DEFAULT 0,
    rule_refs_count INTEGER NOT NULL DEFAULT 0,
    system_prompt_hash TEXT,
    model_config_data TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (turn_id) REFERENCES turns(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_context_snapshots_turn ON context_snapshots(turn_id);
CREATE INDEX IF NOT EXISTS idx_context_snapshots_created ON context_snapshots(created_at);