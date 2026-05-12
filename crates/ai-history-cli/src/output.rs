//! CLI output formatting

use ai_history_core::BranchId;
use ai_history_core::CheckpointId;
use chrono::{DateTime, Utc};
/// Output formatter for CLI
pub struct OutputFormatter;
impl OutputFormatter {
    /// Format a checkpoint for display
    pub fn format_checkpoint(
        id: CheckpointId,
        reason: &str,
        file_count: usize,
        created: DateTime<Utc>,
        name: Option<&str>,
    ) -> String {
        let name_str = name.map(|n| format!(" [{}]", n)).unwrap_or_default();
        format!(
            "Checkpoint: {}{}\n  Reason: {}\n  Files: {}\n  Created: {}",
            id,
            name_str,
            reason,
            file_count,
            created.to_rfc3339()
        )
    }}
impl OutputFormatter {
    /// Format a branch for display
    pub fn format_branch(
        id: BranchId,
        name: &str,
        head_turn: Option<&str>,
        created: DateTime<Utc>,
        is_active: bool,
    ) -> String {
        let active_str = if is_active { " (active)" } else { "" };
        let head_str = head_turn
            .map(|h| format!("\n  Head: {}", h))
            .unwrap_or_default();
        format!(
            "Branch: {} {}{}{}\n  Created: {}",
            name,
            id,
            active_str,
            head_str,
            created.to_rfc3339()
        )
    }}
impl OutputFormatter {
    /// Format an error for display
    pub fn format_error(message: &str, context: Option<&str>) -> String {
        let ctx_str = context.map(|c| format!(" ({})", c)).unwrap_or_default();
        format!("Error: {}{}", message, ctx_str)
    }}
impl OutputFormatter {
    /// Format a success message
    pub fn format_success(message: &str) -> String {
        format!("Success: {}", message)
    }}
impl OutputFormatter {
    /// Format a warning message
    pub fn format_warning(message: &str) -> String {
        format!("Warning: {}", message)
    }}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_error() {
        let msg = OutputFormatter::format_error("Something went wrong", Some("init"));
        assert!(msg.contains("Error"));
        assert!(msg.contains("init"));
    }

    #[test]
    fn test_format_success() {
        let msg = OutputFormatter::format_success("Checkpoint created");
        assert!(msg.contains("Success"));
    }
}
