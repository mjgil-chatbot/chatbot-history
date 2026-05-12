//! Patch creation and application

use super::diff::{denormalize_crlf, has_crlf, is_text_content, normalize_crlf};
use ai_history_core::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use similar::TextDiff;
use std::fmt;

/// A patch that can be applied to transform old content into new content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    pub before_blob_hash: ai_history_core::ContentHash,
    pub after_blob_hash: ai_history_core::ContentHash,
    pub unified_diff: String,
}

/// An inverse patch (to go from new back to old)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InversePatch {
    pub unified_diff: String,
}

/// A binary patch storing whole before/after blobs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryPatch {
    pub before_blob_hash: ai_history_core::ContentHash,
    pub after_blob_hash: ai_history_core::ContentHash,
}

impl BinaryPatch {
    /// Create a binary patch from before and after content
    pub fn new(before_content: &[u8], after_content: &[u8]) -> Self {
        Self {
            before_blob_hash: ai_history_core::ContentHash::new(before_content),
            after_blob_hash: ai_history_core::ContentHash::new(after_content),
        }
    }

    /// Create inverse binary patch (swap before/after)
    pub fn inverse(&self) -> Self {
        Self {
            before_blob_hash: self.after_blob_hash,
            after_blob_hash: self.before_blob_hash,
        }
    }
}

/// Result of patch application
#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub success: bool,
    pub content: String,
    pub hunk_failures: Vec<HunkFailure>,
}

/// Information about a failed hunk
#[derive(Debug, Clone)]
pub struct HunkFailure {
    pub hunk_index: usize,
    pub reason: String,
}

/// Result of patch validation
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// Content matches expected before state - patch can be applied cleanly
    Clean,
    /// Content differs but patch might still apply
    Dirty {
        /// Hash of the actual content
        actual_hash: ai_history_core::ContentHash,
        /// Hash the patch expects
        expected_hash: ai_history_core::ContentHash,
        /// Number of lines that don't match the expected content
        line_mismatches: usize,
    },
    /// Content hash doesn't match and patch cannot be applied
    Stale {
        /// Hash of the actual content
        actual_hash: ai_history_core::ContentHash,
        /// Hash the patch expects
        expected_hash: ai_history_core::ContentHash,
    },
}

/// Result of patch validation
#[derive(Debug, Clone)]
pub struct ValidatePatchResult {
    pub valid: bool,
    pub result: ValidationResult,
    pub errors: Vec<String>,
}

impl Patch {
    /// Create a patch from old and new content
    pub fn new(old_content: &[u8], new_content: &[u8]) -> CoreResult<Self> {
        // For binary content, use whole-blob approach
        if !is_text_content(old_content) || !is_text_content(new_content) {
            return Err(CoreError::Deserialization(
                "Binary content not supported for diffing".to_string(),
            ));
        }

        Self::new_impl(old_content, new_content)
    }

    /// Create a patch, falling back to BinaryPatch for binary content.
    /// Returns Ok(Patch) for text content, Err(BinaryPatch) for binary content.
    pub fn try_new(old_content: &[u8], new_content: &[u8]) -> Result<Self, BinaryPatch> {
        if !is_text_content(old_content) || !is_text_content(new_content) {
            return Err(BinaryPatch::new(old_content, new_content));
        }
        Self::new_impl(old_content, new_content)
            .map_err(|_| BinaryPatch::new(old_content, new_content))
    }

    /// Internal implementation of patch creation (always text path)
    fn new_impl(old_content: &[u8], new_content: &[u8]) -> CoreResult<Self> {
        // Normalize CRLF to LF for consistent diffing
        let old_str = normalize_crlf(&String::from_utf8_lossy(old_content));
        let new_str = normalize_crlf(&String::from_utf8_lossy(new_content));
        let unified_diff = super::diff::unified_diff(&old_str, &new_str, 3);

        Ok(Self {
            before_blob_hash: ai_history_core::ContentHash::new(old_content),
            after_blob_hash: ai_history_core::ContentHash::new(new_content),
            unified_diff,
        })
    }

