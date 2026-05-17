//! Filesystem layout for local index state.

mod layout;

pub use layout::{
    open_global, open_index_for_workspace, GlobalLayout, StorageLayout, WorkspaceIndexLayout,
};
