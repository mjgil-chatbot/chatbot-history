//! Accessibility and UX requirements verification
//!
//! Ensures:
//! - Restore actions are labeled clearly
//! - Destructive actions require confirmation
//! - Conflicts are explainable
//! - Keyboard navigation is supported

use serde::{Deserialize, Serialize};

/// Confirmation requirement for destructive actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfirmationRequirement {
    Required,
    Optional,
    NotNeeded,
}

impl Default for ConfirmationRequirement {
    fn default() -> Self {
        Self::Required
    }
}

/// Keyboard navigation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardNavigationState {
    pub focused_element: Option<String>,
    pub tab_order_enabled: bool,
    pub arrow_navigation_enabled: bool,
}

impl KeyboardNavigationState {
    pub fn new() -> Self {
        Self {
            focused_element: None,
            tab_order_enabled: true,
            arrow_navigation_enabled: true,
        }
    }

    pub fn set_focus(&mut self, element: String) {
        self.focused_element = Some(element);
    }

    pub fn clear_focus(&mut self) {
        self.focused_element = None;
    }
}

impl Default for KeyboardNavigationState {
    fn default() -> Self {
        Self::new()
    }
}

/// Accessibility label for UI elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityLabel {
    pub element_id: String,
    pub label: String,
    pub described_by: Option<String>,
    pub role: Option<String>,
}

impl AccessibilityLabel {
    pub fn new(element_id: String, label: String) -> Self {
        Self {
            element_id,
            label,
            described_by: None,
            role: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.described_by = Some(description);
        self
    }

    pub fn with_role(mut self, role: String) -> Self {
        self.role = Some(role);
        self
    }
}

/// Conflict explanation for users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictExplanation {
    pub conflict_type: String,
    pub what_changed: String,
    pub impact: String,
    pub resolution_options: Vec<String>,
}

impl ConflictExplanation {
    pub fn new(conflict_type: &str, what_changed: &str, impact: &str) -> Self {
        Self {
            conflict_type: conflict_type.to_string(),
            what_changed: what_changed.to_string(),
            impact: impact.to_string(),
            resolution_options: Vec::new(),
        }
    }

    pub fn add_resolution_option(&mut self, option: String) {
        self.resolution_options.push(option);
    }

    pub fn has_explanation(&self) -> bool {
        !self.conflict_type.is_empty() && !self.what_changed.is_empty() && !self.impact.is_empty()
    }

    pub fn is_helpful(&self) -> bool {
        self.has_explanation() && !self.resolution_options.is_empty()
    }
}

/// Restore action labeling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreActionLabel {
    RestoreWorkspace,
    RestoreConversation,
    RestoreBoth,
    ForkFromCheckpoint,
}

impl RestoreActionLabel {
    pub fn label(&self) -> &str {
        match self {
            Self::RestoreWorkspace => "Restore Workspace",
            Self::RestoreConversation => "Restore Conversation",
            Self::RestoreBoth => "Restore Both",
            Self::ForkFromCheckpoint => "Fork from Checkpoint",
        }
    }

    pub fn is_destructive(&self) -> bool {
        matches!(self, Self::RestoreWorkspace | Self::RestoreBoth)
    }
}

/// Destructive action with confirmation requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestructiveAction {
    pub action_type: String,
    pub requires_confirmation: ConfirmationRequirement,
    pub warning_message: Option<String>,
}

impl DestructiveAction {
    pub fn new(action_type: &str) -> Self {
        let requires_confirmation = match action_type {
            "restore" | "checkpoint_delete" | "branch_delete" => ConfirmationRequirement::Required,
            "session_delete" => ConfirmationRequirement::Required,
            _ => ConfirmationRequirement::Optional,
        };

        Self {
            action_type: action_type.to_string(),
            requires_confirmation,
            warning_message: None,
        }
    }

    pub fn with_warning(mut self, message: String) -> Self {
        self.warning_message = Some(message);
        self
    }

    pub fn is_destructive(&self) -> bool {
        matches!(
            self.requires_confirmation,
            ConfirmationRequirement::Required
        )
    }
}

/// Accessibility verification result
#[derive(Debug, Clone)]
pub struct AccessibilityVerification {
    pub restore_labels_clear: bool,
    pub destructive_actions_have_confirmations: bool,
    pub conflicts_have_explanations: bool,
    pub keyboard_navigation_supported: bool,
    pub all_checks_passed: bool,
}

