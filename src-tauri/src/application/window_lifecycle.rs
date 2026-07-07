use tauri::{AppHandle, Manager, WebviewWindow};

use crate::tauri_api::state::AppState;

/// Registers a close handler for a window that, on user-initiated close,
/// force-stops the project session and integrated terminals attached to that
/// window, then destroys the window.
///
/// `destroy()` is used (not `close()`) so the cleanup does not re-trigger
/// `CloseRequested` and loop. The original implementation only attached a
/// close handler to the `"main"` window, so closing a project window that was
/// hosting a running session would skip cleanup and leave every supervised
/// child process orphaned.
///
/// This MUST be attached to every window that can host a session: the main
/// launcher window AND each project window opened via `open_project_window`.
pub fn register_window_close_handler(
    app_handle: AppHandle,
    state: AppState,
    window: &WebviewWindow,
) {
    let app_handle = app_handle;
    let state = state;
    let window_label = window.label().to_string();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let app_handle = app_handle.clone();
            let state = state.clone();
            let label = window_label.clone();
            tauri::async_runtime::spawn(async move {
                let _ = state
                    .orchestrator
                    .force_stop_session(app_handle.clone(), &label)
                    .await;
                let _ = state
                    .terminal_manager
                    .close_all_for_window(app_handle.clone(), &label)
                    .await;
                if let Some(win) = app_handle.get_webview_window(&label) {
                    let _ = win.destroy();
                }
            });
        }
    });
}
