use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProcessStatus {
    Pending,
    Blocked,
    Starting,
    Running,
    Ready,
    Succeeded,
    Failed,
    Stopping,
    Stopped,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogStream {
    Stdout,
    Stderr,
    System,
}
