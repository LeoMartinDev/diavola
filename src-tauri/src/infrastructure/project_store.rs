use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::{
    domain::project::{ProjectId, ProjectRecord, ProjectSource},
    error::AppError,
    infrastructure::config_loader::{find_project_config, load_config},
};

const PROJECTS_FILE_NAME: &str = "projects.yml";
const PROJECT_CONFIGS_DIR_NAME: &str = "project-configs";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct StoredProjects {
    #[serde(default)]
    projects: Vec<ProjectRecord>,
}

#[derive(Debug, Clone)]
pub struct ProjectStore {
    root_dir: PathBuf,
}

impl ProjectStore {
    pub fn new() -> Result<Self, AppError> {
        let project_dirs = ProjectDirs::from("", "", "trame")
            .ok_or_else(|| AppError::project_store("unable to resolve OS config directory"))?;
        let root_dir = project_dirs.config_dir().to_path_buf();
        fs::create_dir_all(root_dir.join(PROJECT_CONFIGS_DIR_NAME))?;
        Ok(Self { root_dir })
    }

    #[cfg(test)]
    fn from_root_dir(root_dir: PathBuf) -> Result<Self, AppError> {
        fs::create_dir_all(root_dir.join(PROJECT_CONFIGS_DIR_NAME))?;
        Ok(Self { root_dir })
    }

    pub fn load(&self) -> Result<Vec<ProjectRecord>, AppError> {
        let path = self.projects_file_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let raw_yaml = fs::read_to_string(&path)?;
        let mut stored: StoredProjects = serde_yaml::from_str(&raw_yaml)?;
        stored
            .projects
            .sort_by(|left, right| left.name.cmp(&right.name));
        Ok(stored.projects)
    }

    pub fn save(&self, projects: &[ProjectRecord]) -> Result<(), AppError> {
        fs::create_dir_all(&self.root_dir)?;
        let payload = StoredProjects {
            projects: projects.to_vec(),
        };
        let yaml = serde_yaml::to_string(&payload)
            .map_err(|error| AppError::project_store(error.to_string()))?;
        fs::write(self.projects_file_path(), yaml)?;
        Ok(())
    }

    pub fn upsert(&self, record: ProjectRecord) -> Result<ProjectRecord, AppError> {
        let mut projects = self.load()?;
        if let Some(existing) = projects.iter_mut().find(|project| project.id == record.id) {
            *existing = record.clone();
        } else {
            projects.push(record.clone());
        }
        self.save(&projects)?;
        Ok(record)
    }

    pub fn remove(&self, project_id: &ProjectId) -> Result<(), AppError> {
        let mut projects = self.load()?;
        if let Some(index) = projects
            .iter()
            .position(|project| &project.id == project_id)
        {
            let project = projects.remove(index);
            if matches!(project.config_source, ProjectSource::AppConfigFile)
                && project.config_path.starts_with(self.project_configs_dir())
                && project.config_path.exists()
            {
                fs::remove_file(project.config_path)?;
            }
        }
        self.save(&projects)?;
        Ok(())
    }

    pub fn get(&self, project_id: &ProjectId) -> Result<Option<ProjectRecord>, AppError> {
        Ok(self
            .load()?
            .into_iter()
            .find(|project| &project.id == project_id))
    }

    pub fn import_project_config_path(
        &self,
        config_path: PathBuf,
    ) -> Result<ProjectRecord, AppError> {
        let loaded = load_config(&config_path)?;
        let canonical_config_path = loaded.config_path;
        let base_dir = loaded.base_dir;
        let name = base_dir
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("trame project")
            .to_string();

        let existing = self.load()?.into_iter().find(|project| {
            project.config_path == canonical_config_path || project.base_dir == base_dir
        });
        let now = Utc::now();
        let record = ProjectRecord {
            id: existing
                .as_ref()
                .map(|project| project.id.clone())
                .unwrap_or_else(ProjectId::new),
            name: existing
                .as_ref()
                .map(|project| project.name.clone())
                .unwrap_or(name),
            base_dir,
            config_source: ProjectSource::ProjectFile,
            config_path: canonical_config_path,
            created_at: existing
                .as_ref()
                .map(|project| project.created_at)
                .unwrap_or(now),
            updated_at: now,
        };

        self.upsert(record)
    }

