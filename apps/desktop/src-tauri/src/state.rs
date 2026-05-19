//! Shared application state (workspace, read-only engine, daemon sidecar).

use std::path::PathBuf;
use std::sync::Arc;

use glean_core::{open_global, GleanConfig, GleanEngine};
use glean_host::parsers::build_default_registry;
use tokio::sync::Mutex;

use crate::daemon_sidecar::{DaemonSidecar, SidecarError};

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error(transparent)]
    Sidecar(#[from] SidecarError),
    #[error(transparent)]
    Core(#[from] glean_core::CoreError),
    #[error(transparent)]
    Storage(#[from] glean_core::StorageError),
    #[error(transparent)]
    Host(#[from] glean_host::HostError),
    #[error("no workspace selected")]
    NoWorkspace,
    #[error("engine not open")]
    NoEngine,
}

#[derive(Default)]
pub struct AppInner {
    pub workspace: Option<PathBuf>,
    pub engine: Option<Arc<GleanEngine>>,
    pub sidecar: Option<DaemonSidecar>,
}

pub type AppState = Arc<Mutex<AppInner>>;

impl AppInner {
    pub async fn set_workspace(&mut self, workspace: PathBuf) -> Result<(), StateError> {
        if let Some(mut old) = self.sidecar.take() {
            old.stop();
        }
        self.engine = None;

        std::env::set_var(
            "GLEAN_WORKSPACE_ROOT",
            workspace.to_string_lossy().as_ref(),
        );

        let sidecar = DaemonSidecar::spawn(&workspace)?;
        self.sidecar = Some(sidecar);

        let cfg = GleanConfig::load_merged()?;
        let global = open_global()?;
        let engine = GleanEngine::open_for_workspace(
            &workspace,
            global,
            build_default_registry(),
            cfg,
        )
        .await?;

        self.workspace = Some(workspace);
        self.engine = Some(engine);
        Ok(())
    }

    pub fn workspace(&self) -> Result<&PathBuf, StateError> {
        self.workspace.as_ref().ok_or(StateError::NoWorkspace)
    }

    pub fn engine(&self) -> Result<Arc<GleanEngine>, StateError> {
        self.engine.clone().ok_or(StateError::NoEngine)
    }

    pub fn daemon_running(&mut self) -> bool {
        self.sidecar
            .as_mut()
            .map(|s| s.is_running())
            .unwrap_or(false)
    }
}
