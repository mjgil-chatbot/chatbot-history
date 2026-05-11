//! Actor types - who performed an action

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Actor who performed an action in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Actor {
    /// The human user
    User,
    /// The AI assistant
    Assistant,
    /// The system (automated actions)
    System,
    /// An external tool or process
    External,
}

impl Default for Actor {
    fn default() -> Self {
        Self::User
    }
}

impl Display for Actor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Actor::User => write!(f, "user"),
            Actor::Assistant => write!(f, "assistant"),
            Actor::System => write!(f, "system"),
            Actor::External => write!(f, "external"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_display() {
        assert_eq!(Actor::User.to_string(), "user");
        assert_eq!(Actor::Assistant.to_string(), "assistant");
        assert_eq!(Actor::System.to_string(), "system");
        assert_eq!(Actor::External.to_string(), "external");
    }

    #[test]
    fn test_actor_default() {
        assert_eq!(Actor::default(), Actor::User);
    }
}
