pub mod dependency;
pub mod lifecycle;
pub mod log;
pub mod session;

use std::{future::Future, pin::Pin, sync::Arc, thread, time::Duration};

use chrono::Utc;
use tauri::{AppHandle, Emitter};
use tokio::{
    process::Child,
    sync::{broadcast, mpsc, oneshot, Mutex},
};
use tracing::{error, info, warn};

use crate::{
    application::{
        command_runner::spawn_process,
        events::{RuntimeEvent, SessionStatusEvent, RUNTIME_ERROR_EVENT, SESSION_SNAPSHOT_EVENT},
        readiness::wait_until_ready,
    },
    domain::{
        config::{ProcessConfig, ProcessKind},
        process::{LogStream, ProcessStatus},
        project::ProjectRecord,
        runtime::{ProcessLogPayload, ProcessSnapshot, RunSessionSnapshot},
    },
    error::AppError,
    infrastructure::{config_loader::LoadedProjectConfig, log_store::LogStore},
};

use session::{ActiveSession, OrchestratorState};

#[derive(Clone)]
pub struct ProcessOrchestrator {
    inner: Arc<Mutex<OrchestratorState>>,
}

pub(super) struct ManagedProcess {
    config: ProcessConfig,
    snapshot: ProcessSnapshot,
    child: Option<Arc<Mutex<Child>>>,
    pid: Option<u32>,
    kill_tx: Option<mpsc::Sender<()>>,
    log_tx: broadcast::Sender<String>,
    terminating: bool,
    generation: u64,
    stop_notify_tx: Option<oneshot::Sender<()>>,
}

