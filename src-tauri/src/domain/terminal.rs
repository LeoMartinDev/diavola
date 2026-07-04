use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct TerminalSessionId(pub Uuid);

impl TerminalSessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSnapshot {
    pub terminal_id: TerminalSessionId,
    pub title: String,
    pub cwd: PathBuf,
    pub created_at: DateTime<Utc>,
    pub is_open: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputPayload {
    pub terminal_id: TerminalSessionId,
    pub chunk: String,
    pub timestamp: DateTime<Utc>,
}
