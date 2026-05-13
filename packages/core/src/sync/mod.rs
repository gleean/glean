//! Incremental sync primitives (pure reconcile + task execution hooks).

pub mod reconcile;

pub use reconcile::{reconcile, DbSnapshot, DiskSnapshot, SyncTask};
