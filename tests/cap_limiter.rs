//! Test helper that sets a 64MB memory cap for all tests in this binary.
//! 
//! Usage: add this to your test files or include it in your test harness:
//! ```rust
//! #[test]
//! fn my_test() {
//!     // This test runs with cap allocator enforcing 64MB limit
//! }
//! ```
//! 
//! Each test binary should include this once, before any test modules.

use std::alloc;

#[global_allocator]
static TEST_ALLOCATOR: cap::Cap<std::alloc::System> =
    cap::Cap::new(std::alloc::System, 64 * 1024 * 1024);
