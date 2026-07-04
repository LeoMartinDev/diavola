use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{
    config::ProcessKind,
    process::{LogStream, ProcessStatus},
    project::ProjectId,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct RunSessionId(pub Uuid);

impl RunSessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ProcessRuntimeId(pub Uuid);

impl ProcessRuntimeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunSessionSnapshot {
    pub session_id: RunSessionId,
    pub project_id: ProjectId,
    pub project_name: String,
    pub base_dir: PathBuf,
    pub started_at: DateTime<Utc>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub processes: Vec<ProcessSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSnapshot {
    pub runtime_id: ProcessRuntimeId,
    pub name: String,
    pub kind: ProcessKind,
    pub status: ProcessStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub exited_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessLogPayload {
    pub session_id: RunSessionId,
    pub runtime_id: ProcessRuntimeId,
    pub process_name: String,
    pub stream: LogStream,
    pub line: String,
    pub timestamp: DateTime<Utc>,
}
