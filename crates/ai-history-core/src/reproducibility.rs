//! Model and tool reproducibility metadata
//!
//! Stores for each assistant turn:
//! - Provider, model, temperature, top-p
//! - Tool versions, system prompt hash, rules hash
//! - Context snapshot ID, runtime feature flags

use crate::ContentHash;
use serde::{Deserialize, Serialize};

/// Model provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelProvider {
    Anthropic,
    OpenAI,
    Google,
    Local,
    Other,
}

impl ModelProvider {
    pub fn display_name(&self) -> &'static str {
        match self {
            ModelProvider::Anthropic => "Anthropic",
            ModelProvider::OpenAI => "OpenAI",
            ModelProvider::Google => "Google",
            ModelProvider::Local => "Local",
            ModelProvider::Other => "Other",
        }
    }
}

/// Reproducibility metadata for an assistant turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproducibilityMetadata {
    pub provider: ModelProvider,
    pub model: String,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub tool_versions: Vec<ToolVersion>,
    pub system_prompt_hash: Option<ContentHash>,
    pub rules_hash: Option<ContentHash>,
    pub context_snapshot_id: Option<String>,
    pub feature_flags: Vec<String>,
}

impl ReproducibilityMetadata {
    pub fn new(provider: ModelProvider, model: String) -> Self {
        Self {
            provider,
            model,
            temperature: None,
            top_p: None,
            tool_versions: Vec::new(),
            system_prompt_hash: None,
            rules_hash: None,
            context_snapshot_id: None,
            feature_flags: Vec::new(),
        }
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn with_tool_version(mut self, name: String, version: String) -> Self {
        self.tool_versions.push(ToolVersion { name, version });
        self
    }

    pub fn with_system_prompt_hash(mut self, hash: ContentHash) -> Self {
        self.system_prompt_hash = Some(hash);
        self
    }

    pub fn with_rules_hash(mut self, hash: ContentHash) -> Self {
        self.rules_hash = Some(hash);
        self
    }

    pub fn with_context_snapshot_id(mut self, id: String) -> Self {
        self.context_snapshot_id = Some(id);
        self
    }

    pub fn with_feature_flag(mut self, flag: String) -> Self {
        self.feature_flags.push(flag);
        self
    }
}

/// Tool version info
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolVersion {
    pub name: String,
    pub version: String,
}

/// Status of a reproducibility dependency
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyStatus {
    Present,
    Missing,
    Stale,
}

/// A single dependency check result
#[derive(Debug, Clone)]
pub struct DependencyCheck {
    pub dependency_type: DependencyType,
    pub reference: String,
    pub status: DependencyStatus,
}

impl DependencyCheck {
    pub fn missing(dep_type: DependencyType, reference: &str) -> Self {
        Self {
            dependency_type: dep_type,
            reference: reference.to_string(),
            status: DependencyStatus::Missing,
        }
    }

    pub fn present(dep_type: DependencyType, reference: &str) -> Self {
        Self {
            dependency_type: dep_type,
            reference: reference.to_string(),
            status: DependencyStatus::Present,
        }
    }

    pub fn stale(dep_type: DependencyType, reference: &str) -> Self {
        Self {
            dependency_type: dep_type,
            reference: reference.to_string(),
            status: DependencyStatus::Stale,
        }
    }
}

/// Type of dependency being checked
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    Blob,
    ToolOutput,
    Summary,
    Memory,
    Rule,
}

impl DependencyType {
    pub fn display_name(&self) -> &'static str {
        match self {
            DependencyType::Blob => "Blob",
            DependencyType::ToolOutput => "Tool Output",
            DependencyType::Summary => "Summary",
            DependencyType::Memory => "Memory",
            DependencyType::Rule => "Rule",
        }
    }
}

/// Result of validating all dependencies for reproducibility
#[derive(Debug, Clone)]
pub struct ReproducibilityValidation {
    pub can_reproduce: bool,
    pub checks: Vec<DependencyCheck>,
    pub missing_count: usize,
    pub stale_count: usize,
}

impl ReproducibilityValidation {
    pub fn new() -> Self {
        Self {
            can_reproduce: true,
            checks: Vec::new(),
            missing_count: 0,
            stale_count: 0,
        }
    }

    pub fn with_check(mut self, check: DependencyCheck) -> Self {
        match check.status {
            DependencyStatus::Missing => {
                self.missing_count += 1;
                self.can_reproduce = false;
            }
            DependencyStatus::Stale => {
                self.stale_count += 1;
                self.can_reproduce = false;
            }
            DependencyStatus::Present => {}
        }
        self.checks.push(check);
        self
    }

    pub fn all_present(&self) -> bool {
        self.missing_count == 0 && self.stale_count == 0
    }

    pub fn missing_references(&self) -> Vec<String> {
        self.checks
            .iter()
            .filter(|c| c.status == DependencyStatus::Missing)
            .map(|c| c.reference.clone())
            .collect()
    }
}

