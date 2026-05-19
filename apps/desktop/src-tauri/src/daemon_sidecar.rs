//! Spawn and manage `glean daemon` as a sidecar process (single writer).

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use tracing::{info, warn};

#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("glean binary not found at {0}")]
    BinaryNotFound(PathBuf),
    #[error("failed to spawn glean daemon: {0}")]
    Spawn(#[from] std::io::Error),
}

pub struct DaemonSidecar {
    child: Child,
}

impl DaemonSidecar {
    pub fn spawn(workspace: &Path) -> Result<Self, SidecarError> {
        let binary = resolve_glean_binary()?;
        if !binary.is_file() {
            return Err(SidecarError::BinaryNotFound(binary));
        }

        info!(
            binary = %binary.display(),
            workspace = %workspace.display(),
            "spawning glean daemon sidecar",
        );

        let child = Command::new(&binary)
            .arg("daemon")
            .arg("--workspace")
            .arg(workspace)
            .env("GLEAN_WORKSPACE_ROOT", workspace.to_string_lossy().as_ref())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        Ok(Self { child })
    }

    pub fn is_running(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(None) => true,
            Ok(Some(status)) => {
                warn!(?status, "glean daemon sidecar exited");
                false
            }
            Err(e) => {
                warn!(error = %e, "glean daemon sidecar status check failed");
                false
            }
        }
    }

    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for DaemonSidecar {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Glean CLI next to the desktop executable (Tauri `externalBin` in release bundles).
fn bundled_glean_next_to_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    #[cfg(windows)]
    {
        let win = dir.join("glean.exe");
        if win.is_file() {
            return Some(win);
        }
    }
    let path = dir.join("glean");
    if path.is_file() {
        return Some(path);
    }
    None
}

/// Resolve the `glean` CLI: `GLEAN_BIN`, bundled sidecar, then monorepo `target/*/glean`.
pub fn resolve_glean_binary() -> Result<PathBuf, SidecarError> {
    if let Ok(path) = std::env::var("GLEAN_BIN") {
        let p = PathBuf::from(path);
        if p.is_file() {
            return Ok(p);
        }
    }

    if let Some(p) = bundled_glean_next_to_exe() {
        return Ok(p);
    }

    // apps/desktop/src-tauri -> ../../../target/{debug,release}/glean (local dev)
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let debug = manifest.join("../../../target/debug/glean");
    if debug.is_file() {
        return Ok(debug.canonicalize().unwrap_or(debug));
    }

    let release = manifest.join("../../../target/release/glean");
    if release.is_file() {
        return Ok(release.canonicalize().unwrap_or(release));
    }

    Err(SidecarError::BinaryNotFound(debug))
}
