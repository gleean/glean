//! Workspace root resolution for daemon, MCP, and status.

use std::path::{Path, PathBuf};

use crate::HostError;

/// Resolve workspace root: CLI override, then `GLEAN_WORKSPACE_ROOT`, then cwd.
pub fn resolve_workspace_root(override_root: Option<PathBuf>) -> Result<PathBuf, HostError> {
    let root = override_root
        .or_else(|| {
            std::env::var("GLEAN_WORKSPACE_ROOT")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));
    Ok(root.canonicalize().unwrap_or(root))
}

/// Same as [`resolve_workspace_root`] with no override (MCP / status default).
pub fn resolve_workspace_from_env() -> Result<PathBuf, HostError> {
    resolve_workspace_root(None)
}

#[allow(dead_code)]
pub fn is_under_workspace(workspace: &Path, path: &Path) -> bool {
    path.starts_with(workspace)
}