impl Default for ReproducibilityValidation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_provider_display() {
        assert_eq!(ModelProvider::Anthropic.display_name(), "Anthropic");
        assert_eq!(ModelProvider::OpenAI.display_name(), "OpenAI");
        assert_eq!(ModelProvider::Local.display_name(), "Local");
    }

    #[test]
    fn test_reproducibility_metadata_new() {
        let meta = ReproducibilityMetadata::new(ModelProvider::Anthropic, "claude-3".to_string());
        assert_eq!(meta.provider, ModelProvider::Anthropic);
        assert_eq!(meta.model, "claude-3");
        assert!(meta.temperature.is_none());
    }

    #[test]
    fn test_reproducibility_metadata_chain() {
        let meta = ReproducibilityMetadata::new(ModelProvider::OpenAI, "gpt-4".to_string())
            .with_temperature(0.7)
            .with_top_p(0.9)
            .with_tool_version("bash".to_string(), "1.0".to_string())
            .with_feature_flag("streaming".to_string());

        assert_eq!(meta.temperature, Some(0.7));
        assert_eq!(meta.top_p, Some(0.9));
        assert_eq!(meta.tool_versions.len(), 1);
        assert_eq!(meta.feature_flags, vec!["streaming"]);
    }

    #[test]
    fn test_reproducibility_metadata_hashes() {
        let hash = ContentHash::new(b"system prompt");
        let meta = ReproducibilityMetadata::new(ModelProvider::Google, "gemini".to_string())
            .with_system_prompt_hash(hash)
            .with_rules_hash(hash);

        assert!(meta.system_prompt_hash.is_some());
        assert!(meta.rules_hash.is_some());
    }

    #[test]
    fn test_tool_version() {
        let tv = ToolVersion {
            name: "read_file".to_string(),
            version: "2.1.0".to_string(),
        };
        assert_eq!(tv.name, "read_file");
        assert_eq!(tv.version, "2.1.0");
    }

    #[test]
    fn test_dependency_status() {
        assert!(matches!(
            DependencyStatus::Present,
            DependencyStatus::Present
        ));
        assert!(matches!(
            DependencyStatus::Missing,
            DependencyStatus::Missing
        ));
        assert!(matches!(DependencyStatus::Stale, DependencyStatus::Stale));
    }

    #[test]
    fn test_dependency_check_missing() {
        let check = DependencyCheck::missing(DependencyType::Blob, "blob:abc123");
        assert!(matches!(check.status, DependencyStatus::Missing));
        assert_eq!(check.reference, "blob:abc123");
        assert_eq!(check.dependency_type, DependencyType::Blob);
    }

    #[test]
    fn test_dependency_check_present() {
        let check = DependencyCheck::present(DependencyType::ToolOutput, "tool:output:456");
        assert!(matches!(check.status, DependencyStatus::Present));
        assert_eq!(check.reference, "tool:output:456");
    }

    #[test]
    fn test_dependency_type_display() {
        assert_eq!(DependencyType::Blob.display_name(), "Blob");
        assert_eq!(DependencyType::ToolOutput.display_name(), "Tool Output");
        assert_eq!(DependencyType::Summary.display_name(), "Summary");
        assert_eq!(DependencyType::Memory.display_name(), "Memory");
        assert_eq!(DependencyType::Rule.display_name(), "Rule");
    }

    #[test]
    fn test_reproducibility_validation_new() {
        let validation = ReproducibilityValidation::new();
        assert!(validation.can_reproduce);
        assert!(validation.checks.is_empty());
        assert_eq!(validation.missing_count, 0);
    }

    #[test]
    fn test_reproducibility_validation_with_check_present() {
        let validation = ReproducibilityValidation::new()
            .with_check(DependencyCheck::present(DependencyType::Blob, "blob:123"));
        assert!(validation.can_reproduce);
        assert_eq!(validation.checks.len(), 1);
    }

    #[test]
    fn test_reproducibility_validation_with_check_missing() {
        let validation = ReproducibilityValidation::new().with_check(DependencyCheck::missing(
            DependencyType::Blob,
            "blob:missing",
        ));
        assert!(!validation.can_reproduce);
        assert_eq!(validation.missing_count, 1);
    }

    #[test]
    fn test_reproducibility_validation_all_present() {
        let validation = ReproducibilityValidation::new()
            .with_check(DependencyCheck::present(DependencyType::Blob, "blob:1"))
            .with_check(DependencyCheck::present(
                DependencyType::ToolOutput,
                "tool:1",
            ));
        assert!(validation.all_present());
    }

    #[test]
    fn test_reproducibility_validation_missing_references() {
        let validation = ReproducibilityValidation::new()
            .with_check(DependencyCheck::missing(DependencyType::Blob, "blob:1"))
            .with_check(DependencyCheck::present(
                DependencyType::ToolOutput,
                "tool:1",
            ))
            .with_check(DependencyCheck::missing(
                DependencyType::Summary,
                "summary:1",
            ));
        let missing = validation.missing_references();
        assert_eq!(missing.len(), 2);
        assert!(missing.contains(&"blob:1".to_string()));
        assert!(missing.contains(&"summary:1".to_string()));
    }
}
