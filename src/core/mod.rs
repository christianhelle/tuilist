//! The engine: it owns every store write and talks to the task service.

pub mod fake;
pub mod sync;

pub use sync::{SyncError, sync_lists, sync_tasks};

