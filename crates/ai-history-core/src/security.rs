//! Security and privacy controls for ai-history
//!
//! Defines:
//! - Encryption-at-rest option
//! - Secret redaction policy
//! - Ignored file policy
//! - Private file exclusion
//! - Safe logging
//! - Workspace access boundaries

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Encryption status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionStatus {
    Disabled,
    Enabled,
    KeyMissing,
}

/// Secret redaction policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretRedactionPolicy {
    /// Never redact secrets
    Disabled,
    /// Redact common secret patterns
    Standard,
    /// Aggressive redaction of all potential secrets
    Aggressive,
}

impl Default for SecretRedactionPolicy {
    fn default() -> Self {
        Self::Standard
    }
}

/// Secret pattern for redaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPattern {
    pub name: String,
    pub pattern: String,
    pub replacement: String,
}

impl SecretPattern {
    pub fn new(name: &str, pattern: &str, replacement: &str) -> Self {
        Self {
            name: name.to_string(),
            pattern: pattern.to_string(),
            replacement: replacement.to_string(),
        }
    }
}

/// Privacy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub encryption_status: EncryptionStatus,
    pub secret_redaction: SecretRedactionPolicy,
    pub private_file_patterns: Vec<String>,
    pub ignored_file_patterns: Vec<String>,
    pub max_file_size_bytes: u64,
    pub binary_file_policy: BinaryFilePolicy,
    pub safe_logging_enabled: bool,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            encryption_status: EncryptionStatus::Disabled,
            secret_redaction: SecretRedactionPolicy::default(),
            private_file_patterns: vec![
                "*.env".to_string(),
                "*.pem".to_string(),
                "*.key".to_string(),
                ".env*".to_string(),
                ".npmrc".to_string(),
            ],
            ignored_file_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "__pycache__".to_string(),
                "target".to_string(),
                ".DS_Store".to_string(),
            ],
            max_file_size_bytes: 10 * 1024 * 1024, // 10MB
            binary_file_policy: BinaryFilePolicy::Skip,
            safe_logging_enabled: true,
        }
    }
}

/// Binary file handling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryFilePolicy {
    /// Skip binary files
    Skip,
    /// Store binary files as-is
    Store,
    /// Hash binary files without storing content
    HashOnly,
}

impl Default for BinaryFilePolicy {
    fn default() -> Self {
        Self::Skip
    }
}

/// Workspace access boundary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessBoundary {
    pub allowed_paths: HashSet<String>,
    pub denied_paths: HashSet<String>,
    pub max_depth: Option<usize>,
}

impl AccessBoundary {
    pub fn new() -> Self {
        Self {
            allowed_paths: HashSet::new(),
            denied_paths: HashSet::new(),
            max_depth: None,
        }
    }

    pub fn allow_path(mut self, path: String) -> Self {
        self.allowed_paths.insert(path);
        self
    }

    pub fn deny_path(mut self, path: String) -> Self {
        self.denied_paths.insert(path);
        self
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    pub fn is_accessible(&self, path: &str) -> bool {
        // Check denied paths first
        for denied in &self.denied_paths {
            if path.starts_with(denied) {
                return false;
            }
        }

        // If allowed paths are specified, check them
        if !self.allowed_paths.is_empty() {
            for allowed in &self.allowed_paths {
                if path.starts_with(allowed) {
                    // Check depth limit
                    if let Some(max) = self.max_depth {
                        let depth = path.matches('/').count();
                        if depth > max {
                            return false;
                        }
                    }
                    return true;
                }
            }
            return false;
        }

        // Check depth limit if no allowed paths specified
        if let Some(max) = self.max_depth {
            let depth = path.matches('/').count();
            if depth > max {
                return false;
            }
        }

        true
    }
}

impl Default for AccessBoundary {
    fn default() -> Self {
        Self::new()
    }
}

/// Workspace security state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSecurity {
    pub is_encrypted: bool,
    pub has_private_files: bool,
    pub secrets_detected: bool,
    pub access_boundary: AccessBoundary,
}

impl WorkspaceSecurity {
    pub fn new() -> Self {
        Self {
            is_encrypted: false,
            has_private_files: false,
            secrets_detected: false,
            access_boundary: AccessBoundary::new(),
        }
    }

    pub fn with_encryption(mut self, encrypted: bool) -> Self {
        self.is_encrypted = encrypted;
        self
    }

    pub fn with_private_files(mut self, has_private: bool) -> Self {
        self.has_private_files = has_private;
        self
    }

    pub fn with_secrets_detected(mut self, detected: bool) -> Self {
        self.secrets_detected = detected;
        self
    }

    pub fn is_safe_for_restore(&self) -> bool {
        !self.has_private_files && !self.secrets_detected
    }
}

impl Default for WorkspaceSecurity {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_config_default() {
        let config = PrivacyConfig::default();
        assert_eq!(config.secret_redaction, SecretRedactionPolicy::Standard);
        assert!(config.safe_logging_enabled);
        assert!(!config.private_file_patterns.is_empty());
    }

    #[test]
    fn test_binary_file_policy_default() {
        assert_eq!(BinaryFilePolicy::default(), BinaryFilePolicy::Skip);
    }

    #[test]
    fn test_access_boundary_allow() {
        let boundary = AccessBoundary::new()
            .allow_path("/workspace".to_string())
            .deny_path("/workspace/.env".to_string());

        assert!(boundary.is_accessible("/workspace/src/main.rs"));
        assert!(!boundary.is_accessible("/workspace/.env"));
    }

    #[test]
    fn test_access_boundary_max_depth() {
        let boundary = AccessBoundary::new().with_max_depth(3);

        assert!(boundary.is_accessible("/a/b/c"));
        assert!(!boundary.is_accessible("/a/b/c/d"));
    }

    #[test]
    fn test_workspace_security_new() {
        let security = WorkspaceSecurity::new();
        assert!(!security.is_encrypted);
        assert!(!security.has_private_files);
    }

    #[test]
    fn test_workspace_security_is_safe_for_restore() {
        let safe = WorkspaceSecurity::new()
            .with_private_files(false)
            .with_secrets_detected(false);
        assert!(safe.is_safe_for_restore());

        let unsafe_with_private = WorkspaceSecurity::new().with_private_files(true);
        assert!(!unsafe_with_private.is_safe_for_restore());

        let unsafe_with_secrets = WorkspaceSecurity::new().with_secrets_detected(true);
        assert!(!unsafe_with_secrets.is_safe_for_restore());
    }

    #[test]
    fn test_secret_pattern_new() {
        let pattern = SecretPattern::new("API Key", r"sk-[a-zA-Z0-9]+", "[REDACTED]");
        assert_eq!(pattern.name, "API Key");
        assert_eq!(pattern.replacement, "[REDACTED]");
    }

    #[test]
    fn test_encryption_status() {
        assert_eq!(EncryptionStatus::Disabled, EncryptionStatus::Disabled);
        assert_eq!(EncryptionStatus::Enabled, EncryptionStatus::Enabled);
    }
}
