mod commands;
mod daemon_sidecar;
mod state;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(Arc::new(Mutex::new(state::AppInner::default())) as AppState);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pick_workspace,
            commands::get_status,
            commands::semantic_search,
            commands::daemon_running,
            commands::current_workspace,
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
