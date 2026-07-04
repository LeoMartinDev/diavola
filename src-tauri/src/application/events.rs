use serde::{Deserialize, Serialize};

use crate::domain::{
    runtime::{ProcessLogPayload, RunSessionSnapshot},
    terminal::{TerminalOutputPayload, TerminalSnapshot},
};

pub const SESSION_SNAPSHOT_EVENT: &str = "session-snapshot";
pub const PROCESS_LOG_EVENT: &str = "process-log";
pub const TERMINAL_OUTPUT_EVENT: &str = "terminal-output";
pub const TERMINAL_SNAPSHOT_EVENT: &str = "terminal-snapshot";
pub const RUNTIME_ERROR_EVENT: &str = "runtime-error";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RuntimeEvent {
    SessionSnapshot {
        snapshot: Option<RunSessionSnapshot>,
    },
    RuntimeError {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessLogEvent {
    pub payload: ProcessLogPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatusEvent {
    pub snapshot: Option<RunSessionSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalEvent {
    pub snapshot: TerminalSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputEvent {
    pub payload: TerminalOutputPayload,
}
