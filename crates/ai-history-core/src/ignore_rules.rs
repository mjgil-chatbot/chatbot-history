//! File inclusion and ignore rules for ai-history
//!
//! Defines:
//! - `.gitignore` support
//! - Product-specific ignore files
//! - Maximum file size limits
//! - Binary policies
//! - Generated file exclusions
//! - Explicit user overrides

use serde::{Deserialize, Serialize};

/// Ignore rule type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IgnoreRuleType {
    /// Pattern from .gitignore
    GitIgnore,
    /// Product-specific ignore
    ProductIgnore,
    /// User-specified override
    UserOverride,
    /// System default
    SystemDefault,
}

/// An ignore rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreRule {
    pub pattern: String,
    pub rule_type: IgnoreRuleType,
    pub is_negation: bool,
}

impl IgnoreRule {
    pub fn new(pattern: &str, rule_type: IgnoreRuleType) -> Self {
        let is_negation = pattern.starts_with('!');
        let pattern = if is_negation {
            pattern.trim_start_matches('!')
        } else {
            pattern
        };

        Self {
            pattern: pattern.to_string(),
            rule_type,
            is_negation,
        }
    }

    pub fn gitignore(pattern: &str) -> Self {
        Self::new(pattern, IgnoreRuleType::GitIgnore)
    }

    pub fn product(pattern: &str) -> Self {
        Self::new(pattern, IgnoreRuleType::ProductIgnore)
    }

    pub fn user(pattern: &str) -> Self {
        Self::new(pattern, IgnoreRuleType::UserOverride)
    }
}

/// File size limit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSizeLimit {
    pub max_bytes: u64,
    pub is_hard_limit: bool,
}

impl FileSizeLimit {
    pub fn new(max_bytes: u64) -> Self {
        Self {
            max_bytes,
            is_hard_limit: true,
        }
    }

    pub fn is_within_limit(&self, size_bytes: u64) -> bool {
        size_bytes <= self.max_bytes
    }
}

/// File inclusion decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InclusionDecision {
    /// File should be included
    Include,
    /// File should be excluded
    Exclude,
    /// No decision (need more rules)
    Unknown,
}

/// File inclusion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInclusion {
    pub path: String,
    pub decision: InclusionDecision,
    pub matched_rule: Option<IgnoreRule>,
    pub reason: Option<String>,
}

impl FileInclusion {
    pub fn included(path: &str, reason: &str) -> Self {
        Self {
            path: path.to_string(),
            decision: InclusionDecision::Include,
            matched_rule: None,
            reason: Some(reason.to_string()),
        }
    }

    pub fn excluded(path: &str, rule: &IgnoreRule, reason: &str) -> Self {
        Self {
            path: path.to_string(),
            decision: InclusionDecision::Exclude,
            matched_rule: Some(rule.clone()),
            reason: Some(reason.to_string()),
        }
    }
}

/// Ignore rules configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreConfig {
    pub rules: Vec<IgnoreRule>,
    pub size_limit: FileSizeLimit,
    pub binary_policy: BinaryPolicy,
    pub generated_file_patterns: Vec<String>,
    pub max_depth: Option<usize>,
    pub follow_symlinks: bool,
}

impl Default for IgnoreConfig {
    fn default() -> Self {
        Self {
            rules: Self::default_rules(),
            size_limit: FileSizeLimit::new(10 * 1024 * 1024), // 10MB
            binary_policy: BinaryPolicy::Skip,
            generated_file_patterns: vec![
                "*.pyc".to_string(),
                "__pycache__".to_string(),
                "node_modules/.cache".to_string(),
                "target/debug".to_string(),
                "target/release".to_string(),
            ],
            max_depth: Some(100),
            follow_symlinks: false,
        }
    }
}

impl IgnoreConfig {
    fn default_rules() -> Vec<IgnoreRule> {
        vec![
            IgnoreRule::gitignore(".git"),
            IgnoreRule::gitignore("node_modules"),
            IgnoreRule::gitignore("target"),
            IgnoreRule::product(".ai-history-ignore"),
        ]
    }
}

/// Binary file policy for inclusion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryPolicy {
    Skip,
    Include,
    HashOnly,
}

impl Default for BinaryPolicy {
    fn default() -> Self {
        Self::Skip
    }
}

/// Ignore rules matcher
pub struct IgnoreMatcher;

impl IgnoreMatcher {
    /// Check if a path should be ignored
    pub fn is_ignored(path: &str, rules: &[IgnoreRule]) -> bool {
        let mut should_ignore = false;

        for rule in rules {
            if Self::matches_pattern(path, &rule.pattern) {
                if rule.is_negation {
                    // Negation rule means don't ignore
                    return false;
                } else {
                    should_ignore = true;
                }
            }
        }

        should_ignore
    }

