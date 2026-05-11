//! Diff generation for text files

use similar::{ChangeTag, TextDiff};

/// Represents a diff between two text contents
#[derive(Debug, Clone)]
pub struct Diff {
    pub changes: Vec<DiffChange>,
    pub old_content: String,
    pub new_content: String,
}

/// A single change in a diff
#[derive(Debug, Clone, PartialEq)]
pub struct DiffChange {
    pub tag: ChangeKind,
    pub old_start: usize,
    pub old_end: usize,
    pub new_start: usize,
    pub new_end: usize,
    pub value: String,
}

/// Type of change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Delete,
    Insert,
    Equal,
}

impl Diff {
    /// Create a diff between two text contents
    pub fn new(old_content: &str, new_content: &str) -> Self {
        let diff = TextDiff::from_lines(old_content, new_content);
        let mut changes = Vec::new();
        let mut old_line = 0usize;
        let mut new_line = 0usize;

        for change in diff.iter_all_changes() {
            let (tag, count) = match change.tag() {
                ChangeTag::Delete => {
                    old_line += 1;
                    (ChangeKind::Delete, 1)
                }
                ChangeTag::Insert => {
                    new_line += 1;
                    (ChangeKind::Insert, 1)
                }
                ChangeTag::Equal => {
                    old_line += 1;
                    new_line += 1;
                    (ChangeKind::Equal, 1)
                }
            };

            let old_start = if tag == ChangeKind::Delete || tag == ChangeKind::Equal {
                old_line.saturating_sub(1)
            } else {
                old_line
            };

            let new_start = if tag == ChangeKind::Insert || tag == ChangeKind::Equal {
                new_line.saturating_sub(1)
            } else {
                new_line
            };

            changes.push(DiffChange {
                tag,
                old_start,
                old_end: old_start + count - 1,
                new_start,
                new_end: new_start + count - 1,
                value: change.value().to_string(),
            });
        }

        Self {
            changes,
            old_content: old_content.to_string(),
            new_content: new_content.to_string(),
        }
    }

    /// Check if the diff has any changes
    pub fn has_changes(&self) -> bool {
        self.changes.iter().any(|c| c.tag != ChangeKind::Equal)
    }

    /// Get the number of changed lines
    pub fn change_count(&self) -> usize {
        self.changes
            .iter()
            .filter(|c| c.tag != ChangeKind::Equal)
            .count()
    }
}

/// Check if content is likely text (not binary)
pub fn is_text_content(content: &[u8]) -> bool {
    // Check for null bytes which indicate binary
    if content.contains(&b'\x00') {
        return false;
    }
    // Also check for valid UTF-8 (invalid sequences indicate binary)
    std::str::from_utf8(content).is_ok()
}

/// Normalize line endings: convert CRLF (\r\n) to LF (\n)
pub fn normalize_crlf(content: &str) -> String {
    content.replace("\r\n", "\n").replace("\r", "\n")
}

/// Check if content contains CRLF line endings
pub fn has_crlf(content: &str) -> bool {
    content.contains("\r\n")
}

/// Restore CRLF line endings after patch application
/// If original had CRLF, convert LF back to CRLF
pub fn denormalize_crlf(content: &str, original_has_crlf: bool) -> String {
    if original_has_crlf {
        content.replace("\n", "\r\n")
    } else {
        content.to_string()
    }
}

/// Unified diff format output
pub fn unified_diff(old: &str, new: &str, context: usize) -> String {
    let diff = TextDiff::from_lines(old, new);
    diff.unified_diff()
        .context_radius(context)
        .header("old", "new")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_creation() {
        let diff = Diff::new("hello", "hello world");
        assert!(diff.has_changes());
        assert_eq!(diff.change_count(), 2); // "hello" + " world" as separate changes
    }

    #[test]
    fn test_diff_no_changes() {
        let diff = Diff::new("hello", "hello");
        assert!(!diff.has_changes());
        assert_eq!(diff.change_count(), 0);
    }

    #[test]
    fn test_is_text_content() {
        assert!(is_text_content(b"hello world"));
        assert!(!is_text_content(b"hello\x00world"));
    }

    #[test]
    fn test_unified_diff() {
        let diff = unified_diff("old line\nfoo\nbar", "new line\nfoo\nbaz", 3);
        assert!(diff.contains("---"));
        assert!(diff.contains("+++"));
    }

    #[test]
    fn test_normalize_crlf() {
        let input = "line1\r\nline2\r\nline3\r\n";
        let result = normalize_crlf(input);
        assert!(!result.contains("\r\n"));
        assert_eq!(result, "line1\nline2\nline3\n");
    }

    #[test]
    fn test_normalize_cr_only() {
        let input = "line1\rline2\rline3\r";
        let result = normalize_crlf(input);
        assert!(!result.contains('\r'));
        assert_eq!(result, "line1\nline2\nline3\n");
    }

    #[test]
    fn test_has_crlf() {
        assert!(has_crlf("line1\r\nline2"));
        assert!(!has_crlf("line1\nline2"));
    }

    #[test]
    fn test_denormalize_crlf() {
        let input = "line1\nline2\n";
        let result = denormalize_crlf(input, true);
        assert!(result.contains("\r\n"));
        assert_eq!(result, "line1\r\nline2\r\n");
    }

    #[test]
    fn test_denormalize_no_op_when_not_crlf() {
        let input = "line1\nline2\n";
        let result = denormalize_crlf(input, false);
        assert_eq!(result, "line1\nline2\n");
    }
}
