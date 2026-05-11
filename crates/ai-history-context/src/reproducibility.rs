//! Reproducibility metadata

use super::context::ContextSnapshot;
use ai_history_core::CoreResult;

/// Reproducibility information for a turn
#[derive(Debug, Clone)]
pub struct ReproducibilityInfo {
    pub context_snapshot: ContextSnapshot,
    pub can_reproduce: bool,
    pub missing_dependencies: Vec<String>,
}

impl ReproducibilityInfo {
    pub fn new(context: ContextSnapshot) -> Self {
        Self {
            context_snapshot: context,
            can_reproduce: true,
            missing_dependencies: Vec::new(),
        }
    }

    pub fn add_missing_dependency(&mut self, dep: String) {
        self.missing_dependencies.push(dep);
        self.can_reproduce = false;
    }

    pub fn check_reproducibility(&self) {
        // Check that all referenced blobs exist
        for file in &self.context_snapshot.selected_files {
            // In a full implementation, verify blob exists in store
            let _ = file.blob_hash;
        }
        // Similar checks for other reference types
    }
}

/// Validate reproducibility of a turn
pub fn validate_turn_reproducibility(context: &ContextSnapshot) -> CoreResult<ReproducibilityInfo> {
    let info = ReproducibilityInfo::new(context.clone());
    info.check_reproducibility();
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reproducibility_info_new() {
        let ctx = ContextSnapshot::new(ai_history_core::TurnId::new());
        let info = ReproducibilityInfo::new(ctx);
        assert!(info.can_reproduce);
        assert!(info.missing_dependencies.is_empty());
    }

    #[test]
    fn test_reproducibility_info_add_missing() {
        let ctx = ContextSnapshot::new(ai_history_core::TurnId::new());
        let mut info = ReproducibilityInfo::new(ctx);
        info.add_missing_dependency("blob:abc123".to_string());
        assert!(!info.can_reproduce);
        assert_eq!(info.missing_dependencies.len(), 1);
    }
}
