//! Accessibility and UX verification types
//!
//! Ensures destructive restore confirmations, clear labels,
//! keyboard navigation, conflict explanations, and screen-reader
//! fields are available in UI models.

use serde::{Deserialize, Serialize};

/// Accessibility labels for UI elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityLabels {
    /// Label for confirm restore button
    pub confirm_restore: String,
    /// Label for cancel button
    pub cancel: String,
    /// Label for mode selector
    pub mode_selector: String,
    /// Label for checkpoint list
    pub checkpoint_list: String,
    /// Label for branch list
    pub branch_list: String,
    /// Label for diff viewer
    pub diff_viewer: String,
    /// Label for fork button
    pub fork_button: String,
    /// Label for fork-from-cursor
    pub fork_from_cursor: String,
    /// Label for fork-current
    pub fork_current: String,
    /// Label for ambiguous forward warning
    pub ambiguous_forward: String,
    /// Label for redo button
    pub redo_button: String,
    /// Label for continue-from-cursor button
    pub continue_button: String,
}

impl Default for AccessibilityLabels {
    fn default() -> Self {
        Self {
            confirm_restore: "Confirm restore".to_string(),
            cancel: "Cancel".to_string(),
            mode_selector: "Select restore mode".to_string(),
            checkpoint_list: "Checkpoint list".to_string(),
            branch_list: "Branch list".to_string(),
            diff_viewer: "File changes viewer".to_string(),
            fork_button: "Fork branch".to_string(),
            fork_from_cursor: "Fork from turn".to_string(),
            fork_current: "Fork current conversation".to_string(),
            ambiguous_forward: "Ambiguous forward navigation".to_string(),
            redo_button: "Redo from checkpoint".to_string(),
            continue_button: "Continue from turn".to_string(),
        }
    }
}

/// Keyboard navigation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardNavigation {
    /// Whether keyboard navigation is enabled
    pub enabled: bool,
    /// Current focused element ID
    pub focused_element: Option<String>,
    /// Tab order index for current view
    pub tab_index: u32,
}

impl Default for KeyboardNavigation {
    fn default() -> Self {
        Self {
            enabled: true,
            focused_element: None,
            tab_index: 0,
        }
    }
}

/// Screen reader announcement for dynamic content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenReaderAnnouncement {
    /// Message to announce
    pub message: String,
    /// Priority level (polite vs assertive)
    pub priority: AnnouncementPriority,
}

/// Priority for screen reader announcements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnouncementPriority {
    Polite,
    Assertive,
}

impl Default for AnnouncementPriority {
    fn default() -> Self {
        Self::Polite
    }
}

/// Destructive action confirmation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestructiveConfirmState {
    /// Whether confirmation is required
    pub confirmation_required: bool,
    /// Whether user has confirmed
    pub user_confirmed: bool,
    /// Warning message to display
    pub warning_message: String,
    /// Type of destructive action
    pub action_type: DestructiveActionType,
}

/// Types of destructive actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DestructiveActionType {
    Restore,
    Fork,
    Gc,
    DeleteBranch,
    DeleteCheckpoint,
}

impl DestructiveConfirmState {
    pub fn for_restore(file_count: usize) -> Self {
        Self {
            confirmation_required: true,
            user_confirmed: false,
            warning_message: format!(
                "This will restore {} files. This action cannot be undone.",
                file_count
            ),
            action_type: DestructiveActionType::Restore,
        }
    }

    pub fn for_fork(branch_name: &str) -> Self {
        Self {
            confirmation_required: true,
            user_confirmed: false,
            warning_message: format!(
                "This will fork from the current branch into '{}'. The current branch will be preserved as an abandoned future.",
                branch_name
            ),
            action_type: DestructiveActionType::Fork,
        }
    }

    pub fn confirm(&mut self) {
        self.user_confirmed = true;
    }
}

/// Conflict explanation for screen readers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictExplanation {
    /// Conflict type in human-readable terms
    pub conflict_type: String,
    /// Why this is a problem
    pub explanation: String,
    /// Available resolutions
    pub resolutions: Vec<String>,
}

impl ConflictExplanation {
    pub fn modified(path: &str) -> Self {
        Self {
            conflict_type: "Modified".to_string(),
            explanation: format!(
                "{} has been modified both in the checkpoint and in your current workspace.",
                path
            ),
            resolutions: vec![
                "Keep current version".to_string(),
                "Restore checkpoint version".to_string(),
                "Merge changes".to_string(),
            ],
        }
    }

    pub fn deleted(path: &str) -> Self {
        Self {
            conflict_type: "Deleted".to_string(),
            explanation: format!(
                "{} was modified in checkpoint but deleted in current workspace.",
                path
            ),
            resolutions: vec![
                "Restore checkpoint version".to_string(),
                "Keep deleted version".to_string(),
            ],
        }
    }
}

