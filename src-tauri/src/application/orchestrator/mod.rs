pub mod dependency;
pub mod lifecycle;
pub mod log;
pub mod session;

use std::{future::Future, pin::Pin, sync::Arc, thread, time::Duration};

use chrono::Utc;
use tauri::{AppHandle, Emitter};
use tokio::{
    process::Child,
    sync::{broadcast, mpsc, Mutex},
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
        runtime::{ProcessLogPayload, ProcessRuntimeId, ProcessSnapshot, RunSessionSnapshot},
    },
    error::AppError,
    infrastructure::config_loader::LoadedProjectConfig,
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
    #[cfg(windows)]
    pub(super) job: Option<Arc<crate::infrastructure::job::Job>>,
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

    pub async fn force_stop_session(
        &self,
        app_handle: AppHandle,
        window_key: &str,
    ) -> Result<Option<RunSessionSnapshot>, AppError> {
        self.finish_session(app_handle, window_key, None, true)
            .await
    }

    pub async fn stop_all_sessions(&self, app_handle: AppHandle) -> Result<(), AppError> {
        let keys: Vec<String> = {
            let state = self.inner.lock().await;
            state.sessions.keys().cloned().collect()
        };
        for key in keys {
            self.finish_session(app_handle.clone(), &key, None, true)
                .await?;
        }
        Ok(())
    }

    pub async fn force_stop_all_sessions(&self, app_handle: AppHandle) -> Result<(), AppError> {
        let keys: Vec<String> = {
            let state = self.inner.lock().await;
            state.sessions.keys().cloned().collect()
        };
        for key in keys {
            self.finish_session(app_handle.clone(), &key, None, true)
                .await?;
        }
        Ok(())
    }

    pub async fn restart_process(
        &self,
        app_handle: AppHandle,
        window_key: &str,
        process_name: &str,
    ) -> Result<Option<RunSessionSnapshot>, AppError> {
        let kill_tx = {
            let mut state = self.inner.lock().await;
            let active = state.sessions.get_mut(window_key).ok_or_else(|| {
                AppError::runtime_with_code(
                    "no session available",
                    crate::error::ErrorCode::ProcessNotFound,
                )
            })?;
            let process = active.processes.get_mut(process_name).ok_or_else(|| {
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
            if active.stop_requested {
                let snapshot = (*active.snapshot).clone();
                return Ok(Some(snapshot));
            }
            let kill_tx = lifecycle::begin_process_termination(process);
            lifecycle::reset_managed_process(process);
            ActiveSession::sync_snapshot_process(
                Arc::make_mut(&mut active.snapshot),
                &process.snapshot,
            );
            kill_tx
        };

        if let Some(kill_tx) = kill_tx {
            let _ = kill_tx.send(()).await;
        }

        self.emit_snapshot(&app_handle, window_key).await?;
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
            if matches!(process.config.kind, ProcessKind::Task) {
                return Err(AppError::runtime_with_code(
                    "cannot start a task process",
                    crate::error::ErrorCode::ProcessCannotRestart,
                ));
            }
            if process.child.is_some()
                || matches!(
                    process.snapshot.status,
                    ProcessStatus::Starting
                        | ProcessStatus::Running
                        | ProcessStatus::Ready
                        | ProcessStatus::Stopping
                )
            {
                let snapshot = (*active.snapshot).clone();
                return Ok(Some(snapshot));
            }
        }
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
        info!(process = %process_name, window = %window_key, "stop_process: entered");
        let kill_tx = {
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
            let kill_tx = lifecycle::begin_process_termination(process);
            info!(
                process = %process_name,
                status = ?process.snapshot.status,
                kill_tx_taken = kill_tx.is_some(),
                "stop_process: begin_process_termination done"
            );
            ActiveSession::sync_snapshot_process(
                Arc::make_mut(&mut active.snapshot),
                &process.snapshot,
            );
            kill_tx
        };

        if let Some(kill_tx) = kill_tx {
            let _ = kill_tx.send(()).await;
        }

        self.emit_snapshot(&app_handle, window_key).await?;

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let pname = process_name.to_string();
        loop {
            let snapshot = match self.snapshot(window_key).await? {
                Some(s) => s,
                None => return Ok(None),
            };
            let reached_terminal = snapshot.processes.iter().any(|p| {
                p.name == pname
                    && matches!(
                        p.status,
                        ProcessStatus::Stopped
                            | ProcessStatus::Failed
                            | ProcessStatus::Succeeded
                    )
            });
            if reached_terminal {
                info!(process = %process_name, "stop_process: process reached terminal state, returning");
                return Ok(Some(snapshot));
            }
            if std::time::Instant::now() > deadline {
                warn!(
                    process = %process_name,
                    "stop_process: process did not reach terminal state within 15s deadline"
                );
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        Ok(self.snapshot(window_key).await?)
    }

    pub async fn snapshot(&self, window_key: &str) -> Result<Option<RunSessionSnapshot>, AppError> {
        let state = self.inner.lock().await;
        Ok(state
            .sessions
            .get(window_key)
            .map(|active| (*active.snapshot).clone()))
    }

    pub async fn search_logs(
        &self,
        window_key: &str,
        runtime_id: &ProcessRuntimeId,
        query: &str,
        regex: bool,
        case_sensitive: bool,
        up_to: usize,
    ) -> Result<crate::infrastructure::log_store::SearchMatches, AppError> {
        let state = self.inner.lock().await;
        let Some(active) = state.sessions.get(window_key) else {
            return Ok(crate::infrastructure::log_store::SearchMatches::default());
        };
        active.logs.search(runtime_id, query, regex, case_sensitive, up_to)
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
        let (session_id, _, base_dir, env, config, runtime_id, log_tx, global_log_entry_pattern) = {
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
            let global_log_entry_pattern =
                active.loaded_config.config.log_entry_pattern.clone();
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
                global_log_entry_pattern,
            )
        };

        let entry_pattern = config
            .log_entry_pattern
            .as_deref()
            .or(global_log_entry_pattern.as_deref())
            .and_then(|p| {
                regex::Regex::new(p)
                    .map_err(|e| {
                        tracing::warn!(
                            process = %process_name,
                            pattern = %p,
                            error = %e,
                            "invalid logEntryPattern, falling back to single-line mode"
                        );
                        e
                    })
                    .ok()
            });

        self.emit_snapshot(&app_handle, window_key).await?;

        let spawned = spawn_process(&config.cmd, &base_dir, &env)?;
        #[cfg(windows)]
        let spawned_job = spawned.job;
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
            #[cfg(windows)]
            {
                process.job = spawned_job;
            }
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

        let readiness_rx = if matches!(config.kind, ProcessKind::Service)
            && config.ready.is_some()
        {
            Some(log_tx.subscribe())
        } else {
            None
        };

        let orchestrator = self.clone();
        let wk = window_key.to_string();
        let append_fn =
            move |payload: ProcessLogPayload| -> Pin<Box<dyn Future<Output = ()> + Send>> {
                let inner = orchestrator.inner.clone();
                let wk = wk.clone();
                Box::pin(async move {
                    let mut state = inner.lock().await;
                    if let Some(active) = state.sessions.get_mut(&wk) {
                        active.logs.append(&payload);
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
            entry_pattern.clone(),
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
                        active.logs.append(&payload);
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
            entry_pattern.clone(),
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
                            readiness_rx,
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
        thread::spawn(move || {
            tauri::async_runtime::block_on(async move {
                let exit_status = {
                    let mut child = child.lock().await;
                    tokio::select! {
                        result = child.wait() => {
                            info!(process = %process_name_for_wait, "wait_task: child.wait() branch won");
                            result
                        }
                _ = kill_rx.recv() => {
                    info!(process = %process_name_for_wait, "wait_task: kill_rx.recv() branch won");
                    let _ = child.kill().await;
                    match tokio::time::timeout(Duration::from_secs(10), child.wait()).await {
                        Ok(result) => {
                            info!(process = %process_name_for_wait, "wait_task: child.wait() after kill completed");
                            result
                        }
                        Err(_) => {
                            warn!(
                                process = %process_name_for_wait,
                                "process did not exit within kill timeout, forcing stop"
                            );
                            Err(std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "process did not exit within kill timeout",
                            ))
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
        let (kind, terminating) = {
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
                info!(
                    process = %process_name,
                    stored_generation = process.generation,
                    wait_generation = generation,
                    "handle_process_exit: generation mismatch, returning early"
                );
                return Ok(());
            }

            process.child = None;
            process.snapshot.exited_at = Some(Utc::now());
            process.snapshot.exit_code = exit_code;
            let terminating = process.terminating || active.stop_requested;

            info!(
                process = %process_name,
                terminating,
                stop_requested = active.stop_requested,
                success,
                exit_code = ?exit_code,
                "handle_process_exit: setting status"
            );

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
            (process.config.kind.clone(), terminating)
        };
        info!(process = %process_name, exit_code = ?exit_code, "process exited");

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
        let terminating = {
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
            terminating
        };

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

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            let snapshot = match self.snapshot(window_key).await? {
                Some(s) => s,
                None => return Ok(None),
            };
            let all_terminal = snapshot.processes.iter().all(|p| {
                matches!(
                    p.status,
                    ProcessStatus::Stopped
                        | ProcessStatus::Failed
                        | ProcessStatus::Succeeded
                        | ProcessStatus::Pending
                        | ProcessStatus::Blocked
                )
            });
            if all_terminal {
                return Ok(Some(snapshot));
            }
            if std::time::Instant::now() > deadline {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
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
                log_entry_pattern: None,
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
