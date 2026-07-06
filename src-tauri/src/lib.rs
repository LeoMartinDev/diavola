pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod tauri_api;

use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tauri::Manager;
use tracing::{error, info};

use crate::infrastructure::config_loader::{find_config_in_cwd_or_parents, load_config};
use crate::tauri_api::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    // intentionally fatal — the app cannot run without app state
    let app_state = AppState::new().expect("failed to initialize app state");
    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let closing = Arc::new(AtomicBool::new(false));

            if let Some(window) = app.get_webview_window("main") {
                #[cfg(not(target_os = "macos"))]
                {
                    if let Err(err) = window.set_decorations(false) {
                        warn!(error = %err, "failed to disable window decorations");
                    }
                }

                #[cfg(target_os = "macos")]
                {
                    use objc2_app_kit::{NSWindow, NSWindowTitleVisibility};
                    if let Ok(ns_window) = window.ns_window() {
                        let ns_window = ns_window as *mut NSWindow;
                        unsafe {
                            (*ns_window).setTitleVisibility(NSWindowTitleVisibility::Hidden);
                        }
                    }
                }

                let app_handle = app.handle().clone();
                let state = app.state::<AppState>().inner().clone();
                let closing_flag = closing.clone();
                window.clone().on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if closing_flag.load(Ordering::Acquire) {
                            return;
                        }
                        closing_flag.store(true, Ordering::Release);
                        api.prevent_close();
                        let app_handle = app_handle.clone();
                        let state = state.clone();
                        let window_label = window.label().to_string();
                        tauri::async_runtime::spawn(async move {
                            let _ = state
                                .orchestrator
                                .force_stop_session(app_handle.clone(), &window_label)
                                .await;
                            let _ = state
                                .terminal_manager
                                .close_all_for_window(
                                    app_handle.clone(),
                                    &window_label,
                                )
                                .await;
                            if let Some(w) = app_handle.get_webview_window(&window_label) {
                                let _ = w.close();
                            }
                        });
                    }
                });
            }

            let state = app.state::<AppState>().inner().clone();
            tauri::async_runtime::block_on(async move {
                let cwd = std::env::current_dir().ok();
                let cli_config_path = launch_config_path();
                let auto_detected_path = find_config_in_cwd_or_parents();
                let config_path = cli_config_path
                    .clone()
                    .or_else(|| auto_detected_path.clone());

                info!(
                    cwd = ?cwd,
                    cli_path = ?cli_config_path,
                    auto_detected_path = ?auto_detected_path,
                    selected_path = ?config_path,
                    "launch config detection"
                );

                let workspace_project = {
                    let store = state.project_store.lock().await;
                    match config_path.clone() {
                        Some(config_path) => store.prepare_project_record_from_config_path(config_path)?,
                        None => {
                            let cwd = cwd.clone().ok_or_else(|| {
                                crate::error::AppError::project_store(
                                    "unable to resolve current working directory",
                                )
                            })?;
                            store.prepare_workspace_project(cwd)?
                        }
                    }
                };
                let mut current_project = state.current_project.lock().await;
                *current_project = Some(workspace_project.clone());
                drop(current_project);
                let mut launch_auto_run = state.launch_auto_run.lock().await;
                *launch_auto_run = false;
                drop(launch_auto_run);

                if let Some(config_path) = config_path {
                    if cli_config_path.is_some() {
                        info!(project_id = ?workspace_project.id, "prepared CLI config project");
                        load_config(&workspace_project.config_path)?;
                        let mut launch_auto_run = state.launch_auto_run.lock().await;
                        *launch_auto_run = true;
                    } else {
                        let config_path_for_error = config_path.clone();
                        if let Err(error) = load_config(&workspace_project.config_path) {
                            info!(error = %error, "failed to load auto-detected config");
                            let mut launch_error = state.launch_error.lock().await;
                            *launch_error = Some(format!(
                                "Failed to load auto-detected config {}: {}",
                                config_path_for_error.display(), error
                            ));
                        } else {
                            let mut launch_auto_run = state.launch_auto_run.lock().await;
                            *launch_auto_run = true;
                        }
                    }
                }
                Ok::<(), crate::error::AppError>(())
            })
            .map_err(|err| {
                error!(error = %err, code = ?err.code(), "failed to import launch config");
                Box::<dyn std::error::Error>::from(err.to_string())
            })?;

            let app_handle = app.handle().clone();
            let state = app.state::<AppState>().inner().clone();
            tauri::async_runtime::spawn(async move {
                shutdown_signal_handler(app_handle, state).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tauri_api::commands::get_launch_project,
            tauri_api::commands::list_projects,
            tauri_api::commands::load_project_config,
            tauri_api::commands::save_project_config,
            tauri_api::commands::start_project,
            tauri_api::commands::stop_project,
            tauri_api::commands::restart_process,
            tauri_api::commands::start_process,
            tauri_api::commands::stop_process,
            tauri_api::commands::get_session_snapshot,
            tauri_api::commands::has_active_sessions,
            tauri_api::commands::open_terminal,
            tauri_api::commands::write_terminal,
            tauri_api::commands::resize_terminal,
            tauri_api::commands::close_terminal,
            tauri_api::commands::open_project_window,
            tauri_api::commands::get_git_info
        ])
        // intentionally fatal — the app cannot recover from a Tauri runtime failure
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn config_flag_path() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let arg_str = arg.to_string_lossy();
        if arg_str == "--config" || arg_str == "-c" {
            return args.next().map(resolve_launch_config_path);
        }
        if let Some(value) = arg_str.strip_prefix("--config=") {
            return Some(resolve_launch_config_path(value));
        }
    }
    None
}

fn launch_config_path() -> Option<PathBuf> {
    config_flag_path().or_else(|| {
        std::env::args_os()
            .skip(1)
            .find(|argument| !argument.to_string_lossy().starts_with("--"))
            .map(resolve_launch_config_path)
    })
}

fn resolve_launch_config_path(argument: impl Into<PathBuf>) -> PathBuf {
    let path = argument.into();
    if path.is_absolute() || path.exists() {
        return path;
    }

    let Ok(current_dir) = std::env::current_dir() else {
        return path;
    };

    if current_dir
        .file_name()
        .is_some_and(|name| name == "src-tauri")
    {
        let workspace_relative = current_dir
            .parent()
            .map(|workspace| workspace.join(&path))
            .filter(|candidate| candidate.exists());
        if let Some(candidate) = workspace_relative {
            return candidate;
        }
    }

    path
}

async fn shutdown_signal_handler(app_handle: tauri::AppHandle, state: AppState) {
    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        )
        .expect("failed to install SIGTERM handler");
        let mut sigint = tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::interrupt(),
        )
        .expect("failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {}
            _ = sigint.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }

    info!("shutdown signal received, stopping all sessions");
    let _ = state
        .orchestrator
        .force_stop_all_sessions(app_handle.clone())
        .await;
    let _ = state.terminal_manager.close_all(app_handle).await;
    std::process::exit(0);
}
