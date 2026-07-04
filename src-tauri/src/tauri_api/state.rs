use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    application::orchestrator::ProcessOrchestrator,
    domain::project::ProjectRecord,
    error::AppError,
    infrastructure::{project_store::ProjectStore, pty::TerminalManager},
};

#[derive(Clone)]
pub struct AppState {
    pub orchestrator: ProcessOrchestrator,
    pub project_store: Arc<Mutex<ProjectStore>>,
    pub terminal_manager: TerminalManager,
    pub current_project: Arc<Mutex<Option<ProjectRecord>>>,
    pub launch_auto_run: Arc<Mutex<bool>>,
    pub launch_error: Arc<Mutex<Option<String>>>,
}

impl AppState {
    pub fn new() -> Result<Self, AppError> {
        Ok(Self {
            orchestrator: ProcessOrchestrator::new(),
            project_store: Arc::new(Mutex::new(ProjectStore::new()?)),
            terminal_manager: TerminalManager::new(),
            current_project: Arc::new(Mutex::new(None)),
            launch_auto_run: Arc::new(Mutex::new(false)),
            launch_error: Arc::new(Mutex::new(None)),
        })
    }
}
