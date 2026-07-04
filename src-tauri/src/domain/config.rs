use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrameConfig {
    #[serde(default)]
    pub env: IndexMap<String, String>,
    pub processes: IndexMap<String, ProcessConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessConfig {
    pub kind: ProcessKind,
    pub cmd: String,
    #[serde(default)]
    pub env: IndexMap<String, String>,
    #[serde(default, rename = "dependsOn")]
    pub depends_on: IndexMap<String, DependencyCondition>,
    #[serde(default)]
    pub ready: Option<ReadyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProcessKind {
    Task,
    Service,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyCondition {
    Success,
    Ready,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ReadyConfig {
    Http(HttpReadyConfig),
    Log(LogReadyConfig),
    Delay(DelayReadyConfig),
    Command(CommandReadyConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HttpReadyConfig {
    pub url: String,
    #[serde(default)]
    pub interval_ms: Option<u64>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LogReadyConfig {
    pub pattern: String,
    #[serde(default)]
    pub regex: bool,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DelayReadyConfig {
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandReadyConfig {
    pub cmd: String,
    #[serde(default)]
    pub interval_ms: Option<u64>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}
