mod commands;
mod daemon_sidecar;
mod prefs;
mod state;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Pin global storage for the whole process tree: CLI/daemon consult `GLEAN_STORAGE_ROOT`;
            // the packaged app must not rely on cwd for an implicit default.
            if std::env::var_os("GLEAN_STORAGE_ROOT").is_none() {
                if let Ok(layout) = glean_core::GlobalLayout::from_env_or_default() {
                    std::env::set_var("GLEAN_STORAGE_ROOT", layout.root.as_os_str());
                }
            }
            app.manage(Arc::new(Mutex::new(state::AppInner::default())) as AppState);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pick_workspace,
            commands::try_restore_workspace,
            commands::get_status,
            commands::semantic_search,
            commands::daemon_running,
            commands::current_workspace,
            commands::recent_changes,
            commands::read_file_context,
            commands::get_global_config_toml,
            commands::set_global_config_key,
            commands::init_global_config,
            commands::reveal_path_in_file_manager,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Ok(mut guard) = window.state::<AppState>().try_lock() {
                    if let Some(mut sidecar) = guard.sidecar.take() {
                        sidecar.stop();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running glean desktop");
}