    /// Check if path matches a pattern (simple glob matching)
    pub fn matches_pattern(path: &str, pattern: &str) -> bool {
        if pattern.ends_with('/') {
            // Directory pattern
            let dir_pattern = pattern.trim_end_matches('/');
            path.starts_with(dir_pattern) || path.contains(&format!("/{}/", dir_pattern))
        } else if pattern.contains('*') {
            // Simple glob pattern - convert * to match anything
            let glob_pattern = pattern.replace("**", "\0");
            let parts: Vec<&str> = glob_pattern.split('*').collect();

            let mut search_str = path;
            for part in &parts {
                if part.is_empty() {
                    continue;
                }
                if let Some(pos) = search_str.find(part) {
                    // Match found, continue searching in remainder
                    search_str = &search_str[pos + part.len()..];
                } else {
                    return false;
                }
            }
            true
        } else {
            // Simple name match - must match exact path or be a directory prefix
            path == pattern
                || path.starts_with(&format!("{}/", pattern))
                || path.contains(&format!("/{}/", pattern))
        }
    }

    /// Check if file is binary
    pub fn is_binary(path: &str) -> bool {
        let binary_extensions = [
            ".exe", ".dll", ".so", ".dylib", ".bin", ".o", ".a", ".png", ".jpg", ".jpeg", ".gif",
            ".bmp", ".ico", ".webp", ".mp3", ".mp4", ".avi", ".mov", ".wav", ".flac", ".zip",
            ".tar", ".gz", ".bz2", ".xz", ".rar", ".7z", ".pdf", ".doc", ".docx", ".xls", ".xlsx",
            ".ppt", ".pptx",
        ];

        binary_extensions.iter().any(|ext| path.ends_with(ext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignore_rule_new() {
        let rule = IgnoreRule::new("test.txt", IgnoreRuleType::GitIgnore);
        assert_eq!(rule.pattern, "test.txt");
        assert!(!rule.is_negation);
    }

    #[test]
    fn test_ignore_rule_negation() {
        let rule = IgnoreRule::new("!test.txt", IgnoreRuleType::GitIgnore);
        assert_eq!(rule.pattern, "test.txt");
        assert!(rule.is_negation);
    }

    #[test]
    fn test_ignore_rule_gitignore() {
        let rule = IgnoreRule::gitignore(".git");
        assert_eq!(rule.pattern, ".git");
        assert_eq!(rule.rule_type, IgnoreRuleType::GitIgnore);
    }

    #[test]
    fn test_ignore_rule_product() {
        let rule = IgnoreRule::product(".ai-history-ignore");
        assert_eq!(rule.pattern, ".ai-history-ignore");
        assert_eq!(rule.rule_type, IgnoreRuleType::ProductIgnore);
    }

    #[test]
    fn test_ignore_rule_user() {
        let rule = IgnoreRule::user("custom_pattern");
        assert_eq!(rule.pattern, "custom_pattern");
        assert_eq!(rule.rule_type, IgnoreRuleType::UserOverride);
    }

    #[test]
    fn test_file_size_limit() {
        let limit = FileSizeLimit::new(1000);
        assert!(limit.is_within_limit(500));
        assert!(limit.is_within_limit(1000));
        assert!(!limit.is_within_limit(1001));
    }

    #[test]
    fn test_file_inclusion_included() {
        let inclusion = FileInclusion::included("src/main.rs", "matches include rule");
        assert_eq!(inclusion.decision, InclusionDecision::Include);
        assert!(inclusion.reason.is_some());
    }

    #[test]
    fn test_file_inclusion_excluded() {
        let rule = IgnoreRule::gitignore("target");
        let inclusion = FileInclusion::excluded("target/debug", &rule, "matches ignore rule");
        assert_eq!(inclusion.decision, InclusionDecision::Exclude);
        assert!(inclusion.matched_rule.is_some());
    }

    #[test]
    fn test_ignore_matcher_is_ignored() {
        let rules = vec![
            IgnoreRule::gitignore("target"),
            IgnoreRule::gitignore("node_modules"),
        ];

        assert!(IgnoreMatcher::is_ignored("target/debug", &rules));
        assert!(IgnoreMatcher::is_ignored("node_modules/package", &rules));
        assert!(!IgnoreMatcher::is_ignored("src/main.rs", &rules));
    }

    #[test]
    fn test_ignore_matcher_is_binary() {
        assert!(IgnoreMatcher::is_binary("image.png"));
        assert!(IgnoreMatcher::is_binary("archive.zip"));
        assert!(!IgnoreMatcher::is_binary("document.txt"));
        assert!(!IgnoreMatcher::is_binary("code.rs"));
    }

    #[test]
    fn test_ignore_config_default() {
        let config = IgnoreConfig::default();
        assert!(!config.rules.is_empty());
        assert!(config.max_depth.is_some());
        assert!(!config.follow_symlinks);
    }

    #[test]
    fn test_ignore_matcher_glob_patterns() {
        let rules = vec![IgnoreRule::gitignore("*.log")];

        // *.log matches any .log file at any level
        assert!(IgnoreMatcher::is_ignored("debug.log", &rules));
        assert!(IgnoreMatcher::is_ignored("error.log", &rules));
        assert!(!IgnoreMatcher::is_ignored("main.rs", &rules));

        // test that nested paths work with simple patterns
        assert!(IgnoreMatcher::is_ignored("logs/debug.log", &rules));
    }

    #[test]
    fn test_ignore_matcher_negation_override() {
        let rules = vec![
            IgnoreRule::gitignore("*.log"),
            IgnoreRule::gitignore("!important.log"),
        ];

        // important.log should NOT be ignored due to negation
        assert!(!IgnoreMatcher::is_ignored("important.log", &rules));
        // But other .log files should still be ignored
        assert!(IgnoreMatcher::is_ignored("debug.log", &rules));
    }

    #[test]
    fn test_ignore_matcher_directory_pattern() {
        let rules = vec![IgnoreRule::gitignore("build/")];

        // Directory pattern should match the directory and its contents
        assert!(IgnoreMatcher::is_ignored("build/", &rules));
        assert!(IgnoreMatcher::is_ignored("build/output", &rules));
        assert!(IgnoreMatcher::is_ignored("build/temp/file.txt", &rules));
        assert!(!IgnoreMatcher::is_ignored("src/main.rs", &rules));
    }

    #[test]
    fn test_ignore_matcher_hidden_files() {
        let rules = vec![IgnoreRule::gitignore(".git"), IgnoreRule::gitignore(".env")];

        // Hidden files starting with . should be matched
        assert!(IgnoreMatcher::is_ignored(".git", &rules));
        assert!(IgnoreMatcher::is_ignored(".git/config", &rules));
        assert!(IgnoreMatcher::is_ignored(".env", &rules));
        assert!(!IgnoreMatcher::is_ignored("src/main.rs", &rules));
    }

    #[test]
    fn test_binary_policy_skip() {
        let policy = BinaryPolicy::Skip;
        assert!(matches!(policy, BinaryPolicy::Skip));
    }

    #[test]
    fn test_binary_policy_include() {
        let policy = BinaryPolicy::Include;
        assert!(matches!(policy, BinaryPolicy::Include));
    }

    #[test]
    fn test_binary_policy_hash_only() {
        let policy = BinaryPolicy::HashOnly;
        assert!(matches!(policy, BinaryPolicy::HashOnly));
    }

    #[test]
    fn test_ignore_matcher_precedence_order() {
        // Rules are evaluated in order; first matching rule wins
        let rules = vec![
            IgnoreRule::gitignore("target"),
            IgnoreRule::user("!target/special"), // Negation to un-ignore special files
        ];

        // target/debug should be ignored
        assert!(IgnoreMatcher::is_ignored("target/debug", &rules));
        // target/special should NOT be ignored due to negation
        assert!(!IgnoreMatcher::is_ignored("target/special", &rules));
    }

    #[test]
    fn test_file_size_limit_hard_limit() {
        let limit = FileSizeLimit::new(1024);
        assert!(limit.is_hard_limit);
        assert!(limit.is_within_limit(1024));
        assert!(!limit.is_within_limit(1025));
    }

    #[test]
    fn test_generated_file_patterns() {
        let config = IgnoreConfig::default();
        assert!(!config.generated_file_patterns.is_empty());
        assert!(config
            .generated_file_patterns
            .contains(&"*.pyc".to_string()));
        assert!(config
            .generated_file_patterns
            .contains(&"__pycache__".to_string()));
        assert!(config
            .generated_file_patterns
            .contains(&"target/debug".to_string()));
    }

    #[test]
    fn test_ignore_config_with_custom_rules() {
        let rules = vec![IgnoreRule::gitignore("*.bak"), IgnoreRule::product("*.swp")];

        let config = IgnoreConfig {
            rules,
            size_limit: FileSizeLimit::new(5 * 1024 * 1024),
            binary_policy: BinaryPolicy::Skip,
            generated_file_patterns: vec![],
            max_depth: Some(50),
            follow_symlinks: false,
        };

        assert!(config.rules.len() == 2);
        assert!(config.max_depth == Some(50));
    }

    #[test]
    fn test_ignore_matcher_subdirectory_match() {
        let rules = vec![IgnoreRule::gitignore("node_modules")];

        // Should match node_modules anywhere in path
        assert!(IgnoreMatcher::is_ignored("node_modules", &rules));
        assert!(IgnoreMatcher::is_ignored("node_modules/package", &rules));
        assert!(IgnoreMatcher::is_ignored("nested/node_modules/lib", &rules));
    }

    #[test]
    fn test_ignore_matcher_explicit_exclude() {
        // Without negation, *.log ignores ALL .log files
        let rules = vec![IgnoreRule::gitignore("*.log")];
        assert!(IgnoreMatcher::is_ignored("file.log", &rules));
        assert!(IgnoreMatcher::is_ignored("nested/path.log", &rules));
    }

    #[test]
    fn test_ignore_matcher_negation_prevents_exclusion() {
        // With negation rule, important.log is NOT ignored
        let rules = vec![
            IgnoreRule::gitignore("*.log"),
            IgnoreRule::gitignore("!important.log"),
        ];
        assert!(!IgnoreMatcher::is_ignored("important.log", &rules));
        // But other .log files are still ignored
        assert!(IgnoreMatcher::is_ignored("debug.log", &rules));
    }
}
