//! Ambiguous forward navigation UI model types
//!
//! Provides UI-ready DTOs for forward navigation disambiguation,
//! carrying target branch names, abandoned-future flags, and counts
//! for screen readers and disambiguation dialogs.

use ai_history_core::{BranchId, TurnId};
use serde::{Deserialize, Serialize};

/// A single forward navigation target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardNavigationTarget {
    /// The target turn ID
    pub turn_id: TurnId,
    /// The target branch ID
    pub branch_id: BranchId,
    /// Human-readable branch name
    pub branch_name: String,
    /// Whether this target is an abandoned future
    pub is_abandoned_future: bool,
    /// Whether this target is on the current branch
    pub is_current_branch: bool,
}

/// Result of forward navigation with multiple possible targets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbiguousForwardUi {
    /// The cursor turn where navigation originated
    pub cursor_turn_id: TurnId,
    /// All possible forward targets
    pub targets: Vec<ForwardNavigationTarget>,
    /// Count of targets on the current branch (non-abandoned)
    pub current_branch_count: usize,
    /// Count of abandoned future targets
    pub abandoned_future_count: usize,
    /// Warning message for screen readers
    pub screen_reader_warning: String,
}

impl AmbiguousForwardUi {
    pub fn new(cursor_turn_id: TurnId) -> Self {
        Self {
            cursor_turn_id,
            targets: Vec::new(),
            current_branch_count: 0,
            abandoned_future_count: 0,
            screen_reader_warning: String::new(),
        }
    }

    pub fn add_target(&mut self, target: ForwardNavigationTarget) {
        if target.is_abandoned_future {
            self.abandoned_future_count += 1;
        } else {
            self.current_branch_count += 1;
        }
        self.targets.push(target);
    }

    pub fn has_multiple(&self) -> bool {
        self.targets.len() > 1
    }

    pub fn has_none(&self) -> bool {
        self.targets.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    pub fn len(&self) -> usize {
        self.targets.len()
    }

    /// Get the single target if exactly one exists
    pub fn get_single(&self) -> Option<&ForwardNavigationTarget> {
        if self.targets.len() == 1 {
            Some(&self.targets[0])
        } else {
            None
        }
    }

    /// Get all targets on the current branch (not abandoned)
    pub fn current_branch_targets(&self) -> Vec<&ForwardNavigationTarget> {
        self.targets
            .iter()
            .filter(|t| !t.is_abandoned_future)
            .collect()
    }

    /// Get all abandoned future targets
    pub fn abandoned_future_targets(&self) -> Vec<&ForwardNavigationTarget> {
        self.targets
            .iter()
            .filter(|t| t.is_abandoned_future)
            .collect()
    }

    /// Build the screen reader warning from current state
    pub fn build_screen_reader_warning(&mut self) {
        if self.targets.len() > 1 {
            self.screen_reader_warning = format!(
                "Ambiguous forward navigation: {} targets available. {} on current branch, {} abandoned futures.",
                self.targets.len(),
                self.current_branch_count,
                self.abandoned_future_count
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_history_core::TurnId;

    #[test]
    fn test_ambiguous_forward_ui_empty() {
        let turn_id = TurnId::new();
        let ui = AmbiguousForwardUi::new(turn_id);
        assert!(ui.is_empty());
        assert!(!ui.has_multiple());
        assert!(ui.has_none());
    }

    #[test]
    fn test_ambiguous_forward_ui_single_target() {
        let turn_id = TurnId::new();
        let mut ui = AmbiguousForwardUi::new(turn_id);
        ui.add_target(ForwardNavigationTarget {
            turn_id: TurnId::new(),
            branch_id: BranchId::new(),
            branch_name: "feature".to_string(),
            is_abandoned_future: false,
            is_current_branch: true,
        });
        assert!(!ui.is_empty());
        assert!(!ui.has_multiple());
        assert!(!ui.has_none());
        assert_eq!(ui.len(), 1);
        assert!(ui.get_single().is_some());
        assert_eq!(ui.current_branch_count, 1);
        assert_eq!(ui.abandoned_future_count, 0);
    }

    #[test]
    fn test_ambiguous_forward_ui_multiple_targets() {
        let turn_id = TurnId::new();
        let mut ui = AmbiguousForwardUi::new(turn_id);
        ui.add_target(ForwardNavigationTarget {
            turn_id: TurnId::new(),
            branch_id: BranchId::new(),
            branch_name: "feature-1".to_string(),
            is_abandoned_future: false,
            is_current_branch: true,
        });
        ui.add_target(ForwardNavigationTarget {
            turn_id: TurnId::new(),
            branch_id: BranchId::new(),
            branch_name: "feature-2".to_string(),
            is_abandoned_future: true,
            is_current_branch: false,
        });
        assert!(!ui.is_empty());
        assert!(ui.has_multiple());
        assert!(!ui.has_none());
        assert_eq!(ui.len(), 2);
        assert!(ui.get_single().is_none());
        assert_eq!(ui.current_branch_count, 1);
        assert_eq!(ui.abandoned_future_count, 1);
    }

    #[test]
    fn test_ambiguous_forward_ui_screen_reader_warning() {
        let turn_id = TurnId::new();
        let mut ui = AmbiguousForwardUi::new(turn_id);
        ui.add_target(ForwardNavigationTarget {
            turn_id: TurnId::new(),
            branch_id: BranchId::new(),
            branch_name: "feature-1".to_string(),
            is_abandoned_future: false,
            is_current_branch: true,
        });
        ui.add_target(ForwardNavigationTarget {
            turn_id: TurnId::new(),
            branch_id: BranchId::new(),
            branch_name: "feature-2".to_string(),
            is_abandoned_future: true,
            is_current_branch: false,
        });
        ui.build_screen_reader_warning();
        assert!(ui
            .screen_reader_warning
            .contains("Ambiguous forward navigation"));
        assert!(ui.screen_reader_warning.contains("2 targets"));
    }
}