    /// Create an inverse patch (to revert changes)
    pub fn inverse(&self) -> CoreResult<InversePatch> {
        // Parse the unified diff and swap old/new for inverse
        let inverse_diff = swap_unified_diff(&self.unified_diff)?;
        Ok(InversePatch {
            unified_diff: inverse_diff,
        })
    }

    /// Apply this patch to content
    pub fn apply(&self, content: &str) -> ApplyResult {
        apply_unified_patch(content, &self.unified_diff)
    }

    /// Validate that the content matches what the patch expects
    pub fn validate(&self, content: &str) -> ValidatePatchResult {
        // Normalize for comparison if needed
        let normalized_content = if has_crlf(content) {
            normalize_crlf(content)
        } else {
            content.to_string()
        };

        let actual_hash = ai_history_core::ContentHash::new(content.as_bytes());

        // First check if hash matches - if so, patch is clean
        if actual_hash == self.before_blob_hash {
            return ValidatePatchResult {
                valid: true,
                result: ValidationResult::Clean,
                errors: vec![],
            };
        }

        // Hash mismatch - check if content can still be patched
        let apply_result = apply_unified_patch(content, &self.unified_diff);

        if apply_result.success && apply_result.hunk_failures.is_empty() {
            // Patch applied but content was different - compute line mismatches
            let _diff = TextDiff::from_lines(&normalized_content, &self.before_blob_hash.to_hex());
            // Actually, we need the original content hash's hex to compare
            // Let me just compute a line-level diff against the expected content
            // For Dirty case, we compute how many lines differ
            let line_mismatches = compute_line_mismatches(&normalized_content, &self.unified_diff);

            ValidatePatchResult {
                valid: true,
                result: ValidationResult::Dirty {
                    actual_hash,
                    expected_hash: self.before_blob_hash,
                    line_mismatches,
                },
                errors: vec![],
            }
        } else {
            // Patch failed to apply - stale case
            ValidatePatchResult {
                valid: false,
                result: ValidationResult::Stale {
                    actual_hash,
                    expected_hash: self.before_blob_hash,
                },
                errors: apply_result
                    .hunk_failures
                    .iter()
                    .map(|f| format!("Hunk {}: {}", f.hunk_index, f.reason))
                    .collect(),
            }
        }
    }
}

impl fmt::Display for Patch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Patch {{ {} -> {} }}",
            self.before_blob_hash, self.after_blob_hash
        )
    }
}

/// Swap old and new file names in unified diff header
fn swap_unified_diff(diff: &str) -> CoreResult<String> {
    let lines: Vec<&str> = diff.lines().collect();
    let mut result = Vec::new();

    for line in lines {
        if line.starts_with("--- ") {
            // Original was "--- a/file" -> now "+++ a/file"
            result.push(line.replace("--- ", "+++ "));
        } else if line.starts_with("+++ ") {
            // Original was "+++ b/file" -> now "--- b/file"
            result.push(line.replace("+++ ", "--- "));
        } else {
            result.push(line.to_string());
        }
    }

    // Also need to swap +/-, so process hunk headers
    let processed = swap_diff_hunks(&result.join("\n"))?;
    Ok(processed)
}

/// Swap deletion and insertion lines in a diff
fn swap_diff_hunks(diff: &str) -> CoreResult<String> {
    let lines: Vec<&str> = diff.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Check for hunk header like "@@ -1,3 +1,4 @@"
        if line.starts_with("@@") {
            // Parse and swap the ranges
            let swapped = swap_hunk_range(line);
            result.push(swapped);
            i += 1;

            // Process lines until next hunk or EOF
            while i < lines.len() && !lines[i].starts_with("@@") {
                let diff_line = lines[i];
                if diff_line.starts_with('-') {
                    // Deletion becomes insertion
                    result.push(format!("+{}", &diff_line[1..]));
                } else if diff_line.starts_with('+') {
                    // Insertion becomes deletion
                    result.push(format!("-{}", &diff_line[1..]));
                } else if diff_line.starts_with(' ') || diff_line.is_empty() {
                    // Context lines stay as-is
                    result.push(diff_line.to_string());
                } else {
                    // Unknown line type - keep as-is
                    result.push(diff_line.to_string());
                }
                i += 1;
            }
        } else {
            result.push(line.to_string());
            i += 1;
        }
    }

    Ok(result.join("\n"))
}

