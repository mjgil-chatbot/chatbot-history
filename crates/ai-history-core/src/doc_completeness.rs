//! Documentation completeness verification
//!
//! Ensures the design document contains all required sections:
//! - Overview
//! - Goals
//! - Non-Goals
//! - Architecture
//! - State Machine
//! - Event Schema
//! - Storage
//! - Rollback Flows
//! - Rust Crates
//! - Testing
//! - Security
//! - Operations
//! - Open Questions

use std::fs;
use std::path::Path;

/// Required sections in the design document
pub const REQUIRED_SECTIONS: &[&str] = &[
    "## Overview",
    "## Goals",
    "## Non-Goals",
    "## Architecture",
    "## State Machine",
    "## Event Schema",
    "## Storage",
    "## Rollback Flows",
    "## Rust Crates",
    "## Testing",
    "## Security",
    "## Operations",
    "## Open Questions",
];

/// Documentation completeness check result
#[derive(Debug, Clone)]
pub struct DocCompleteness {
    pub section: String,
    pub present: bool,
}

/// Design document verification result
#[derive(Debug, Clone)]
pub struct DesignDocVerification {
    pub all_sections_present: bool,
    pub missing_sections: Vec<String>,
    pub section_count: usize,
    pub total_required: usize,
}

impl DesignDocVerification {
    /// Create a new verification result
    pub fn new() -> Self {
        Self {
            all_sections_present: false,
            missing_sections: Vec::new(),
            section_count: 0,
            total_required: REQUIRED_SECTIONS.len(),
        }
    }

    /// Add a section check result
    pub fn add_section_check(&mut self, result: DocCompleteness) {
        self.section_count += 1;
        if !result.present {
            self.missing_sections.push(result.section.clone());
        }
    }

    /// Finalize verification
    pub fn finalize(&mut self) {
        self.all_sections_present = self.missing_sections.is_empty();
    }

    /// Check if verification passed
    pub fn passed(&self) -> bool {
        self.all_sections_present
    }
}

impl Default for DesignDocVerification {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a design document contains all required sections
pub fn verify_design_doc(design_doc_path: &Path) -> Result<DesignDocVerification, String> {
    let content = fs::read_to_string(design_doc_path)
        .map_err(|e| format!("Failed to read design doc: {}", e))?;

    let mut verification = DesignDocVerification::new();

    for &section in REQUIRED_SECTIONS {
        let present = content.contains(section);
        verification.add_section_check(DocCompleteness {
            section: section.to_string(),
            present,
        });
    }

    verification.finalize();
    Ok(verification)
}

/// Check for required sub-sections within major sections
pub const REQUIRED_SUBSECTIONS: &[(&str, &[&str])] = &[
    ("## State Machine", &["State Transitions", "State Diagram"]),
    ("## Event Schema", &["Event types", "Event Structure"]),
    (
        "## Storage",
        &[
            "Content-Addressed Blob Store",
            "SQLite Metadata Store",
            "Directory Structure",
        ],
    ),
    (
        "## Rollback Flows",
        &["Safe Rollback Protocol", "Rollback Decision Tree"],
    ),
    (
        "## Rust Crates",
        &["ai-history-core", "ai-history-events", "ai-history-store"],
    ),
    ("## Security", &["Threat Model", "Security Controls"]),
    (
        "## Operations",
        &["Health Checks", "Metrics", "Backup and Recovery"],
    ),
];

/// Verify sub-sections are present
pub fn verify_sub_sections(design_doc_path: &Path) -> Result<Vec<(String, Vec<String>)>, String> {
    let content = fs::read_to_string(design_doc_path)
        .map_err(|e| format!("Failed to read design doc: {}", e))?;

    let mut missing = Vec::new();

    for &(section, sub_sections) in REQUIRED_SUBSECTIONS {
        let mut section_missing = Vec::new();
        for &sub_section in sub_sections {
            if !content.contains(sub_section) {
                section_missing.push(sub_section.to_string());
            }
        }
        if !section_missing.is_empty() {
            missing.push((section.to_string(), section_missing));
        }
    }

    Ok(missing)
}

/// Complete design doc check
pub struct DesignDocCheck {
    pub sections_verified: DesignDocVerification,
    pub sub_sections_ok: bool,
    pub architecture_diagram_present: bool,
    pub code_examples_present: bool,
}

impl DesignDocCheck {
    /// Run all design doc checks
    pub fn run(design_doc_path: &Path) -> Result<Self, String> {
        let sections_verified = verify_design_doc(design_doc_path)?;

        let sub_section_missing = verify_sub_sections(design_doc_path)?;
        let sub_sections_ok = sub_section_missing.is_empty();

        let content = fs::read_to_string(design_doc_path)
            .map_err(|e| format!("Failed to read design doc: {}", e))?;

        let architecture_diagram_present =
            content.contains("Architecture") && content.contains("┌──") && content.contains("└──");

        let code_examples_present = content.contains("```rust")
            || content.contains("```sql")
            || content.contains("```bash");

        Ok(Self {
            sections_verified,
            sub_sections_ok,
            architecture_diagram_present,
            code_examples_present,
        })
    }

