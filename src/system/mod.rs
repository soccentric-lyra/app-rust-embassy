//! Runtime services shared by the tasks.
//!
//! - Author: Sandesh Ghimire <sandesh@soccentric.com>
//! - SRS: RGEN-002, RRT-003, RRT-004, RDBG-001

// Declared first so the log macros are visible to the modules below.
#[macro_use]
pub mod log;

pub mod build_info;
pub mod ipc;
