//! CLI commands implementation

use ai_history_core::{BranchId, CheckpointId, SessionId};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// AI History CLI
#[derive(Parser)]
#[command(name = "ai-history")]
#[command(about = "AI coding assistant history and rollback system", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Workspace root directory
    #[arg(short, long, default_value = ".")]
    pub workspace: PathBuf,

    /// Session ID (defaults to discovering from workspace)
    #[arg(short, long)]
    pub session_id: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new history session
    Init {
        /// Session title
        #[arg(short, long)]
        title: Option<String>,
    },

    /// Create a named checkpoint
    Checkpoint {
        /// Checkpoint name
        #[arg(short, long)]
        name: Option<String>,

        /// Checkpoint reason
        #[arg(short, long, default_value = "manual")]
        reason: String,
    },

    /// List all checkpoints
    ListCheckpoints {
        /// Branch to list checkpoints for
        #[arg(short, long, default_value = "main")]
        branch: String,
    },

    /// Restore from a checkpoint
    Restore {
        /// Checkpoint ID to restore from
        checkpoint_id: String,

        /// Restore mode
        #[arg(short, long, value_enum, default_value = "both")]
        mode: RestoreModeCli,
    },

    /// Fork a new branch from a checkpoint
    Fork {
        /// Checkpoint ID to fork from
        checkpoint_id: String,

        /// New branch name
        #[arg(short, long)]
        name: String,
    },

    /// List all branches
    Branches,

    /// Show diff between current and checkpoint
    Diff {
        /// Checkpoint ID to diff against
        #[arg(short, long)]
        checkpoint_id: String,

        /// Show only changed files
        #[arg(short, long)]
        files_only: bool,
    },

    /// Show current status
    Status,

    /// Run garbage collection
    Gc {
        /// Actually delete files (dry run by default)
        #[arg(short, long)]
        execute: bool,

        /// Protected checkpoint IDs (can be specified multiple times)
        #[arg(short, long)]
        protected: Vec<String>,
    },

    /// Run integrity checks
    Doctor {
        /// Fix issues if possible
        #[arg(short, long)]
        fix: bool,
    },

    /// Detect and recover from interrupted operations
    Recover {
        /// Actually perform recovery (default is dry-run)
        #[arg(short, long)]
        execute: bool,
    },

    /// Navigate to previous turn in history
    NavigateBack {
        /// Number of turns to go back
        #[arg(default_value = "1")]
        count: Option<u32>,
    },

    /// Navigate to next turn in history (if available)
    NavigateForward {
        /// Number of turns to go forward
        #[arg(default_value = "1")]
        count: Option<u32>,
    },

    /// Fork a new branch from the current head (no cursor)
    ForkCurrent {
        /// New branch name
        #[arg(short, long)]
        name: String,
    },

    /// Fork a new branch from a specific turn (cursor-based)
    ForkFromCursor {
        /// Turn ID to fork from
        turn_id: String,

        /// New branch name
        #[arg(short, long)]
        name: String,
    },

    /// Continue from a cursor position with new prompt
    ContinueFromCursor {
        /// Turn ID to continue from
        turn_id: String,

        /// New prompt content
        #[arg(short, long)]
        prompt: String,
    },

    /// Export session data to a file
    Export {
        /// Output file path
        #[arg(short, long)]
        output: String,

        /// Include blobs in export
        #[arg(short, long)]
        include_blobs: bool,
    },

    /// Import session data from a file
    Import {
        /// Input file path
        #[arg(short, long)]
        input: String,

        /// Target session ID (optional, creates new if not specified)
        #[arg(short, long)]
        session_id: Option<String>,
    },

    /// Verify session integrity and consistency
    Verify {
        /// Checkpoint ID to verify against (optional)
        checkpoint_id: Option<String>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum RestoreModeCli {
    ConversationOnly,
    WorkspaceOnly,
    #[default]
    Both,
    Fork,
}

/// Parse a session ID from string
pub fn parse_session_id(s: &str) -> Result<SessionId, String> {
    s.parse().map_err(|_| format!("Invalid session ID: {}", s))
}

/// Parse a checkpoint ID from string
pub fn parse_checkpoint_id(s: &str) -> Result<CheckpointId, String> {
    s.parse()
        .map_err(|_| format!("Invalid checkpoint ID: {}", s))
}

/// Parse a branch ID from string
pub fn parse_branch_id(s: &str) -> Result<BranchId, String> {
    s.parse().map_err(|_| format!("Invalid branch ID: {}", s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_session_id() {
        let id = SessionId::new();
        let result = parse_session_id(&id.to_string()).unwrap();
        assert_eq!(id, result);
    }

    #[test]
    fn test_parse_checkpoint_id() {
        let id = CheckpointId::new();
        let result = parse_checkpoint_id(&id.to_string()).unwrap();
        assert_eq!(id, result);
    }

    #[test]
    fn test_parse_branch_id() {
        let id = BranchId::new();
        let result = parse_branch_id(&id.to_string()).unwrap();
        assert_eq!(id, result);
    }

    #[test]
    fn test_parse_invalid_session_id() {
        let result = parse_session_id("not-a-valid-id");
        assert!(result.is_err());
    }
}