    pub fn prepare_project_record_from_config_path(
        &self,
        config_path: PathBuf,
    ) -> Result<ProjectRecord, AppError> {
        let canonical_config_path = fs::canonicalize(&config_path).map_err(|error| {
            AppError::project_store(format!(
                "unable to resolve config path {}: {error}",
                config_path.display()
            ))
        })?;
        let base_dir = canonical_config_path
            .parent()
            .ok_or_else(|| AppError::project_store("config file must have a parent directory"))?
            .to_path_buf();
        let name = base_dir
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("trame project")
            .to_string();
        let now = Utc::now();

        Ok(ProjectRecord {
            id: ProjectId::new(),
            name,
            base_dir,
            config_source: ProjectSource::ProjectFile,
            config_path: canonical_config_path,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn load_project_config_raw(&self, project: &ProjectRecord) -> Result<String, AppError> {
        fs::read_to_string(&project.config_path).map_err(AppError::from)
    }

    pub fn load_project_config_raw_optional(
        &self,
        project: &ProjectRecord,
    ) -> Result<Option<String>, AppError> {
        if !project.config_path.exists() {
            return Ok(None);
        }
        self.load_project_config_raw(project).map(Some)
    }

    pub fn save_project_config_raw(
        &self,
        project: &ProjectRecord,
        raw_yaml: &str,
    ) -> Result<(), AppError> {
        if let Some(parent) = project.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let yaml = if matches!(project.config_source, ProjectSource::AppConfigFile) {
            yaml_with_base_dir_reference(raw_yaml, &project.base_dir)?
        } else {
            raw_yaml.to_string()
        };
        fs::write(&project.config_path, yaml)?;
        Ok(())
    }

    pub fn prepare_project_record(
        &self,
        existing_id: Option<ProjectId>,
        name: String,
        base_dir: PathBuf,
        requested_source: Option<ProjectSource>,
    ) -> Result<ProjectRecord, AppError> {
        let canonical_base_dir = fs::canonicalize(&base_dir).map_err(|error| {
            AppError::project_store(format!(
                "unable to resolve project directory {}: {error}",
                base_dir.display()
            ))
        })?;

        let project_id = existing_id.unwrap_or_else(ProjectId::new);
        let now = Utc::now();
        let existing = self.get(&project_id)?;

        let detected_project_file = find_project_config(&canonical_base_dir)?;
        let config_source = match (requested_source, detected_project_file.as_ref()) {
            (Some(ProjectSource::ProjectFile), Some(_)) => ProjectSource::ProjectFile,
            (Some(ProjectSource::ProjectFile), None) => ProjectSource::ProjectFile,
            (Some(ProjectSource::AppConfigFile), _) => ProjectSource::AppConfigFile,
            (None, Some(_)) => ProjectSource::ProjectFile,
            (None, None) => ProjectSource::AppConfigFile,
        };

        let config_path = match config_source {
            ProjectSource::ProjectFile => {
                detected_project_file.unwrap_or_else(|| canonical_base_dir.join("trame.yml"))
            }
            ProjectSource::AppConfigFile => self.app_config_path(&project_id),
        };

        Ok(ProjectRecord {
            id: project_id,
            name,
            base_dir: canonical_base_dir,
            config_source,
            config_path,
            created_at: existing
                .as_ref()
                .map(|project| project.created_at)
                .unwrap_or(now),
            updated_at: now,
        })
    }

    pub fn prepare_workspace_project(&self, base_dir: PathBuf) -> Result<ProjectRecord, AppError> {
        let canonical_base_dir = fs::canonicalize(&base_dir).map_err(|error| {
            AppError::project_store(format!(
                "unable to resolve project directory {}: {error}",
                base_dir.display()
            ))
        })?;
        let name = canonical_base_dir
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("trame")
            .to_string();

        self.prepare_project_record(
            None,
            name,
            canonical_base_dir,
            Some(ProjectSource::ProjectFile),
        )
    }

    pub fn app_config_path(&self, project_id: &ProjectId) -> PathBuf {
        self.project_configs_dir()
            .join(format!("{}.yml", project_id.0))
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    fn projects_file_path(&self) -> PathBuf {
        self.root_dir.join(PROJECTS_FILE_NAME)
    }

    fn project_configs_dir(&self) -> PathBuf {
        self.root_dir.join(PROJECT_CONFIGS_DIR_NAME)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn temp_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4()))
    }

    fn test_store() -> (ProjectStore, PathBuf) {
        let root = temp_dir("trame-project-store");
        let store = ProjectStore::from_root_dir(root.clone()).expect("create project store");
        (store, root)
    }

    fn create_project_dir(with_project_file: bool) -> PathBuf {
        let base_dir = temp_dir("trame-project");
        fs::create_dir_all(&base_dir).expect("create project dir");
        if with_project_file {
            fs::write(
                base_dir.join("trame.yml"),
                "processes:\n  web:\n    kind: service\n    cmd: deno task dev\n",
            )
            .expect("write project config");
        }
        base_dir
    }

    #[test]
    fn saves_and_loads_project_records_round_trip_sorted_by_name() {
        let (store, root) = test_store();
        let first = ProjectRecord {
            id: ProjectId::new(),
            name: "zeta".to_string(),
            base_dir: create_project_dir(false),
            config_source: ProjectSource::AppConfigFile,
            config_path: root.join("project-configs/zeta.yml"),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let second = ProjectRecord {
            id: ProjectId::new(),
            name: "alpha".to_string(),
            base_dir: create_project_dir(false),
            config_source: ProjectSource::ProjectFile,
            config_path: root.join("alpha/trame.yml"),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        store
            .save(&[first.clone(), second.clone()])
            .expect("save records");

        let loaded = store.load().expect("load records");
        assert_eq!(loaded, vec![second, first]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepares_project_file_record_and_edits_trame_yml() {
        let (store, root) = test_store();
        let base_dir = create_project_dir(true);

        let record = store
            .prepare_project_record(
                None,
                "with project file".to_string(),
                base_dir.clone(),
                None,
            )
            .expect("prepare record");

        assert_eq!(record.config_source, ProjectSource::ProjectFile);
        assert_eq!(record.config_path, base_dir.join("trame.yml"));

        store
            .save_project_config_raw(
                &record,
                "processes:\n  api:\n    kind: service\n    cmd: cargo run\n",
            )
            .expect("edit project config");

        assert!(fs::read_to_string(base_dir.join("trame.yml"))
            .expect("read project config")
            .contains("cargo run"));
        assert!(!store.app_config_path(&record.id).exists());

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn prepares_app_config_record_and_edits_project_configs() {
        let (store, root) = test_store();
        let base_dir = create_project_dir(false);

        let record = store
            .prepare_project_record(
                None,
                "without project file".to_string(),
                base_dir.clone(),
                None,
            )
            .expect("prepare record");

        assert_eq!(record.config_source, ProjectSource::AppConfigFile);
        assert_eq!(record.config_path, store.app_config_path(&record.id));
        assert!(record.config_path.starts_with(store.project_configs_dir()));

        store
            .save_project_config_raw(
                &record,
                "processes:\n  web:\n    kind: service\n    cmd: deno task dev\n",
            )
            .expect("write app config");

        assert!(record.config_path.exists());
        let saved_yaml = fs::read_to_string(&record.config_path).expect("read app config");
        assert!(saved_yaml.contains("baseDir:"));
        assert!(saved_yaml.contains(&record.base_dir.display().to_string()));
        assert!(!base_dir.join("trame.yml").exists());

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn removes_owned_app_config_file_with_record() {
        let (store, root) = test_store();
        let base_dir = create_project_dir(false);
        let record = store
            .prepare_project_record(None, "app config".to_string(), base_dir.clone(), None)
            .expect("prepare record");
        store
            .save_project_config_raw(
                &record,
                "processes:\n  web:\n    kind: service\n    cmd: deno task dev\n",
            )
            .expect("write app config");
        store.upsert(record.clone()).expect("save record");

        store.remove(&record.id).expect("remove project");

        assert!(!record.config_path.exists());
        assert!(store.load().expect("load after remove").is_empty());

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn prepares_workspace_project_for_missing_trame_yml() {
        let (store, root) = test_store();
        let base_dir = create_project_dir(false);

        let record = store
            .prepare_workspace_project(base_dir.clone())
            .expect("prepare workspace project");

        assert_eq!(record.base_dir, base_dir);
        assert_eq!(record.config_path, base_dir.join("trame.yml"));

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn returns_none_when_workspace_config_does_not_exist() {
        let (store, root) = test_store();
        let base_dir = create_project_dir(false);
        let record = store
            .prepare_workspace_project(base_dir.clone())
            .expect("prepare workspace project");

        assert_eq!(
            store
                .load_project_config_raw_optional(&record)
                .expect("load optional config"),
            None
        );

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn prepares_project_record_from_explicit_config_path() {
        let (store, root) = test_store();
        let base_dir = create_project_dir(false);
        let config_path = base_dir.join("custom-trame.yml");
        fs::write(
            &config_path,
            "processes:\n  web:\n    kind: service\n    cmd: deno task dev\n",
        )
        .expect("write explicit config");

        let record = store
            .prepare_project_record_from_config_path(config_path.clone())
            .expect("prepare explicit config record");

        assert_eq!(record.base_dir, base_dir);
        assert_eq!(record.config_path, config_path);
        assert_eq!(record.config_source, ProjectSource::ProjectFile);

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(base_dir);
    }
}

fn yaml_with_base_dir_reference(raw_yaml: &str, base_dir: &Path) -> Result<String, AppError> {
    let mut value: serde_yaml::Value = serde_yaml::from_str(raw_yaml)?;
    let mapping = value.as_mapping_mut().ok_or_else(|| {
        AppError::validation_with_code(
            "configuration root must be a YAML mapping",
            crate::error::ErrorCode::ConfigValidationFailed,
        )
    })?;
    mapping.insert(
        serde_yaml::Value::String("baseDir".to_string()),
        serde_yaml::Value::String(base_dir.display().to_string()),
    );
    serde_yaml::to_string(&value).map_err(|error| AppError::project_store(error.to_string()))
}