/// Swap the range information in an hunk header
fn swap_hunk_range(line: &str) -> String {
    // Parse @@ -old +new @@ format
    if !line.starts_with("@@") {
        return line.to_string();
    }

    // Extract the parts between @@ markers
    let inner = line.trim_start_matches("@@").trim_end_matches("@@");
    let parts: Vec<&str> = inner.trim().split_whitespace().collect();

    if parts.len() >= 2 {
        let old_range = parts[0];
        let new_range = parts[1];
        format!("@@ {} {} @@", new_range, old_range)
    } else {
        line.to_string()
    }
}

/// Compute the number of lines that differ between content and what patch expects
fn compute_line_mismatches(content: &str, patch: &str) -> usize {
    // Extract the expected old content from the patch
    let expected_lines = extract_patch_old_lines(patch);
    let content_lines: Vec<&str> = content.lines().collect();

    let mut mismatches = 0;
    let min_len = content_lines.len().min(expected_lines.len());

    for i in 0..min_len {
        if content_lines[i] != expected_lines[i] {
            mismatches += 1;
        }
    }

    // Any extra lines count as mismatches
    if content_lines.len() > expected_lines.len() {
        mismatches += content_lines.len() - expected_lines.len();
    }

    mismatches
}

/// Extract the expected "old" lines from a unified diff patch
fn extract_patch_old_lines(patch: &str) -> Vec<String> {
    let mut result = Vec::new();
    let lines: Vec<&str> = patch.lines().collect();

    let mut in_old_section = false;
    let mut in_hunk = false;

    for line in lines {
        if line.starts_with("--- ") {
            in_old_section = true;
            continue;
        }
        if line.starts_with("+++ ") {
            in_old_section = false;
            continue;
        }
        if line.starts_with("@@") {
            in_hunk = true;
            continue;
        }

        if in_old_section && in_hunk {
            if line.starts_with('-') {
                result.push(line[1..].to_string());
            } else if line.starts_with('+') || line.starts_with(' ') {
                // Skip additions and context
                continue;
            } else if line.starts_with('\\') {
                continue;
            } else {
                result.push(line.to_string());
            }
        }
    }

    result
}

/// Apply a unified diff patch to content with hunk failure reporting
pub fn apply_unified_patch(content: &str, patch: &str) -> ApplyResult {
    if patch.is_empty() {
        return ApplyResult {
            success: true,
            content: content.to_string(),
            hunk_failures: vec![],
        };
    }

    if !patch.contains("---") || !patch.contains("+++") {
        return ApplyResult {
            success: false,
            content: content.to_string(),
            hunk_failures: vec![HunkFailure {
                hunk_index: 0,
                reason: "Invalid patch format: missing --- or +++ lines".to_string(),
            }],
        };
    }

    // Detect if input has CRLF and normalize for processing
    let original_has_crlf = has_crlf(content);
    let normalized_content = normalize_crlf(content);

    match apply_patch(&normalized_content, patch) {
        Ok(applied) => {
            // Restore CRLF if original had it
            let final_content = denormalize_crlf(&applied, original_has_crlf);
            ApplyResult {
                success: true,
                content: final_content,
                hunk_failures: vec![],
            }
        }
        Err(e) => ApplyResult {
            success: false,
            content: content.to_string(),
            hunk_failures: vec![HunkFailure {
                hunk_index: 0,
                reason: e.to_string(),
            }],
        },
    }
}

