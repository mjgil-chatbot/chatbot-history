//! Timeline cursor navigation for ai-history
//!
//! Task 11: Add timeline cursor navigation
//! - view_cursor_turn_id is separate from branch.head_turn_id
//! - Backward/forward navigation does NOT mutate history or workspace
//! - Cursor position is transient - not persisted as part of branch state

use super::{BranchId, SessionId, TurnId};
use std::collections::HashMap;

/// Cursor position in timeline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorPosition {
    /// At the branch head (default)
    AtHead,
    /// At a specific turn
    AtTurn(TurnId),
}

/// A cursor for navigating the conversation timeline
#[derive(Debug, Clone)]
pub struct TimelineCursor {
    session_id: SessionId,
    branch_id: BranchId,
    position: CursorPosition,
    /// Turn ID -> list of child turn IDs (for forward navigation)
    children_map: HashMap<TurnId, Vec<TurnId>>,
}

impl TimelineCursor {
    pub fn new(session_id: SessionId, branch_id: BranchId) -> Self {
        Self {
            session_id,
            branch_id,
            position: CursorPosition::AtHead,
            children_map: HashMap::new(),
        }
    }

    pub fn at_turn(session_id: SessionId, branch_id: BranchId, turn_id: TurnId) -> Self {
        Self {
            session_id,
            branch_id,
            position: CursorPosition::AtTurn(turn_id),
            children_map: HashMap::new(),
        }
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn branch_id(&self) -> BranchId {
        self.branch_id
    }

    pub fn position(&self) -> CursorPosition {
        self.position
    }

    /// Set cursor to branch head
    pub fn go_to_head(&mut self) {
        self.position = CursorPosition::AtHead;
    }

    /// Set cursor to a specific turn
    pub fn go_to_turn(&mut self, turn_id: TurnId) {
        self.position = CursorPosition::AtTurn(turn_id);
    }

    /// Navigate backward in history (does NOT mutate workspace)
    /// Returns the previous turn ID if available
    pub fn navigate_backward(&self, parent_turn_id: Option<TurnId>) -> Option<TurnId> {
        match parent_turn_id {
            Some(pid) => Some(pid),
            None => None,
        }
    }

    /// Navigate forward in history (does NOT mutate workspace)
    /// Returns forward targets if cursor is at a turn that has children
    pub fn navigate_forward(&self) -> Vec<TurnId> {
        match &self.position {
            CursorPosition::AtHead => Vec::new(),
            CursorPosition::AtTurn(turn_id) => {
                self.children_map.get(turn_id).cloned().unwrap_or_default()
            }
        }
    }

    /// Navigate forward from a specific turn (used when cursor is at head)
    pub fn navigate_forward_from(&self, turn_id: TurnId) -> Vec<TurnId> {
        self.children_map.get(&turn_id).cloned().unwrap_or_default()
    }

    /// Check if cursor is at branch head
    pub fn is_at_head(&self) -> bool {
        matches!(self.position, CursorPosition::AtHead)
    }

    /// Check if cursor is at a specific turn
    pub fn is_at_turn(&self, turn_id: TurnId) -> bool {
        matches!(self.position, CursorPosition::AtTurn(id) if id == turn_id)
    }

    /// Get current turn ID (None if at head)
    pub fn current_turn_id(&self) -> Option<TurnId> {
        match self.position {
            CursorPosition::AtHead => None,
            CursorPosition::AtTurn(id) => Some(id),
        }
    }

    /// Build child map for forward navigation
    pub fn register_children(&mut self, parent_turn_id: TurnId, child_turn_ids: Vec<TurnId>) {
        self.children_map.insert(parent_turn_id, child_turn_ids);
    }
}

/// Forward navigation target (when multiple branches exist)
#[derive(Debug, Clone)]
pub struct ForwardTarget {
    pub turn_id: TurnId,
    pub branch_id: BranchId,
    pub branch_name: Option<String>,
    pub is_abandoned_future: bool,
}

/// Result of forward navigation with multiple possible targets
#[derive(Debug, Clone)]
pub struct AmbiguousForwardTargets {
    pub targets: Vec<ForwardTarget>,
    pub cursor_turn_id: TurnId,
}

impl AmbiguousForwardTargets {
    pub fn new(cursor_turn_id: TurnId) -> Self {
        Self {
            targets: Vec::new(),
            cursor_turn_id,
        }
    }

