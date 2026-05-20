//! Tauri command handlers exposed to the Next.js UI.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::prefs;
use crate::state::{AppInner, AppState, StateError};

#[derive(Serialize)]
pub struct SearchHitDto {
    pub path: String,
    pub preview: String,
}

async fn apply_workspace(
    guard: &mut AppInner,
    workspace: PathBuf,
    app: &AppHandle,
) -> Result<String, String> {
    guard
        .set_workspace(workspace.clone())
        .await
        .map_err(|e| e.to_string())?;
    prefs::save_last_workspace(app, &workspace)?;
    Ok(workspace.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn pick_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<String, String> {
    let Some(path) = path else {
        return Err("no folder selected".into());
    };
    let workspace = PathBuf::from(path);
    let mut guard = state.lock().await;
    apply_workspace(&mut guard, workspace, &app).await
}

#[tauri::command]
pub async fn try_restore_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let Some(workspace) = prefs::last_workspace_if_valid(&app)? else {
        return Ok(false);
    };
    let mut guard = state.lock().await;
    if guard
        .workspace
        .as_ref()
        .is_some_and(|current| current == &workspace)
    {
        return Ok(true);
    }
    apply_workspace(&mut guard, workspace, &app).await?;
    Ok(true)
}

#[tauri::command]
pub async fn get_status(
    state: State<'_, AppState>,
) -> Result<glean_host::status::StatusReport, String> {
    let guard = state.lock().await;
    let workspace = guard.workspace().map_err(|e: StateError| e.to_string())?;
    glean_host::status::collect_status_for_workspace(workspace).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn semantic_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SearchHitDto>, String> {
    let guard = state.lock().await;
    let engine = guard.engine().map_err(|e: StateError| e.to_string())?;
    let limit = limit.unwrap_or(32) as usize;
    let hits = engine
        .semantic_search(query.trim(), limit)
        .await
        .map_err(|e| e.to_string())?;
    Ok(hits
        .into_iter()
        .map(|(path, text)| SearchHitDto {
            preview: text.chars().take(240).collect(),
            path,
        })
        .collect())
}

#[tauri::command]
pub async fn daemon_running(state: State<'_, AppState>) -> Result<bool, String> {
    let mut guard = state.lock().await;
    Ok(guard.daemon_running())
}

#[tauri::command]
pub async fn current_workspace(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let guard = state.lock().await;
    Ok(guard
        .workspace
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
pub fn get_global_config_toml() -> Result<String, String> {
    glean_host::config::merged_config_toml().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_global_config_key(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<String, String> {
    let path = glean_host::config::set_global_key(key, value).map_err(|e| e.to_string())?;
    let mut guard = state.lock().await;
    guard
        .reload_daemon_and_config()
        .await
        .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

async fn resolve_hit_path(state: &AppState, path: String) -> Result<PathBuf, String> {
    let raw = PathBuf::from(path.trim());
    let guard = state.lock().await;
    let candidate = if raw.is_absolute() {
        raw
    } else if let Some(ws) = guard.workspace.as_ref() {
        ws.join(&raw)
    } else {
        return Err("no workspace selected".into());
    };
    drop(guard);
    if !candidate.exists() {
        return Err(format!("path does not exist: {}", candidate.display()));
    }
    candidate.canonicalize().map_err(|e| e.to_string())
}

/// Reveal a search hit in the system file explorer (NSWorkspace on macOS).
#[tauri::command]
pub async fn reveal_path_in_file_manager(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let resolved = resolve_hit_path(state.inner(), path).await?;
    let display = resolved.to_string_lossy().into_owned();
    app.opener()
        .reveal_item_in_dir(display)
        .map_err(|e| e.to_string())
}
