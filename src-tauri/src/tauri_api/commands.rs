use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use tauri::TitleBarStyle;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::{
    domain::{
        config::DiavolaConfig,
        project::{ProjectId, ProjectRecord},
        runtime::RunSessionSnapshot,
        terminal::{TerminalSessionId, TerminalSnapshot},
    },
    error::{AppError, ErrorCode},
    infrastructure::{
        config_loader::{load_config_async, parse_config_document},
        git_info::{self, GitInfo},
    },
    tauri_api::state::AppState,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadProjectConfigRequest {
    pub project_id: ProjectId,
    pub yaml: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigDocument {
    pub project: ProjectRecord,
    pub yaml: String,
    pub config: DiavolaConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProjectConfigRequest {
    pub project_id: ProjectId,
    pub yaml: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchProjectInfo {
    pub project: Option<ProjectRecord>,
    pub locked: bool,
    pub auto_run: bool,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectActionRequest {
    pub project_id: ProjectId,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessActionRequest {
    pub process_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenTerminalRequest {
    pub project_id: ProjectId,
    pub title: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteTerminalRequest {
    pub terminal_id: TerminalSessionId,
    pub data: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizeTerminalRequest {
    pub terminal_id: TerminalSessionId,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseTerminalRequest {
    pub terminal_id: TerminalSessionId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppErrorPayload {
    message: String,
    code: ErrorCode,
}

fn to_error_string(error: AppError) -> String {
    let payload = AppErrorPayload {
        message: error.to_string(),
        code: error.code(),
    };
    serde_json::to_string(&payload).unwrap_or_else(|_| error.to_string())
}

fn window_key(window: &WebviewWindow) -> String {
    window.label().to_string()
}

async fn check_launch_locked(state: &AppState) -> Result<(), String> {
    if state.current_project.lock().await.is_some() {
        return Err(AppError::LaunchLocked.to_string());
    }
    Ok(())
}

async fn current_project_or_err(state: &AppState) -> Result<ProjectRecord, String> {
    state
        .current_project
        .lock()
        .await
        .clone()
        .ok_or_else(|| AppError::project_store("workspace project not initialized").to_string())
}

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectRecord>, String> {
    let store = state.project_store.lock().await;
    store.load().map_err(to_error_string)
}

#[tauri::command]
pub async fn get_launch_project(state: State<'_, AppState>) -> Result<LaunchProjectInfo, String> {
    let project = state.current_project.lock().await.clone();
    let auto_run = *state.launch_auto_run.lock().await;
    let mut launch_error = state.launch_error.lock().await;
    Ok(LaunchProjectInfo {
        locked: project.is_some(),
        project,
        auto_run,
        error: launch_error.take(),
    })
}

#[tauri::command]
pub async fn load_project_config(
    state: State<'_, AppState>,
    request: LoadProjectConfigRequest,
) -> Result<Option<ProjectConfigDocument>, String> {
    let project = current_project_or_err(&state).await?;
    if request.project_id != project.id {
        return Err(AppError::LaunchLocked.to_string());
    }
    let yaml = {
        let store = state.project_store.lock().await;
        match request.yaml {
            Some(yaml) => Some(yaml),
            None => store
                .load_project_config_raw_optional(&project)
                .map_err(to_error_string)?,
        }
    };
    let Some(yaml) = yaml else {
        return Ok(None);
    };

    let loaded = parse_config_document(&project.config_path, &yaml).map_err(to_error_string)?;
    Ok(Some(ProjectConfigDocument {
        project,
        yaml,
        config: loaded.config,
    }))
}

#[tauri::command]
pub async fn save_project_config(
    state: State<'_, AppState>,
    request: SaveProjectConfigRequest,
) -> Result<ProjectConfigDocument, String> {
    let project = current_project_or_err(&state).await?;
    if request.project_id != project.id {
        return Err(AppError::LaunchLocked.to_string());
    }
    let yaml = request.yaml;
    let loaded = parse_config_document(&project.config_path, &yaml).map_err(to_error_string)?;
    let store = state.project_store.lock().await;
    store
        .save_project_config_raw(&project, &yaml)
        .map_err(to_error_string)?;
    Ok(ProjectConfigDocument {
        project,
        yaml,
        config: loaded.config,
    })
}

#[tauri::command]
pub async fn start_project(
    app_handle: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: ProjectActionRequest,
) -> Result<RunSessionSnapshot, String> {
    let project = current_project_or_err(&state).await?;
    if project.id != request.project_id {
        return Err(AppError::LaunchLocked.to_string());
    }

    let loaded = load_config_async(&project.config_path)
        .await
        .map_err(to_error_string)?;
    state
        .orchestrator
        .start_session(app_handle, window_key(&window), project, loaded)
        .await
        .map_err(to_error_string)
}

#[tauri::command]
pub async fn stop_project(
    app_handle: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<RunSessionSnapshot>, String> {
    let key = window_key(&window);
    let snapshot = state
        .orchestrator
        .stop_session(app_handle.clone(), &key)
        .await
        .map_err(to_error_string)?;
    state
        .terminal_manager
        .close_all_for_window(app_handle, &key)
        .await
        .map_err(to_error_string)?;
    Ok(snapshot)
}

#[tauri::command]
pub async fn restart_process(
    app_handle: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: ProcessActionRequest,
) -> Result<Option<RunSessionSnapshot>, String> {
    state
        .orchestrator
        .restart_process(app_handle, &window_key(&window), &request.process_name)
        .await
        .map_err(to_error_string)
}

#[tauri::command]
pub async fn start_process(
    app_handle: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: ProcessActionRequest,
) -> Result<Option<RunSessionSnapshot>, String> {
    state
        .orchestrator
        .start_process(app_handle, &window_key(&window), &request.process_name)
        .await
        .map_err(to_error_string)
}

#[tauri::command]
pub async fn stop_process(
    app_handle: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: ProcessActionRequest,
) -> Result<Option<RunSessionSnapshot>, String> {
    state
        .orchestrator
        .stop_process(app_handle, &window_key(&window), &request.process_name)
        .await
        .map_err(to_error_string)
}

#[tauri::command]
pub async fn get_session_snapshot(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<RunSessionSnapshot>, String> {
    state
        .orchestrator
        .snapshot(&window_key(&window))
        .await
        .map_err(to_error_string)
}

/// Reports whether any window has an active (not-yet-stopped) project
/// session. The frontend uses this to refuse an update install/relaunch
/// while supervised processes are still running anywhere, since an update
/// install must not silently orphan them.
#[tauri::command]
pub async fn has_active_sessions(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.orchestrator.has_active_session().await)
}

#[tauri::command]
pub async fn open_terminal(
    app_handle: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: OpenTerminalRequest,
) -> Result<TerminalSnapshot, String> {
    let project = current_project_or_err(&state).await?;
    if project.id != request.project_id {
        return Err(AppError::LaunchLocked.to_string());
    }
    state
        .terminal_manager
        .open_terminal(
            app_handle,
            window_key(&window),
            request
                .title
                .unwrap_or_else(|| format!("{} shell", project.name)),
            &project.base_dir,
            request.cols.unwrap_or(100),
            request.rows.unwrap_or(28),
        )
        .await
        .map_err(to_error_string)
}

#[tauri::command]
pub async fn write_terminal(
    state: State<'_, AppState>,
    request: WriteTerminalRequest,
) -> Result<(), String> {
    state
        .terminal_manager
        .write_terminal(&request.terminal_id, &request.data)
        .await
        .map_err(to_error_string)
}

#[tauri::command]
pub async fn resize_terminal(
    state: State<'_, AppState>,
    request: ResizeTerminalRequest,
) -> Result<(), String> {
    state
        .terminal_manager
        .resize_terminal(&request.terminal_id, request.cols, request.rows)
        .await
        .map_err(to_error_string)
}

#[tauri::command]
pub async fn close_terminal(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    request: CloseTerminalRequest,
) -> Result<Option<TerminalSnapshot>, String> {
    state
        .terminal_manager
        .close_terminal(app_handle, &request.terminal_id)
        .await
        .map_err(to_error_string)
}

#[tauri::command]
pub fn get_git_info(base_dir: String) -> Result<GitInfo, String> {
    Ok(git_info::detect_git_info(std::path::Path::new(&base_dir)))
}

#[tauri::command]
pub async fn open_project_window(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    request: ProjectActionRequest,
) -> Result<(), String> {
    check_launch_locked(&state).await?;

    let project = {
        let store = state.project_store.lock().await;
        store
            .get(&request.project_id)
            .map_err(to_error_string)?
            .ok_or_else(|| AppError::project_store("project not found").to_string())?
    };

    let project_id = project.id.0.to_string();
    let label = format!("project-{project_id}");
    if let Some(window) = app_handle.get_webview_window(&label) {
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    let url = WebviewUrl::App(format!("?projectId={project_id}&autorun=1").into());
    let win_builder = WebviewWindowBuilder::new(&app_handle, label, url)
        .title(format!("{} — Diavola", project.name));

    #[cfg(target_os = "macos")]
    let win_builder = win_builder.title_bar_style(TitleBarStyle::Overlay);

    #[cfg(not(target_os = "macos"))]
    let win_builder = win_builder.decorations(false);

    win_builder.build().map_err(|error| error.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;

    use super::*;
    use crate::domain::project::ProjectSource;

    #[test]
    fn launch_project_info_serializes_workspace_project() {
        let project = ProjectRecord {
            id: ProjectId::new(),
            name: "demo-app".to_string(),
            base_dir: PathBuf::from("/tmp/demo-app"),
            config_source: ProjectSource::ProjectFile,
            config_path: PathBuf::from("/tmp/demo-app/diavola.yml"),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let payload = serde_json::to_value(LaunchProjectInfo {
            project: Some(project),
            locked: true,
            auto_run: false,
            error: None,
        })
        .expect("serialize launch info");

        assert!(payload.get("project").is_some());
        assert_eq!(
            payload.get("autoRun").and_then(|value| value.as_bool()),
            Some(false)
        );
        assert!(payload.get("projectId").is_none());
    }
}