    pub fn add_target(&mut self, target: ForwardTarget) {
        self.targets.push(target);
    }

    pub fn has_multiple(&self) -> bool {
        self.targets.len() > 1
    }

    pub fn has_none(&self) -> bool {
        self.targets.is_empty()
    }

    pub fn len(&self) -> usize {
        self.targets.len()
    }

    pub fn get_single(&self) -> Option<TurnId> {
        if self.targets.len() == 1 {
            Some(self.targets[0].turn_id)
        } else {
            None
        }
    }

    /// Get all targets that are in the current branch (not abandoned)
    pub fn current_branch_targets(&self) -> Vec<&ForwardTarget> {
        self.targets
            .iter()
            .filter(|t| !t.is_abandoned_future)
            .collect()
    }

    /// Get all abandoned future targets
    pub fn abandoned_future_targets(&self) -> Vec<&ForwardTarget> {
        self.targets
            .iter()
            .filter(|t| t.is_abandoned_future)
            .collect()
    }
}

/// Timeline navigation context
#[derive(Debug, Clone)]
pub struct NavigationContext {
    pub cursor: TimelineCursor,
    pub branch_head_turn_id: Option<TurnId>,
    pub is_branch_head: bool,
}

impl NavigationContext {
    pub fn new(session_id: SessionId, branch_id: BranchId, branch_head: Option<TurnId>) -> Self {
        Self {
            cursor: TimelineCursor::new(session_id, branch_id),
            branch_head_turn_id: branch_head,
            is_branch_head: true,
        }
    }

    /// Get the effective turn ID (where we are now conceptually)
    pub fn effective_turn_id(&self) -> Option<TurnId> {
        self.cursor.current_turn_id().or(self.branch_head_turn_id)
    }

    /// Check if cursor position matches branch head
    pub fn is_cursor_at_branch_head(&self) -> bool {
        self.cursor.is_at_head()
    }

    /// Get available forward navigation targets
    pub fn get_forward_targets(&self) -> AmbiguousForwardTargets {
        let cursor_turn = self
            .cursor
            .current_turn_id()
            .or(self.branch_head_turn_id)
            .unwrap_or_else(|| TurnId::new());

        let mut result = AmbiguousForwardTargets::new(cursor_turn);

        // Get children for the current position
        let children = if self.cursor.is_at_head() {
            if let Some(head) = self.branch_head_turn_id {
                self.cursor.navigate_forward_from(head)
            } else {
                Vec::new()
            }
        } else {
            self.cursor.navigate_forward()
        };

        for child in children {
            result.add_target(ForwardTarget {
                turn_id: child,
                branch_id: self.cursor.branch_id(),
                branch_name: None,
                is_abandoned_future: false,
            });
        }

        result
    }

    /// Get forward targets with branch info (for multi-branch scenarios)
    pub fn get_branch_aware_forward_targets(
        &self,
        branch_info: &HashMap<TurnId, (BranchId, Option<String>, bool)>,
    ) -> AmbiguousForwardTargets {
        let cursor_turn = self
            .cursor
            .current_turn_id()
            .or(self.branch_head_turn_id)
            .unwrap_or_else(TurnId::new);

        let mut result = AmbiguousForwardTargets::new(cursor_turn);

        // Get children for the current position
        let children = if self.cursor.is_at_head() {
            if let Some(head) = self.branch_head_turn_id {
                self.cursor.navigate_forward_from(head)
            } else {
                Vec::new()
            }
        } else {
            self.cursor.navigate_forward()
        };

        for child in children {
            if let Some((branch_id, branch_name, is_abandoned)) = branch_info.get(&child) {
                result.add_target(ForwardTarget {
                    turn_id: child,
                    branch_id: *branch_id,
                    branch_name: branch_name.clone(),
                    is_abandoned_future: *is_abandoned,
                });
            } else {
                // Unknown branch info - assume current branch, not abandoned
                result.add_target(ForwardTarget {
                    turn_id: child,
                    branch_id: self.cursor.branch_id(),
                    branch_name: None,
                    is_abandoned_future: false,
                });
            }
        }

        result
    }

