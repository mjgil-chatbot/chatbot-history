//! CLI output formatting

use chrono::{DateTime, Utc};
/// Output formatter for CLI
pub struct OutputFormatter;
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