    /// Check if all checks passed
    pub fn all_passed(&self) -> bool {
        self.sections_verified.passed()
            && self.sub_sections_ok
            && self.architecture_diagram_present
            && self.code_examples_present
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_sections_defined() {
        assert!(REQUIRED_SECTIONS.len() >= 12);
        assert!(REQUIRED_SECTIONS.contains(&"## Overview"));
        assert!(REQUIRED_SECTIONS.contains(&"## Architecture"));
        assert!(REQUIRED_SECTIONS.contains(&"## State Machine"));
        assert!(REQUIRED_SECTIONS.contains(&"## Security"));
    }

    #[test]
    fn test_design_doc_verification_new() {
        let verification = DesignDocVerification::new();
        assert!(!verification.passed());
        assert!(verification.missing_sections.is_empty());
    }

    #[test]
    fn test_design_doc_verification_add_missing() {
        let mut verification = DesignDocVerification::new();
        verification.add_section_check(DocCompleteness {
            section: "## Overview".to_string(),
            present: false,
        });
        verification.finalize();
        assert!(!verification.passed());
        assert_eq!(verification.missing_sections.len(), 1);
    }

    #[test]
    fn test_design_doc_verification_all_present() {
        let mut verification = DesignDocVerification::new();
        for &section in REQUIRED_SECTIONS {
            verification.add_section_check(DocCompleteness {
                section: section.to_string(),
                present: true,
            });
        }
        verification.finalize();
        assert!(verification.passed());
        assert!(verification.missing_sections.is_empty());
    }

    #[test]
    fn test_doc_completeness_debug() {
        let completeness = DocCompleteness {
            section: "## Overview".to_string(),
            present: true,
        };
        let debug_str = format!("{:?}", completeness);
        assert!(debug_str.contains("Overview"));
    }

    #[test]
    fn test_verify_design_doc_rejects_missing_file() {
        let result = verify_design_doc(Path::new("/nonexistent/design.md"));
        assert!(result.is_err());
    }

    #[test]
    fn test_required_sub_sections_defined() {
        assert!(!REQUIRED_SUBSECTIONS.is_empty());

        // Check State Machine has state transitions
        let state_machine_subs: Vec<_> = REQUIRED_SUBSECTIONS
            .iter()
            .filter(|(s, _)| *s == "## State Machine")
            .collect();
        assert!(!state_machine_subs.is_empty());

        // Check Storage has blob store
        let storage_subs: Vec<_> = REQUIRED_SUBSECTIONS
            .iter()
            .filter(|(s, _)| *s == "## Storage")
            .collect();
        assert!(!storage_subs.is_empty());
    }

    #[test]
    fn test_design_doc_check_requires_all() {
        let verification = DesignDocVerification::new();
        let mut v = DesignDocVerification::new();
        for &section in REQUIRED_SECTIONS {
            v.add_section_check(DocCompleteness {
                section: section.to_string(),
                present: true,
            });
        }
        v.finalize();

        let check = DesignDocCheck {
            sections_verified: v,
            sub_sections_ok: true,
            architecture_diagram_present: true,
            code_examples_present: true,
        };

        assert!(check.all_passed());
    }

    #[test]
    fn test_design_doc_check_fails_without_sections() {
        let mut verification = DesignDocVerification::new();
        verification.add_section_check(DocCompleteness {
            section: "## Overview".to_string(),
            present: false,
        });
        verification.finalize();

        let check = DesignDocCheck {
            sections_verified: verification,
            sub_sections_ok: true,
            architecture_diagram_present: true,
            code_examples_present: true,
        };

        assert!(!check.all_passed());
    }

    #[test]
    fn test_design_doc_check_fails_without_architecture() {
        let mut verification = DesignDocVerification::new();
        for &section in REQUIRED_SECTIONS {
            verification.add_section_check(DocCompleteness {
                section: section.to_string(),
                present: true,
            });
        }
        verification.finalize();

        let check = DesignDocCheck {
            sections_verified: verification,
            sub_sections_ok: true,
            architecture_diagram_present: false,
            code_examples_present: true,
        };

        assert!(!check.all_passed());
    }
}