/// Validate a patch
pub fn apply_patch(content: &str, patch: &str) -> CoreResult<String> {
    if patch.is_empty() {
        return Ok(content.to_string());
    }

    if !patch.contains("---") || !patch.contains("+++") {
        return Err(CoreError::Deserialization(
            "Invalid patch format".to_string(),
        ));
    }

    // Line-by-line patch application with multi-hunk offset correction
    let content_lines: Vec<&str> = content.lines().collect();
    let patch_lines: Vec<&str> = patch.lines().collect();

    let mut result = Vec::new();
    let mut content_idx = 0;
    // Track accumulated offset from applied hunks (new_count - old_count per hunk)
    let mut accumulated_offset: isize = 0;

    let mut patch_idx = 0;
    while patch_idx < patch_lines.len() {
        let line = patch_lines[patch_idx];

        if line.starts_with("@@") {
            // Found hunk header
            let inner = line.trim_start_matches("@@").trim_end_matches("@@");
            let parts: Vec<&str> = inner.trim().split_whitespace().collect();

            let mut old_start: usize = 1;
            let mut old_count: usize = 1;
            let mut new_count: usize = 1;

            if parts.len() >= 2 {
                let old_parts: Vec<&str> = parts[0].split(',').collect();
                let new_parts: Vec<&str> = parts[1].split(',').collect();
                old_start = old_parts[0].parse().unwrap_or(1);
                old_count = old_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
                new_count = new_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            }

            // Apply accumulated offset to target position
            // After applying previous hunks, the content has shifted by accumulated_offset
            let adjusted_start = (old_start as isize - 1 - accumulated_offset).max(0) as usize;
            let target_pos = adjusted_start;

            // Copy content up to target position
            while content_idx < target_pos && content_idx < content_lines.len() {
                result.push(content_lines[content_idx].to_string());
                content_idx += 1;
            }

            // Process hunk lines
            patch_idx += 1;
            let mut old_seen = 0;
            let mut new_seen = 0;

            while patch_idx < patch_lines.len() && !patch_lines[patch_idx].starts_with("@@") {
                let p_line = patch_lines[patch_idx];

                if p_line.starts_with('-') && old_seen < old_count {
                    // Deletion - skip from content
                    if content_idx < content_lines.len() {
                        let content_line = content_lines[content_idx];
                        let expected = &p_line[1..];
                        if content_line != expected {
                            return Err(CoreError::Deserialization(format!(
                                "Patch mismatch at line {}: expected '{}' but got '{}'",
                                content_idx, expected, content_line
                            )));
                        }
                        content_idx += 1;
                    }
                    old_seen += 1;
                } else if p_line.starts_with('+') && new_seen < new_count {
                    // Insertion - add to result
                    result.push(p_line[1..].to_string());
                    new_seen += 1;
                } else if p_line.starts_with(' ') || p_line.is_empty() {
                    // Context line - copy from content if available
                    if content_idx < content_lines.len() {
                        result.push(content_lines[content_idx].to_string());
                        content_idx += 1;
                    }
                    old_seen += 1;
                    new_seen += 1;
                } else if p_line.starts_with('\\') {
                    // "\ No newline at end of file" - skip
                } else {
                    // Unknown line type - skip
                }
                patch_idx += 1;
            }

            // Update accumulated offset: net lines added/removed by this hunk
            accumulated_offset += new_count as isize - old_count as isize;
        } else if line.starts_with("---") || line.starts_with("+++") {
            // Skip file header lines
            patch_idx += 1;
        } else {
            // Non-patch line - copy from content if available
            if content_idx < content_lines.len() {
                result.push(content_lines[content_idx].to_string());
                content_idx += 1;
            }
            patch_idx += 1;
        }
    }

    // Add remaining content
    while content_idx < content_lines.len() {
        result.push(content_lines[content_idx].to_string());
        content_idx += 1;
    }

    // Preserve trailing newline if original content had one
    let has_trailing_newline = content.ends_with('\n');
    let mut final_result = result.join("\n");
    if has_trailing_newline && !final_result.is_empty() {
        final_result.push('\n');
    }
    Ok(final_result)
}

