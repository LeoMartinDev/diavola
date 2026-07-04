use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ProjectId(pub Uuid);

impl ProjectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProjectSource {
    ProjectFile,
    AppConfigFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: ProjectId,
    pub name: String,
    pub base_dir: PathBuf,
    pub config_source: ProjectSource,
    pub config_path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