    /// Can we navigate backward from current position?
    pub fn can_navigate_backward(&self) -> bool {
        self.cursor.current_turn_id().is_some()
    }

    /// Can we navigate forward from current position?
    pub fn has_forward_targets(&self) -> bool {
        !self.cursor.navigate_forward().is_empty()
    }

    /// Navigate forward and return selected turn ID (or ambiguous)
    pub fn navigate_forward_and_select(
        &mut self,
        branch_info: &HashMap<TurnId, (BranchId, Option<String>, bool)>,
    ) -> Result<TurnId, AmbiguousForwardTargets> {
        let targets = self.get_branch_aware_forward_targets(branch_info);

        if targets.has_none() {
            // No forward targets - we're at the head
            return Err(targets);
        }

        if targets.has_multiple() {
            // Ambiguous - need user to select
            return Err(targets);
        }

        // Single target - select it automatically
        if let Some(turn_id) = targets.get_single() {
            self.cursor.go_to_turn(turn_id);
            self.is_branch_head = false;
            Ok(turn_id)
        } else {
            Err(targets)
        }
    }

    /// Navigate backward and update cursor
    pub fn navigate_backward_to(&mut self, parent_turn_id: TurnId) {
        self.cursor.go_to_turn(parent_turn_id);
        self.is_branch_head = false;
    }

    /// Return to branch head
    pub fn return_to_head(&mut self) {
        self.cursor.go_to_head();
        self.is_branch_head = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_new_at_head() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let cursor = TimelineCursor::new(session_id, branch_id);
        assert!(cursor.is_at_head());
        assert!(cursor.current_turn_id().is_none());
    }

    #[test]
    fn test_cursor_go_to_turn() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut cursor = TimelineCursor::new(session_id, branch_id);
        let turn_id = TurnId::new();
        cursor.go_to_turn(turn_id);
        assert!(cursor.is_at_turn(turn_id));
        assert_eq!(cursor.current_turn_id(), Some(turn_id));
    }