impl ProcessOrchestrator {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(OrchestratorState::new())),
        }
    }

    pub async fn start_session(
        &self,
        app_handle: AppHandle,
        window_key: String,
        project: ProjectRecord,
        loaded_config: LoadedProjectConfig,
    ) -> Result<RunSessionSnapshot, AppError> {
        info!(project = %project.name, "starting session");
        let snapshot = {
            let mut state = self.inner.lock().await;
            if let Some(active) = state.sessions.get(&window_key) {
                if active.snapshot.stopped_at.is_none() {
                    return Err(AppError::runtime_with_code(
                        "a project session is already running in this window",
                        crate::error::ErrorCode::ProjectAlreadyRunning,
                    ));
                }
            }

            let session = ActiveSession::new(project, loaded_config);
            let session_snapshot = (*session.snapshot).clone();
            state.sessions.insert(window_key.clone(), session);

            session_snapshot
        };

        self.emit_snapshot(&app_handle, &window_key).await?;
        self.spawn_runnable_processes(app_handle.clone(), &window_key)
            .await?;

        Ok(snapshot)
    }

    pub async fn stop_session(
        &self,
        app_handle: AppHandle,
        window_key: &str,
    ) -> Result<Option<RunSessionSnapshot>, AppError> {
        self.finish_session(app_handle, window_key, None, true)
            .await
    }

    pub async fn restart_process(
        &self,
        app_handle: AppHandle,
        window_key: &str,
        process_name: &str,
    ) -> Result<Option<RunSessionSnapshot>, AppError> {
        {
            let state = self.inner.lock().await;
            let active = state.sessions.get(window_key).ok_or_else(|| {
                AppError::runtime_with_code(
                    "no session available",
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;
            let process = active.processes.get(process_name).ok_or_else(|| {
                AppError::runtime_with_code(
                    format!("unknown process `{process_name}`"),
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;
            if matches!(process.config.kind, ProcessKind::Task) {
                return Err(AppError::runtime_with_code(
                    "cannot restart a task process",
                    crate::error::ErrorCode::ProcessCannotRestart,
                ));
            }
        }
        self.stop_process(app_handle.clone(), window_key, process_name)
            .await?;
        self.reset_process(window_key, process_name).await?;
        self.spawn_runnable_processes(app_handle.clone(), window_key)
            .await?;
        self.snapshot(window_key).await
    }

    pub async fn start_process(
        &self,
        app_handle: AppHandle,
        window_key: &str,
        process_name: &str,
    ) -> Result<Option<RunSessionSnapshot>, AppError> {
        self.reset_process(window_key, process_name).await?;
        self.spawn_runnable_processes(app_handle.clone(), window_key)
            .await?;
        self.snapshot(window_key).await
    }

    pub async fn stop_process(
        &self,
        app_handle: AppHandle,
        window_key: &str,
        process_name: &str,
    ) -> Result<Option<RunSessionSnapshot>, AppError> {
        let stop_grace = {
            let state = self.inner.lock().await;
            state
                .sessions
                .get(window_key)
                .and_then(|active| {
                    active.processes.get(&*process_name).map(|process| {
                        lifecycle::resolve_stop_timeout(
                            process.config.stop_timeout_ms,
                            active.loaded_config.config.stop_timeout_ms,
                        )
                    })
                })
                .unwrap_or_else(|| lifecycle::resolve_stop_timeout(None, None))
        };

        let (kill_tx, done_rx) = {
            let mut state = self.inner.lock().await;
            let active = state.sessions.get_mut(window_key).ok_or_else(|| {
                AppError::runtime_with_code(
                    "no session available",
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;
            let process = active.processes.get_mut(&*process_name).ok_or_else(|| {
                AppError::runtime_with_code(
                    format!("unknown process `{process_name}`"),
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;
            if matches!(process.config.kind, ProcessKind::Task) {
                return Err(AppError::runtime_with_code(
                    "cannot stop a task process",
                    crate::error::ErrorCode::ProcessCannotRestart,
                ));
            }
            let (notify_tx, notify_rx) = oneshot::channel();
            process.stop_notify_tx = Some(notify_tx);
            let kill_tx = lifecycle::begin_process_termination(process);
            ActiveSession::sync_snapshot_process(
                Arc::make_mut(&mut active.snapshot),
                &process.snapshot,
            );
            (kill_tx, notify_rx)
        };

        if let Some(kill_tx) = kill_tx {
            let _ = kill_tx.send(()).await;
        }

        let _ = tokio::time::timeout(stop_grace + Duration::from_secs(5), done_rx).await;

        self.emit_snapshot(&app_handle, window_key).await?;
        Ok(self.snapshot(window_key).await?)
    }

    pub async fn snapshot(&self, window_key: &str) -> Result<Option<RunSessionSnapshot>, AppError> {
        let state = self.inner.lock().await;
        Ok(state
            .sessions
            .get(window_key)
            .map(|active| (*active.snapshot).clone()))
    }

    /// Returns true if any window has a project session that has not stopped
    /// yet. Used to prevent an app update install from silently orphaning
    /// supervised processes in windows other than the one initiating install.
    pub async fn has_active_session(&self) -> bool {
        let state = self.inner.lock().await;
        state
            .sessions
            .values()
            .any(|active| active.snapshot.stopped_at.is_none())
    }

    async fn reset_process(&self, window_key: &str, process_name: &str) -> Result<(), AppError> {
        let mut state = self.inner.lock().await;
        let active = state.sessions.get_mut(window_key).ok_or_else(|| {
            AppError::runtime_with_code(
                "no session available",
                crate::error::ErrorCode::ProcessNotFound,
            )
        })?;
        let process = active.processes.get_mut(&*process_name).ok_or_else(|| {
            AppError::runtime_with_code(
                format!("unknown process `{process_name}`"),
                crate::error::ErrorCode::ProcessNotFound,
            )
        })?;
        lifecycle::reset_managed_process(process);
        ActiveSession::sync_snapshot_process(
            Arc::make_mut(&mut active.snapshot),
            &process.snapshot,
        );
        Ok(())
    }

    async fn spawn_runnable_processes(
        &self,
        app_handle: AppHandle,
        window_key: &str,
    ) -> Result<(), AppError> {
        let runnable = {
            let state = self.inner.lock().await;
            let Some(active) = state.sessions.get(window_key) else {
                return Ok(());
            };
            dependency::runnable_names(&active.processes, active.stop_requested)
        };

        for process_name in runnable {
            Box::pin(self.spawn_named_process(app_handle.clone(), window_key, &process_name))
                .await?;
        }

        Ok(())
    }

    async fn spawn_named_process(
        &self,
        app_handle: AppHandle,
        window_key: &str,
        process_name: &str,
    ) -> Result<(), AppError> {
        let (session_id, _, base_dir, env, config, runtime_id, log_tx, global_stop_timeout_ms) = {
            let mut state = self.inner.lock().await;
            let active = state.sessions.get_mut(window_key).ok_or_else(|| {
                AppError::runtime_with_code(
                    "no session available",
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;
            let session_id = active.snapshot.session_id.clone();
            let process = active.processes.get_mut(&*process_name).ok_or_else(|| {
                AppError::runtime_with_code(
                    format!("unknown process `{process_name}`"),
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;
            if process.child.is_some() {
                return Ok(());
            }
            let env =
                lifecycle::build_process_env(&active.loaded_config.config.env, &process.config.env);
            let global_stop_timeout_ms = active.loaded_config.config.stop_timeout_ms;
            process.snapshot.status = ProcessStatus::Starting;
            process.snapshot.started_at = Some(Utc::now());
            process.snapshot.exited_at = None;
            process.snapshot.exit_code = None;
            ActiveSession::sync_snapshot_process(
                Arc::make_mut(&mut active.snapshot),
                &process.snapshot,
            );
            (
                session_id,
                active.project.name.clone(),
                active.loaded_config.base_dir.clone(),
                env,
                process.config.clone(),
                process.snapshot.runtime_id.clone(),
                process.log_tx.clone(),
                global_stop_timeout_ms,
            )
        };

        self.emit_snapshot(&app_handle, window_key).await?;

        let spawned = spawn_process(&config.cmd, &base_dir, &env)?;
        let child_pid = spawned.child.id();
        info!(process = %process_name, pid = ?child_pid, "process started");
        let child = Arc::new(Mutex::new(spawned.child));
        let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);

        let generation = {
            let mut state = self.inner.lock().await;
            let active = state.sessions.get_mut(window_key).ok_or_else(|| {
                AppError::runtime_with_code(
                    "no session available",
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;
            if active.stop_requested {
                let _ = child.lock().await.kill().await;
                return Ok(());
            }
            let process = active.processes.get_mut(&*process_name).ok_or_else(|| {
                AppError::runtime_with_code(
                    format!("unknown process `{process_name}`"),
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;
            process.child = Some(child.clone());
            process.pid = child_pid;
            process.kill_tx = Some(kill_tx);
            process.snapshot.status = ProcessStatus::Running;
            ActiveSession::sync_snapshot_process(
                Arc::make_mut(&mut active.snapshot),
                &process.snapshot,
            );
            process.generation
        };

        self.emit_snapshot(&app_handle, window_key).await?;

        let orchestrator = self.clone();
        let wk = window_key.to_string();
        let append_fn =
            move |payload: ProcessLogPayload| -> Pin<Box<dyn Future<Output = ()> + Send>> {
                let inner = orchestrator.inner.clone();
                let wk = wk.clone();
                Box::pin(async move {
                    let mut state = inner.lock().await;
                    if let Some(active) = state.sessions.get_mut(&wk) {
                        active.logs.append(payload);
                    }
                })
            };

        log::spawn_log_task(
            app_handle.clone(),
            window_key.to_string(),
            session_id.clone(),
            process_name.to_string(),
            runtime_id.clone(),
            LogStream::Stdout,
            spawned.stdout,
            log_tx.clone(),
            append_fn,
        );

        let orchestrator = self.clone();
        let wk = window_key.to_string();
        let append_fn =
            move |payload: ProcessLogPayload| -> Pin<Box<dyn Future<Output = ()> + Send>> {
                let inner = orchestrator.inner.clone();
                let wk = wk.clone();
                Box::pin(async move {
                    let mut state = inner.lock().await;
                    if let Some(active) = state.sessions.get_mut(&wk) {
                        active.logs.append(payload);
                    }
                })
            };

        log::spawn_log_task(
            app_handle.clone(),
            window_key.to_string(),
            session_id.clone(),
            process_name.to_string(),
            runtime_id.clone(),
            LogStream::Stderr,
            spawned.stderr,
            log_tx.clone(),
            append_fn,
        );

        if matches!(config.kind, ProcessKind::Service) {
            if let Some(ready) = config.ready.clone() {
                let orchestrator = self.clone();
                let env_for_ready = env.clone();
                let base_dir_for_ready = base_dir.clone();
                let process_name_for_ready = process_name.to_string();
                let readiness_app_handle = app_handle.clone();
                let readiness_window_key = window_key.to_string();
                thread::spawn(move || {
                    tauri::async_runtime::block_on(async move {
                        let result = wait_until_ready(
                            &ready,
                            &base_dir_for_ready,
                            &env_for_ready,
                            Some(log_tx.subscribe()),
                        )
                        .await;
                        match result {
                            Ok(()) => {
                                let _ = orchestrator
                                    .mark_process_ready(
                                        readiness_app_handle.clone(),
                                        &readiness_window_key,
                                        &process_name_for_ready,
                                    )
                                    .await;
                            }
                            Err(error) => {
                                warn!(process = %process_name_for_ready, "readiness timeout for {}", process_name_for_ready);
                                let _ = readiness_app_handle.emit_to(
                                    &readiness_window_key,
                                    RUNTIME_ERROR_EVENT,
                                    RuntimeEvent::RuntimeError {
                                        message: format!("process `{process_name_for_ready}` readiness failed: {error}"),
                                    },
                                );
                                let _ = orchestrator
                                    .handle_process_failure(
                                        readiness_app_handle.clone(),
                                        &readiness_window_key,
                                        &process_name_for_ready,
                                        None,
                                        error.to_string(),
                                        generation,
                                    )
                                    .await;
                            }
                        }
                    });
                });
            } else {
                self.mark_process_ready(app_handle.clone(), window_key, process_name)
                    .await?;
            }
        }

        let orchestrator = self.clone();
        let process_name_for_wait = process_name.to_string();
        let exit_app_handle = app_handle.clone();
        let exit_window_key = window_key.to_string();
        #[cfg(unix)]
        let wait_pid = child_pid;
        let stop_grace = lifecycle::resolve_stop_timeout(
            config.stop_timeout_ms,
            global_stop_timeout_ms,
        );
        #[cfg(windows)]
        let spawned_job = {
            let mut state = self.inner.lock().await;
            state
                .sessions
                .get_mut(window_key)
                .and_then(|active| active.processes.get_mut(&*process_name))
                .and_then(|p| p.job.clone())
        };
        #[cfg(windows)]
        let wait_job = spawned_job.clone();
        thread::spawn(move || {
            tauri::async_runtime::block_on(async move {
                let exit_status = {
                    let mut child = child.lock().await;
                    tokio::select! {
                        result = child.wait() => result,
                        _ = kill_rx.recv() => {
                            // Graceful signal already sent by begin_process_termination.
                            match tokio::time::timeout(stop_grace, child.wait()).await {
                                Ok(result) => result,
                                Err(_) => {
                                    #[cfg(unix)]
                                    if let Some(pid) = wait_pid {
                                        unsafe {
                                            libc::kill(-(pid as i32), libc::SIGKILL);
                                        }
                                    }
                                    #[cfg(windows)]
                                    if let Some(job) = wait_job.as_ref() {
                                        let _ = job.terminate();
                                    }
                                    let _ = child.kill().await;
                                    child.wait().await
                                }
                            }
                        }
                    }
                };
                match exit_status {
                    Ok(status) => {
                        let code = status.code();
                        let _ = orchestrator
                            .handle_process_exit(
                                exit_app_handle.clone(),
                                &exit_window_key,
                                &process_name_for_wait,
                                code,
                                status.success(),
                                generation,
                            )
                            .await;
                    }
                    Err(error) => {
                        let _ = orchestrator
                            .handle_process_failure(
                                exit_app_handle.clone(),
                                &exit_window_key,
                                &process_name_for_wait,
                                None,
                                error.to_string(),
                                generation,
                            )
                            .await;
                    }
                }
            });
        });

        Ok(())
    }

    async fn mark_process_ready(
        &self,
        app_handle: AppHandle,
        window_key: &str,
        process_name: &str,
    ) -> Result<(), AppError> {
        {
            let mut state = self.inner.lock().await;
            let active = state.sessions.get_mut(window_key).ok_or_else(|| {
                AppError::runtime_with_code(
                    "no session available",
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;
            let process = active.processes.get_mut(&*process_name).ok_or_else(|| {
                AppError::runtime_with_code(
                    format!("unknown process `{process_name}`"),
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;
            if process.terminating {
                return Ok(());
            }
            process.snapshot.status = ProcessStatus::Ready;
            ActiveSession::sync_snapshot_process(
                Arc::make_mut(&mut active.snapshot),
                &process.snapshot,
            );
        }
        self.emit_snapshot(&app_handle, window_key).await?;
        self.spawn_runnable_processes(app_handle, window_key)
            .await?;
        Ok(())
    }

    async fn handle_process_exit(
        &self,
        app_handle: AppHandle,
        window_key: &str,
        process_name: &str,
        exit_code: Option<i32>,
        success: bool,
        generation: u64,
    ) -> Result<(), AppError> {
        let (kind, terminating, notify_tx) = {
            let mut state = self.inner.lock().await;
            let active = state.sessions.get_mut(window_key).ok_or_else(|| {
                AppError::runtime_with_code(
                    "no session available",
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;
            let process = active.processes.get_mut(&*process_name).ok_or_else(|| {
                AppError::runtime_with_code(
                    format!("unknown process `{process_name}`"),
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;

            if process.generation != generation {
                return Ok(());
            }

            let notify_tx = process.stop_notify_tx.take();
            process.child = None;
            process.snapshot.exited_at = Some(Utc::now());
            process.snapshot.exit_code = exit_code;
            let terminating = process.terminating || active.stop_requested;

            if terminating {
                process.snapshot.status = ProcessStatus::Stopped;
            } else if success && matches!(process.config.kind, ProcessKind::Task) {
                process.snapshot.status = ProcessStatus::Succeeded;
            } else {
                process.snapshot.status = ProcessStatus::Failed;
            }
            ActiveSession::sync_snapshot_process(
                Arc::make_mut(&mut active.snapshot),
                &process.snapshot,
            );
            (process.config.kind.clone(), terminating, notify_tx)
        };
        info!(process = %process_name, exit_code = ?exit_code, "process exited");

        if let Some(tx) = notify_tx {
            let _ = tx.send(());
        }

        self.emit_snapshot(&app_handle, window_key).await?;

        if terminating {
            return Ok(());
        }

        if success && matches!(kind, ProcessKind::Task) {
            self.spawn_runnable_processes(app_handle, window_key)
                .await?;
            return Ok(());
        }

        self.finish_session(
            app_handle,
            window_key,
            Some(format!("process `{process_name}` exited unexpectedly")),
            false,
        )
        .await?;
        Ok(())
    }

    async fn handle_process_failure(
        &self,
        app_handle: AppHandle,
        window_key: &str,
        process_name: &str,
        exit_code: Option<i32>,
        message: String,
        generation: u64,
    ) -> Result<(), AppError> {
        error!(process = %process_name, error = %message, "process failed");
        let (terminating, notify_tx) = {
            let mut state = self.inner.lock().await;
            let active = state.sessions.get_mut(window_key).ok_or_else(|| {
                AppError::runtime_with_code(
                    "no session available",
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;
            let process = active.processes.get_mut(&*process_name).ok_or_else(|| {
                AppError::runtime_with_code(
                    format!("unknown process `{process_name}`"),
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;

            if process.generation != generation {
                return Ok(());
            }

            let notify_tx = process.stop_notify_tx.take();
            let terminating = process.terminating || active.stop_requested;
            if terminating {
                process.snapshot.status = ProcessStatus::Stopped;
            } else {
                process.snapshot.status = ProcessStatus::Failed;
            }
            process.snapshot.exited_at = Some(Utc::now());
            process.snapshot.exit_code = exit_code;
            process.child = None;
            ActiveSession::sync_snapshot_process(
                Arc::make_mut(&mut active.snapshot),
                &process.snapshot,
            );
            (terminating, notify_tx)
        };

        if let Some(tx) = notify_tx {
            let _ = tx.send(());
        }

        self.emit_snapshot(&app_handle, window_key).await?;

        if terminating {
            return Ok(());
        }

        self.finish_session(app_handle, window_key, Some(message), false)
            .await?;
        Ok(())
    }

    async fn finish_session(
        &self,
        app_handle: AppHandle,
        window_key: &str,
        failure_message: Option<String>,
        explicit_stop: bool,
    ) -> Result<Option<RunSessionSnapshot>, AppError> {
        info!("session stopped");
        let kill_txs = {
            let mut state = self.inner.lock().await;
            let Some(active) = state.sessions.get_mut(window_key) else {
                return Ok(None);
            };
            active.stop_requested = true;
            let stopped_at = Utc::now();
            Arc::make_mut(&mut active.snapshot).stopped_at = Some(stopped_at);
            let mut kill_txs = Vec::new();
            for process in active.processes.values_mut() {
                if let Some(kill_tx) = lifecycle::begin_process_termination(process) {
                    kill_txs.push(kill_tx);
                } else if explicit_stop
                    && matches!(
                        process.snapshot.status,
                        ProcessStatus::Pending | ProcessStatus::Blocked
                    )
                {
                    process.snapshot.status = ProcessStatus::Stopped;
                }
                ActiveSession::sync_snapshot_process(
                    Arc::make_mut(&mut active.snapshot),
                    &process.snapshot,
                );
            }
            kill_txs
        };

        self.emit_snapshot(&app_handle, window_key).await?;

        // Signal each wait task to kill its own child. Locking the child
        // mutexes here would deadlock against the wait tasks.
        for kill_tx in kill_txs {
            let _ = kill_tx.send(()).await;
        }

        if let Some(message) = failure_message {
            app_handle
                .emit_to(
                    window_key,
                    RUNTIME_ERROR_EVENT,
                    RuntimeEvent::RuntimeError { message },
                )
                .map_err(|error| {
                    AppError::runtime_with_code(
                        error.to_string(),
                        crate::error::ErrorCode::ProcessStartFailed,
                    )
                })?;
        }

        self.emit_snapshot(&app_handle, window_key).await?;
        self.snapshot(window_key).await
    }

    async fn emit_snapshot(
        &self,
        app_handle: &AppHandle,
        window_key: &str,
    ) -> Result<(), AppError> {
        let snapshot = self.snapshot(window_key).await?;
        app_handle
            .emit_to(
                window_key,
                SESSION_SNAPSHOT_EVENT,
                SessionStatusEvent { snapshot },
            )
            .map_err(|error| {
                AppError::runtime_with_code(
                    error.to_string(),
                    crate::error::ErrorCode::ProcessStartFailed,
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::domain::{
        config::ProcessKind,
        process::ProcessStatus,
        project::ProjectId,
        runtime::{ProcessRuntimeId, ProcessSnapshot, RunSessionId, RunSessionSnapshot},
    };

    #[test]
    fn sync_snapshot_process_replaces_matching_runtime_snapshot() {
        let runtime_id = ProcessRuntimeId::new();
        let mut session_snapshot = RunSessionSnapshot {
            session_id: RunSessionId::new(),
            project_id: ProjectId::new(),
            project_name: "demo".to_string(),
            base_dir: std::env::temp_dir(),
            started_at: Utc::now(),
            stopped_at: None,
            processes: vec![ProcessSnapshot {
                runtime_id: runtime_id.clone(),
                name: "web".to_string(),
                kind: ProcessKind::Service,
                status: ProcessStatus::Running,
                started_at: None,
                exited_at: None,
                exit_code: None,
            }],
        };
        let updated = ProcessSnapshot {
            runtime_id,
            name: "web".to_string(),
            kind: ProcessKind::Service,
            status: ProcessStatus::Ready,
            started_at: Some(Utc::now()),
            exited_at: None,
            exit_code: None,
        };

        ActiveSession::sync_snapshot_process(&mut session_snapshot, &updated);

        assert_eq!(session_snapshot.processes.len(), 1);
        assert_eq!(session_snapshot.processes[0], updated);
    }

    fn test_project(name: &str) -> ProjectRecord {
        let now = Utc::now();
        ProjectRecord {
            id: ProjectId::new(),
            name: name.to_string(),
            base_dir: std::env::temp_dir(),
            config_source: crate::domain::project::ProjectSource::ProjectFile,
            config_path: std::env::temp_dir().join("diavola.yml"),
            created_at: now,
            updated_at: now,
        }
    }

    fn test_loaded_config() -> crate::infrastructure::config_loader::LoadedProjectConfig {
        crate::infrastructure::config_loader::LoadedProjectConfig {
            config_path: std::env::temp_dir().join("diavola.yml"),
            base_dir: std::env::temp_dir(),
            config: crate::domain::config::DiavolaConfig {
                env: Default::default(),
                processes: Default::default(),
                stop_timeout_ms: None,
            },
            raw_yaml: String::new(),
        }
    }

    #[tokio::test]
    async fn has_active_session_is_false_with_no_sessions() {
        let orchestrator = ProcessOrchestrator::new();
        assert!(!orchestrator.has_active_session().await);
    }

    #[tokio::test]
    async fn has_active_session_detects_a_running_session_in_any_window() {
        let orchestrator = ProcessOrchestrator::new();
        {
            let mut state = orchestrator.inner.lock().await;
            state.sessions.insert(
                "project-other-window".to_string(),
                ActiveSession::new(test_project("other"), test_loaded_config()),
            );
        }

        assert!(orchestrator.has_active_session().await);
    }

    #[tokio::test]
    async fn has_active_session_is_false_once_the_session_is_stopped() {
        let orchestrator = ProcessOrchestrator::new();
        {
            let mut state = orchestrator.inner.lock().await;
            let mut session = ActiveSession::new(test_project("other"), test_loaded_config());
            Arc::make_mut(&mut session.snapshot).stopped_at = Some(Utc::now());
            state
                .sessions
                .insert("project-other-window".to_string(), session);
        }

        assert!(!orchestrator.has_active_session().await);
    }
}
