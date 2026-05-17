//! `glean config`: thin CLI over `glean-host` config editor.

use std::path::PathBuf;

use anyhow::{Context, Result};

/// Print merged `GleanConfig` as TOML (stdout).
pub fn run_config_list(_workspace: Option<PathBuf>, show_sources: bool) -> Result<()> {
    if show_sources {
        let global =
            glean_core::GlobalLayout::from_env_or_default().map_err(|e| anyhow::anyhow!(e))?;
        let header = glean_host::config::format_section_provenance(&global)
            .map_err(|e| anyhow::anyhow!(e))?;
        print!("{header}\n\n");
    }
    let text = glean_host::config::merged_config_toml().context("load merged config")?;
    print!("{text}");
    Ok(())
}

/// Initialize `$GLEAN_STORAGE_ROOT/config.toml` (`--force` overwrites).
pub fn run_config_init(_workspace_flag: Option<PathBuf>, force: bool) -> Result<()> {
    let path = glean_host::config::init_global_config(force).map_err(|e| anyhow::anyhow!(e))?;
    eprintln!("Wrote {}", path.display());
    Ok(())
}

/// Set a single scalar in `$GLEAN_STORAGE_ROOT/config.toml`.
pub fn run_config_set(_workspace: Option<PathBuf>, key: String, value: String) -> Result<()> {
    let path =
        glean_host::config::set_global_key(key.clone(), value).map_err(|e| anyhow::anyhow!(e))?;
    let (section, field) = key.split_once('.').unwrap_or(("", ""));
    eprintln!("Updated {} → [{}].{}", path.display(), section, field);
    Ok(())
}