    #[test]
    fn test_cursor_go_to_head() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let turn_id = TurnId::new();
        let mut cursor = TimelineCursor::new(session_id, branch_id);
        cursor.go_to_turn(turn_id);
        cursor.go_to_head();
        assert!(cursor.is_at_head());
    }

    #[test]
    fn test_cursor_navigate_backward() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let cursor = TimelineCursor::new(session_id, branch_id);
        let parent_id = TurnId::new();
        let result = cursor.navigate_backward(Some(parent_id));
        assert_eq!(result, Some(parent_id));
    }

    #[test]
    fn test_cursor_navigate_backward_no_parent() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let cursor = TimelineCursor::new(session_id, branch_id);
        let result = cursor.navigate_backward(None);
        assert!(result.is_none());
    }

    #[test]
    fn test_cursor_navigate_forward() {
        let mut session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut cursor = TimelineCursor::new(session_id, branch_id);
        let parent_id = TurnId::new();
        let child1 = TurnId::new();
        let child2 = TurnId::new();
        cursor.register_children(parent_id, vec![child1.clone(), child2.clone()]);
        cursor.go_to_turn(parent_id);

        let forward = cursor.navigate_forward();
        assert_eq!(forward.len(), 2);
        assert!(forward.contains(&child1));
        assert!(forward.contains(&child2));
    }

    #[test]
    fn test_cursor_navigate_forward_at_head() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let cursor = TimelineCursor::new(session_id, branch_id);
        // At head, no forward navigation
        let forward = cursor.navigate_forward();
        assert!(forward.is_empty());
    }

    #[test]
    fn test_ambiguous_forward_targets() {
        let turn_id = TurnId::new();
        let branch_id = BranchId::new();
        let mut result = AmbiguousForwardTargets::new(turn_id);
        assert!(result.has_none());

        result.add_target(ForwardTarget {
            turn_id: TurnId::new(),
            branch_id,
            branch_name: Some("feature".to_string()),
            is_abandoned_future: false,
        });
        result.add_target(ForwardTarget {
            turn_id: TurnId::new(),
            branch_id,
            branch_name: Some("main".to_string()),
            is_abandoned_future: true,
        });

        assert!(result.has_multiple());
        assert_eq!(result.targets.len(), 2);
    }

    #[test]
    fn test_navigation_context_effective_turn_id_cursor_takes_precedence() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let branch_head = TurnId::new();
        let cursor_turn = TurnId::new();

        let mut ctx = NavigationContext::new(session_id, branch_id, Some(branch_head));
        ctx.cursor.go_to_turn(cursor_turn);

        assert_eq!(ctx.effective_turn_id(), Some(cursor_turn));
    }

    #[test]
    fn test_navigation_context_effective_turn_id_falls_back_to_branch_head() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let branch_head = TurnId::new();

        let ctx = NavigationContext::new(session_id, branch_id, Some(branch_head));
        // No cursor set, should fall back to branch head
        assert_eq!(ctx.effective_turn_id(), Some(branch_head));
    }

    #[test]
    fn test_forward_target_creation() {
        let branch_id = BranchId::new();
        let target = ForwardTarget {
            turn_id: TurnId::new(),
            branch_id,
            branch_name: Some("feature".to_string()),
            is_abandoned_future: true,
        };
        assert!(target.is_abandoned_future);
        assert_eq!(target.branch_name, Some("feature".to_string()));
        assert_eq!(target.branch_id, branch_id);
    }

    #[test]
    fn test_ambiguous_forward_targets_current_branch_targets() {
        let branch_id = BranchId::new();
        let mut result = AmbiguousForwardTargets::new(TurnId::new());

        result.add_target(ForwardTarget {
            turn_id: TurnId::new(),
            branch_id,
            branch_name: Some("feature".to_string()),
            is_abandoned_future: false,
        });
        result.add_target(ForwardTarget {
            turn_id: TurnId::new(),
            branch_id,
            branch_name: Some("main".to_string()),
            is_abandoned_future: true,
        });

        let current = result.current_branch_targets();
        assert_eq!(current.len(), 1);

        let abandoned = result.abandoned_future_targets();
        assert_eq!(abandoned.len(), 1);
    }

    #[test]
    fn test_navigation_context_navigate_forward_single_target() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let branch_head = TurnId::new();
        let mut ctx = NavigationContext::new(session_id, branch_id, Some(branch_head));

        // Set up children on the cursor for the branch head
        let child_turn = TurnId::new();
        ctx.cursor.register_children(branch_head, vec![child_turn]);

        let branch_info = HashMap::new();
        let result = ctx.navigate_forward_and_select(&branch_info);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), child_turn);
    }

    #[test]
    fn test_navigation_context_return_to_head() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut ctx = NavigationContext::new(session_id, branch_id, Some(TurnId::new()));

        ctx.cursor.go_to_turn(TurnId::new());
        assert!(!ctx.is_cursor_at_branch_head());

        ctx.return_to_head();
        assert!(ctx.is_cursor_at_branch_head());
    }

    #[test]
    fn test_navigation_context_navigate_backward() {
        let session_id = SessionId::new();
        let branch_id = BranchId::new();
        let mut ctx = NavigationContext::new(session_id, branch_id, Some(TurnId::new()));

        let parent = TurnId::new();
        ctx.navigate_backward_to(parent);
        assert_eq!(ctx.cursor.current_turn_id(), Some(parent));
        assert!(!ctx.is_cursor_at_branch_head());
    }

    #[test]
    fn test_ambiguous_forward_targets_get_single() {
        let mut result = AmbiguousForwardTargets::new(TurnId::new());
        assert!(result.get_single().is_none());

        let turn_id = TurnId::new();
        result.add_target(ForwardTarget {
            turn_id,
            branch_id: BranchId::new(),
            branch_name: None,
            is_abandoned_future: false,
        });

        assert_eq!(result.get_single(), Some(turn_id));

        // Add another - now it's ambiguous
        result.add_target(ForwardTarget {
            turn_id: TurnId::new(),
            branch_id: BranchId::new(),
            branch_name: None,
            is_abandoned_future: false,
        });

        assert!(result.get_single().is_none());
    }
}
