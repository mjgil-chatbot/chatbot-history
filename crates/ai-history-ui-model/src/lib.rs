// AI History UI Model - UI-ready read models
//
// This crate provides:
// - Timeline views
// - Checkpoint list views
// - Branch timeline views
// - File change summaries
// - Restore modal states
// - Conflict screen states
// - Accessibility and UX verification
//
// No business logic lives here - only data shapes for UI consumption.

pub mod accessibility;
pub mod ambiguous_forward;
pub mod branches;
pub mod checkpoints;
pub mod conflicts;
pub mod restore;
pub mod timeline;

pub use accessibility::*;
pub use ambiguous_forward::*;
pub use branches::*;
pub use checkpoints::*;
pub use conflicts::*;
pub use restore::*;
pub use timeline::*;