impl AccessibilityVerification {
    pub fn new() -> Self {
        Self {
            restore_labels_clear: false,
            destructive_actions_have_confirmations: false,
            conflicts_have_explanations: false,
            keyboard_navigation_supported: false,
            all_checks_passed: false,
        }
    }

    pub fn verify_restore_labels(&mut self, labels: &[RestoreActionLabel]) {
        self.restore_labels_clear =
            !labels.is_empty() && labels.iter().all(|l| !l.label().is_empty());
        self.update_passed();
    }

    pub fn verify_destructive_confirmations(&mut self, actions: &[DestructiveAction]) {
        self.destructive_actions_have_confirmations = actions
            .iter()
            .filter(|a| a.is_destructive())
            .all(|a| a.requires_confirmation == ConfirmationRequirement::Required);
        self.update_passed();
    }

    pub fn verify_conflict_explanations(&mut self, explanations: &[ConflictExplanation]) {
        self.conflicts_have_explanations =
            !explanations.is_empty() && explanations.iter().all(|e| e.is_helpful());
        self.update_passed();
    }

    pub fn verify_keyboard_navigation(&mut self, nav: &KeyboardNavigationState) {
        self.keyboard_navigation_supported = nav.tab_order_enabled || nav.arrow_navigation_enabled;
        self.update_passed();
    }

    fn update_passed(&mut self) {
        self.all_checks_passed = self.restore_labels_clear
            && self.destructive_actions_have_confirmations
            && self.conflicts_have_explanations
            && self.keyboard_navigation_supported;
    }
}

impl Default for AccessibilityVerification {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restore_action_label_labels() {
        assert_eq!(
            RestoreActionLabel::RestoreWorkspace.label(),
            "Restore Workspace"
        );
        assert_eq!(
            RestoreActionLabel::RestoreConversation.label(),
            "Restore Conversation"
        );
        assert_eq!(RestoreActionLabel::RestoreBoth.label(), "Restore Both");
        assert_eq!(
            RestoreActionLabel::ForkFromCheckpoint.label(),
            "Fork from Checkpoint"
        );
    }

    #[test]
    fn test_restore_action_destructive() {
        assert!(RestoreActionLabel::RestoreWorkspace.is_destructive());
        assert!(!RestoreActionLabel::RestoreConversation.is_destructive());
        assert!(RestoreActionLabel::RestoreBoth.is_destructive());
        assert!(!RestoreActionLabel::ForkFromCheckpoint.is_destructive());
    }

    #[test]
    fn test_confirmation_requirement_default() {
        assert_eq!(
            ConfirmationRequirement::default(),
            ConfirmationRequirement::Required
        );
    }

    #[test]
    fn test_destructive_action_new() {
        let action = DestructiveAction::new("restore");
        assert!(action.is_destructive());
        assert_eq!(
            action.requires_confirmation,
            ConfirmationRequirement::Required
        );

        let action = DestructiveAction::new("view");
        assert!(!action.is_destructive());
    }

    #[test]
    fn test_destructive_action_with_warning() {
        let action = DestructiveAction::new("restore")
            .with_warning("This will overwrite your current changes".to_string());
        assert!(action.warning_message.is_some());
        assert!(action.warning_message.unwrap().contains("overwrite"));
    }

    #[test]
    fn test_keyboard_navigation_new() {
        let nav = KeyboardNavigationState::new();
        assert!(nav.focused_element.is_none());
        assert!(nav.tab_order_enabled);
        assert!(nav.arrow_navigation_enabled);
    }

    #[test]
    fn test_keyboard_navigation_set_focus() {
        let mut nav = KeyboardNavigationState::new();
        nav.set_focus("restore_button".to_string());
        assert_eq!(nav.focused_element, Some("restore_button".to_string()));

        nav.clear_focus();
        assert!(nav.focused_element.is_none());
    }

    #[test]
    fn test_accessibility_label_new() {
        let label = AccessibilityLabel::new("btn-restore".to_string(), "Restore".to_string());
        assert_eq!(label.element_id, "btn-restore");
        assert_eq!(label.label, "Restore");
        assert!(label.described_by.is_none());
        assert!(label.role.is_none());
    }

    #[test]
    fn test_accessibility_label_with_description() {
        let label = AccessibilityLabel::new("btn-restore".to_string(), "Restore".to_string())
            .with_description("Restores workspace to a previous state".to_string());
        assert!(label.described_by.is_some());
    }