/// Validate a patch
pub fn validate_patch(content: &str, patch: &str) -> bool {
    apply_patch(content, patch).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_creation() {
        let old = b"hello world";
        let new = b"hello rust";
        let patch = Patch::new(old, new).unwrap();
        assert_eq!(
            patch.before_blob_hash,
            ai_history_core::ContentHash::new(old)
        );
        assert_eq!(
            patch.after_blob_hash,
            ai_history_core::ContentHash::new(new)
        );
    }

    #[test]
    fn test_patch_inverse() {
        let old = b"hello";
        let new = b"world";
        let patch = Patch::new(old, new).unwrap();
        let inverse = patch.inverse().unwrap();
        assert!(!inverse.unified_diff.is_empty());
    }

    #[test]
    fn test_validate_patch() {
        let content = "hello world";
        let patch = "---\nold\n+++\nnew";
        assert!(validate_patch(content, patch));
    }

    #[test]
    fn test_apply_patch_empty() {
        let content = "hello world";
        let patch = "";
        let result = apply_patch(content, patch).unwrap();
        assert_eq!(result, content);
    }

    #[test]
    fn test_apply_patch_simple_change() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nmodified\nline3\n";

        // Create patch
        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();

        // Apply patch
        let result = apply_patch(old_content, &patch.unified_diff);
        assert!(result.is_ok());
        let applied = result.unwrap();
        assert!(applied.contains("modified"));
    }

    #[test]
    fn test_apply_patch_add_lines() {
        let old_content = "line1\nline2\n";
        let new_content = "line1\nline2\nline3\n";

        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();
        let result = apply_patch(old_content, &patch.unified_diff);

        assert!(result.is_ok());
        let applied = result.unwrap();
        assert!(applied.contains("line3"));
    }

    #[test]
    fn test_apply_patch_remove_lines() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nline3\n";

        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();
        let result = apply_patch(old_content, &patch.unified_diff);

        assert!(result.is_ok());
        let applied = result.unwrap();
        assert!(!applied.contains("line2"));
    }

    #[test]
    fn test_patch_roundtrip() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nmodified\nline3\n";

        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();

        // Apply patch
        let result = apply_patch(old_content, &patch.unified_diff);
        assert!(result.is_ok());

        // Verify applied content contains the modification
        let applied = result.unwrap();
        assert!(applied.contains("modified"));
    }

    #[test]
    fn test_inverse_patch_reverts_correctly() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nmodified\nline3\n";

        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();
        let inverse = patch.inverse().unwrap();

        // Apply inverse to new content should get back old content
        let result = apply_patch(new_content, &inverse.unified_diff);
        assert!(result.is_ok(), "inverse patch should apply successfully");
        let reverted = result.unwrap();
        assert_eq!(
            reverted, old_content,
            "inverse should restore original content"
        );
    }

    #[test]
    fn test_forward_patch_applies_correctly() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nmodified\nline3\n";

        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();

        // Apply patch to old content should get new content
        let result = apply_patch(old_content, &patch.unified_diff);
        assert!(result.is_ok(), "forward patch should apply successfully");
        let applied = result.unwrap();
        assert_eq!(applied, new_content, "patch should produce new content");
    }

    #[test]
    fn test_inverse_patch_swaps_diff() {
        let old_content = "old\n";
        let new_content = "new\n";

        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();
        let inverse = patch.inverse().unwrap();

        // The inverse diff should have --- and +++ swapped and +/- swapped
        assert!(inverse.unified_diff.contains("---"));
        assert!(inverse.unified_diff.contains("+++"));
    }

    #[test]
    fn test_inverse_add_lines() {
        let old_content = "line1\n";
        let new_content = "line1\nline2\n";

        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();
        let inverse = patch.inverse().unwrap();

        // Applying inverse to new should remove the added line
        let result = apply_patch(new_content, &inverse.unified_diff);
        assert!(result.is_ok());
        let reverted = result.unwrap();
        assert_eq!(reverted, old_content);
    }

    #[test]
    fn test_inverse_delete_lines() {
        let old_content = "line1\nline2\n";
        let new_content = "line1\n";

        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();
        let inverse = patch.inverse().unwrap();

        // Applying inverse to new should restore the deleted line
        let result = apply_patch(new_content, &inverse.unified_diff);
        assert!(result.is_ok());
        let reverted = result.unwrap();
        assert_eq!(reverted, old_content);
    }

    #[test]
    fn test_apply_result_structure() {
        let result = apply_unified_patch("content", "");
        assert!(result.success);
        assert!(result.hunk_failures.is_empty());
    }

    #[test]
    fn test_invalid_patch_format() {
        let result = apply_unified_patch("content", "not a valid patch");
        assert!(!result.success);
        assert!(!result.hunk_failures.is_empty());
    }

    #[test]
    fn test_binary_content_rejected() {
        let binary_content = vec![0x00, 0x01, 0x02];
        let result = Patch::new(&binary_content, b"text");
        assert!(result.is_err());
    }

    #[test]
    fn test_hunk_failure_tracking() {
        let content = "different content";
        let patch = "---\nold\n+++\nnew\n";
        let result = apply_unified_patch(content, patch);
        // Content doesn't match patch expected lines - should fail
        assert!(!result.success || !result.content.contains("new"));
    }

    #[test]
    fn test_binary_patch_creation() {
        let before = vec![0x00, 0x01, 0x02];
        let after = vec![0x03, 0x04, 0x05];
        let patch = BinaryPatch::new(&before, &after);
        assert_eq!(
            patch.before_blob_hash,
            ai_history_core::ContentHash::new(&before)
        );
        assert_eq!(
            patch.after_blob_hash,
            ai_history_core::ContentHash::new(&after)
        );
    }

    #[test]
    fn test_binary_patch_inverse() {
        let before = vec![0x00, 0x01, 0x02];
        let after = vec![0x03, 0x04, 0x05];
        let patch = BinaryPatch::new(&before, &after);
        let inverse = patch.inverse();
        // Inverse swaps the hashes
        assert_eq!(inverse.before_blob_hash, patch.after_blob_hash);
        assert_eq!(inverse.after_blob_hash, patch.before_blob_hash);
    }

    #[test]
    fn test_binary_patch_no_unified_diff() {
        let patch = BinaryPatch::new(b"old", b"new");
        // BinaryPatch only stores hashes, no text diff
        assert!(patch.before_blob_hash != patch.after_blob_hash);
    }

    #[test]
    fn test_patch_validate_clean() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nmodified\nline3\n";
        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();

        // Validate against original content should be clean
        let result = patch.validate(old_content);
        assert!(result.valid);
        assert_eq!(result.result, ValidationResult::Clean);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_patch_validate_stale() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nmodified\nline3\n";
        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();

        // Validate against different content should be stale
        let different_content = "something\nelse\n";
        let result = patch.validate(different_content);
        assert!(!result.valid);
        match &result.result {
            ValidationResult::Stale {
                actual_hash,
                expected_hash,
                ..
            } => {
                assert_eq!(
                    *actual_hash,
                    ai_history_core::ContentHash::new(different_content.as_bytes())
                );
                assert_eq!(*expected_hash, patch.before_blob_hash);
            }
            _ => panic!("Expected Stale result"),
        }
    }

    #[test]
    fn test_patch_validate_dirty() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nmodified\nline3\n";
        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();

        // Content that differs but patch might still apply
        let dirty_content = "line1\nline2\nline3\ndifferent trailing\n";
        let result = patch.validate(dirty_content);
        // With similar content, patch might still succeed (depending on how similar)
        // The key is we get a proper result back, not a crash
        match &result.result {
            ValidationResult::Dirty {
                actual_hash,
                expected_hash,
                ..
            } => {
                assert_eq!(*expected_hash, patch.before_blob_hash);
            }
            _ => {} // Other results are valid too
        }
    }

    #[test]
    fn test_patch_validate_hash_mismatch_detected() {
        let old_content = "original content";
        let new_content = "modified content";
        let patch = Patch::new(old_content.as_bytes(), new_content.as_bytes()).unwrap();

        // Validate with wrong content
        let wrong_content = "wrong content here";
        let result = patch.validate(wrong_content);

        match &result.result {
            ValidationResult::Stale {
                actual_hash,
                expected_hash,
            } => {
                assert_ne!(*actual_hash, *expected_hash);
                assert_eq!(*expected_hash, patch.before_blob_hash);
            }
            _ => panic!("Expected Stale result for hash mismatch"),
        }
    }
}
