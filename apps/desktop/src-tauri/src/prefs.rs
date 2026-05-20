//! Desktop app preferences persisted under the Tauri app data directory.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const PREFS_FILE: &str = "desktop_prefs.json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct DesktopPrefs {
    #[serde(skip_serializing_if = "Option::is_none")]
    last_workspace: Option<String>,
}

fn prefs_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(PREFS_FILE))
}

fn load_prefs(app: &AppHandle) -> Result<DesktopPrefs, String> {
    let path = prefs_path(app)?;
    if !path.is_file() {
        return Ok(DesktopPrefs::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn save_prefs(app: &AppHandle, prefs: &DesktopPrefs) -> Result<(), String> {
    let path = prefs_path(app)?;
    let raw = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

pub fn save_last_workspace(app: &AppHandle, workspace: &Path) -> Result<(), String> {
    let mut prefs = load_prefs(app)?;
    prefs.last_workspace = Some(workspace.to_string_lossy().into_owned());
    save_prefs(app, &prefs)
}

pub fn clear_last_workspace(app: &AppHandle) -> Result<(), String> {
    let mut prefs = load_prefs(app)?;
    prefs.last_workspace = None;
    save_prefs(app, &prefs)
}

pub fn load_last_workspace(app: &AppHandle) -> Result<Option<String>, String> {
    Ok(load_prefs(app)?.last_workspace)
}

pub fn last_workspace_if_valid(app: &AppHandle) -> Result<Option<PathBuf>, String> {
    let Some(raw) = load_last_workspace(app)? else {
        return Ok(None);
    };
    let path = PathBuf::from(&raw);
    if path.is_dir() {
        Ok(Some(path))
    } else {
        let _ = clear_last_workspace(app);
        Ok(None)
    }
}
