//! Tauri command handlers exposed to the Next.js UI.

use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::state::{AppState, StateError};

#[derive(Serialize)]
pub struct SearchHitDto {
    pub path: String,
    pub preview: String,
}

#[tauri::command]
pub async fn pick_workspace(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<String, String> {
    let Some(path) = path else {
        return Err("no folder selected".into());
    };
    let workspace = PathBuf::from(path);
    let mut guard = state.lock().await;
    guard.set_workspace(workspace.clone()).await.map_err(|e| e.to_string())?;
    Ok(workspace.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn get_status(state: State<'_, AppState>) -> Result<glean_host::status::StatusReport, String> {
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