/// UX copy for common actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UxCopy {
    /// Confirmation button text
    pub confirm_button: String,
    /// Cancel button text
    pub cancel_button: String,
    /// Warning title
    pub warning_title: String,
    /// Warning body
    pub warning_body: String,
    /// Fork button text
    pub fork_button: String,
    /// Fork confirmation title
    pub fork_title: String,
    /// Fork confirmation body when forking from checkpoint
    pub fork_checkpoint_body: String,
    /// Fork confirmation body when forking current conversation
    pub fork_current_body: String,
    /// Ambiguous forward warning title
    pub ambiguous_forward_title: String,
    /// Ambiguous forward warning body
    pub ambiguous_forward_body: String,
}

impl Default for UxCopy {
    fn default() -> Self {
        Self {
            confirm_button: "Confirm".to_string(),
            cancel_button: "Cancel".to_string(),
            warning_title: "Warning".to_string(),
            warning_body: "This action cannot be undone.".to_string(),
            fork_button: "Fork branch".to_string(),
            fork_title: "Fork conversation".to_string(),
            fork_checkpoint_body: "This will create a new branch from the selected checkpoint."
                .to_string(),
            fork_current_body: "This will create a new branch from the current conversation head."
                .to_string(),
            ambiguous_forward_title: "Ambiguous forward navigation".to_string(),
            ambiguous_forward_body:
                "There are multiple possible forward targets. Please select one.".to_string(),
        }
    }
}

/// Accessibility contract for the UI model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityContract {
    /// All accessibility labels
    pub labels: AccessibilityLabels,
    /// Keyboard navigation state
    pub keyboard: KeyboardNavigation,
    /// Default UX copy
    pub ux_copy: UxCopy,
}

impl Default for AccessibilityContract {
    fn default() -> Self {
        Self {
            labels: AccessibilityLabels::default(),
            keyboard: KeyboardNavigation::default(),
            ux_copy: UxCopy::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accessibility_labels_default() {
        let labels = AccessibilityLabels::default();
        assert_eq!(labels.confirm_restore, "Confirm restore");
        assert_eq!(labels.cancel, "Cancel");
    }

    #[test]
    fn test_keyboard_navigation_default() {
        let nav = KeyboardNavigation::default();
        assert!(nav.enabled);
        assert!(nav.focused_element.is_none());
    }

    #[test]
    fn test_destructive_confirm_state_for_restore() {
        let state = DestructiveConfirmState::for_restore(5);
        assert!(state.confirmation_required);
        assert!(!state.user_confirmed);
        assert!(state.warning_message.contains("5 files"));
        assert_eq!(state.action_type, DestructiveActionType::Restore);
    }

    #[test]
    fn test_destructive_confirm_state_confirm() {
        let mut state = DestructiveConfirmState::for_restore(3);
        state.confirm();
        assert!(state.user_confirmed);
    }

    #[test]
    fn test_conflict_explanation_modified() {
        let explanation = ConflictExplanation::modified("src/main.rs");
        assert_eq!(explanation.conflict_type, "Modified");
        assert!(explanation.explanation.contains("src/main.rs"));
        assert_eq!(explanation.resolutions.len(), 3);
    }

    #[test]
    fn test_conflict_explanation_deleted() {
        let explanation = ConflictExplanation::deleted("foo.txt");
        assert_eq!(explanation.conflict_type, "Deleted");
        assert!(explanation.explanation.contains("foo.txt"));
        assert_eq!(explanation.resolutions.len(), 2);
    }

    #[test]
    fn test_ux_copy_default() {
        let ux = UxCopy::default();
        assert_eq!(ux.confirm_button, "Confirm");
        assert_eq!(ux.warning_title, "Warning");
    }

    #[test]
    fn test_accessibility_contract_default() {
        let contract = AccessibilityContract::default();
        assert_eq!(contract.labels.confirm_restore, "Confirm restore");
        assert!(contract.keyboard.enabled);
    }

    #[test]
    fn test_announcement_priority_default() {
        assert_eq!(
            AnnouncementPriority::default(),
            AnnouncementPriority::Polite
        );
    }

    #[test]
    fn test_destructive_action_type_variants() {
        assert_eq!(
            DestructiveActionType::Restore,
            DestructiveActionType::Restore
        );
        assert_eq!(DestructiveActionType::Gc, DestructiveActionType::Gc);
        assert_eq!(
            DestructiveActionType::DeleteBranch,
            DestructiveActionType::DeleteBranch
        );
        assert_eq!(
            DestructiveActionType::DeleteCheckpoint,
            DestructiveActionType::DeleteCheckpoint
        );
    }

    #[test]
    fn test_screen_reader_announcement() {
        let announcement = ScreenReaderAnnouncement {
            message: "Restore completed".to_string(),
            priority: AnnouncementPriority::Assertive,
        };
        assert_eq!(announcement.message, "Restore completed");
        assert_eq!(announcement.priority, AnnouncementPriority::Assertive);
    }
}