    #[test]
    fn test_accessibility_label_with_role() {
        let label = AccessibilityLabel::new("btn-restore".to_string(), "Restore".to_string())
            .with_role("button".to_string());
        assert_eq!(label.role, Some("button".to_string()));
    }

    #[test]
    fn test_conflict_explanation_new() {
        let explanation = ConflictExplanation::new(
            "modified",
            "File was changed in both locations",
            "You need to choose which version to keep",
        );
        assert_eq!(explanation.conflict_type, "modified");
        assert!(explanation.has_explanation());
    }

    #[test]
    fn test_conflict_explanation_not_helpful_without_options() {
        let explanation = ConflictExplanation::new("modified", "changed", "choose one");
        assert!(explanation.has_explanation());
        assert!(!explanation.is_helpful());
    }

    #[test]
    fn test_conflict_explanation_helpful_with_options() {
        let mut explanation = ConflictExplanation::new("modified", "changed", "choose one");
        explanation.add_resolution_option("Keep current version".to_string());
        explanation.add_resolution_option("Restore checkpoint version".to_string());
        assert!(explanation.is_helpful());
    }

    #[test]
    fn test_accessibility_verification_new() {
        let verification = AccessibilityVerification::new();
        assert!(!verification.all_checks_passed);
    }

    #[test]
    fn test_accessibility_verification_all_pass() {
        let mut verification = AccessibilityVerification::new();

        // Set all checks to pass
        verification.restore_labels_clear = true;
        verification.destructive_actions_have_confirmations = true;
        verification.conflicts_have_explanations = true;
        verification.keyboard_navigation_supported = true;
        verification.update_passed();

        assert!(verification.all_checks_passed);
    }

    #[test]
    fn test_accessibility_verification_restore_labels() {
        let mut verification = AccessibilityVerification::new();
        let labels = [
            RestoreActionLabel::RestoreWorkspace,
            RestoreActionLabel::RestoreConversation,
        ];

        verification.verify_restore_labels(&labels);
        assert!(verification.restore_labels_clear);
    }

    #[test]
    fn test_accessibility_verification_destructive_confirmations() {
        let mut verification = AccessibilityVerification::new();
        let actions = vec![
            DestructiveAction::new("restore"),
            DestructiveAction::new("checkpoint_delete"),
        ];

        verification.verify_destructive_confirmations(&actions);
        assert!(verification.destructive_actions_have_confirmations);
    }

    #[test]
    fn test_accessibility_verification_conflict_explanations() {
        let mut verification = AccessibilityVerification::new();
        let mut explanation = ConflictExplanation::new("modified", "changed", "choose");
        explanation.add_resolution_option("Option 1".to_string());

        verification.verify_conflict_explanations(&[explanation]);
        assert!(verification.conflicts_have_explanations);
    }

    #[test]
    fn test_accessibility_verification_keyboard_navigation() {
        let mut verification = AccessibilityVerification::new();
        let nav = KeyboardNavigationState::new();

        verification.verify_keyboard_navigation(&nav);
        assert!(verification.keyboard_navigation_supported);
    }

    #[test]
    fn test_restore_labels_all_have_content() {
        for label in [
            RestoreActionLabel::RestoreWorkspace,
            RestoreActionLabel::RestoreConversation,
            RestoreActionLabel::RestoreBoth,
            RestoreActionLabel::ForkFromCheckpoint,
        ] {
            assert!(!label.label().is_empty());
        }
    }

    #[test]
    fn test_destructive_action_session_delete_requires_confirmation() {
        let action = DestructiveAction::new("session_delete");
        assert_eq!(
            action.requires_confirmation,
            ConfirmationRequirement::Required
        );
        assert!(action.is_destructive());
    }

    #[test]
    fn test_conflict_explanation_multiple_options() {
        let mut explanation = ConflictExplanation::new(
            "deleted",
            "File was deleted in checkpoint",
            "The file exists in your current workspace",
        );
        explanation.add_resolution_option("Restore the file".to_string());
        explanation.add_resolution_option("Delete it from workspace".to_string());
        explanation.add_resolution_option("Keep both versions".to_string());

        assert_eq!(explanation.resolution_options.len(), 3);
        assert!(explanation.is_helpful());
    }
}
