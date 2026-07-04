use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use tokio::sync::broadcast;

use crate::{
    domain::{
        process::ProcessStatus,
        project::ProjectRecord,
        runtime::{ProcessRuntimeId, ProcessSnapshot, RunSessionId, RunSessionSnapshot},
    },
    infrastructure::{
        config_loader::LoadedProjectConfig,
        log_store::InMemoryLogStore,
    },
};

use super::ManagedProcess;

pub(super) struct OrchestratorState {
    pub(super) sessions: HashMap<String, ActiveSession>,
}

impl OrchestratorState {
    pub(super) fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }
}

pub(super) struct ActiveSession {
    pub(super) snapshot: Arc<RunSessionSnapshot>,
    pub(super) project: ProjectRecord,
    pub(super) loaded_config: LoadedProjectConfig,
    pub(super) processes: HashMap<String, ManagedProcess>,
    pub(super) stop_requested: bool,
    pub(super) logs: InMemoryLogStore,
}

impl ActiveSession {
    pub(super) fn new(project: ProjectRecord, loaded_config: LoadedProjectConfig) -> Self {
        let session_id = RunSessionId::new();
        let started_at = Utc::now();
        let mut snapshot = RunSessionSnapshot {
            session_id: session_id.clone(),
            project_id: project.id.clone(),
            project_name: project.name.clone(),
            base_dir: project.base_dir.clone(),
            started_at,
            stopped_at: None,
            processes: Vec::new(),
        };

        let mut processes = HashMap::new();
        for (name, config) in &loaded_config.config.processes {
            let (log_tx, _) = broadcast::channel(4096);
            let process_snapshot = ProcessSnapshot {
                runtime_id: ProcessRuntimeId::new(),
                name: name.clone(),
                kind: config.kind.clone(),
                status: ProcessStatus::Pending,
                started_at: None,
                exited_at: None,
                exit_code: None,
            };
            snapshot.processes.push(process_snapshot.clone());
            processes.insert(
                name.clone(),
                ManagedProcess {
                    config: config.clone(),
                    snapshot: process_snapshot,
                    child: None,
                    pid: None,
                    kill_tx: None,
                    log_tx,
                    terminating: false,
                    generation: 0,
                    stop_notify_tx: None,
                },
            );
        }

        Self {
            snapshot: Arc::new(snapshot),
            project,
            loaded_config,
            processes,
            stop_requested: false,
            logs: InMemoryLogStore::default(),
        }
    }

    pub(super) fn sync_snapshot_process(
        session_snapshot: &mut RunSessionSnapshot,
        process_snapshot: &ProcessSnapshot,
    ) {
        if let Some(existing) = session_snapshot
            .processes
            .iter_mut()
            .find(|process| process.runtime_id == process_snapshot.runtime_id)
        {
            *existing = process_snapshot.clone();
        }
    }

}
