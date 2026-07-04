use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    domain::config::{ReadyConfig, DiavolaConfig},
    error::{AppError, ErrorCode},
};

const PROJECT_CONFIG_FILE_NAME: &str = "diavola.yml";

#[derive(Debug, Clone)]
pub struct LoadedProjectConfig {
    pub config_path: PathBuf,
    pub base_dir: PathBuf,
    pub config: DiavolaConfig,
    pub raw_yaml: String,
}

pub fn load_config(config_path: &Path) -> Result<LoadedProjectConfig, AppError> {
    let canonical_path = fs::canonicalize(config_path).map_err(|error| {
        AppError::config(format!(
            "unable to resolve config path {}: {error}",
            config_path.display()
        ))
    })?;
    let raw_yaml = fs::read_to_string(&canonical_path)?;
    let config: DiavolaConfig = serde_yaml::from_str(&raw_yaml)?;
    validate_graph(&config)?;

    let base_dir = canonical_path
        .parent()
        .ok_or_else(|| AppError::config("config file must have a parent directory"))?
        .to_path_buf();

    Ok(LoadedProjectConfig {
        config_path: canonical_path,
        base_dir,
        config,
        raw_yaml,
    })
}

pub async fn load_config_async(config_path: &Path) -> Result<LoadedProjectConfig, AppError> {
    let path = config_path.to_path_buf();
    tokio::task::spawn_blocking(move || load_config(&path))
        .await
        .map_err(|e| AppError::runtime_with_code(e.to_string(), ErrorCode::IoError))?
}

pub fn parse_config_document(
    config_path: &Path,
    raw_yaml: &str,
) -> Result<LoadedProjectConfig, AppError> {
    let config: DiavolaConfig = serde_yaml::from_str(raw_yaml)?;
    validate_graph(&config)?;
    let canonical_path = canonicalize_for_write_target(config_path)?;
    let base_dir = canonical_path
        .parent()
        .ok_or_else(|| AppError::config("config file must have a parent directory"))?
        .to_path_buf();

    Ok(LoadedProjectConfig {
        config_path: canonical_path,
        base_dir,
        config,
        raw_yaml: raw_yaml.to_string(),
    })
}

pub fn find_project_config(base_dir: &Path) -> Result<Option<PathBuf>, AppError> {
    let canonical_base_dir = fs::canonicalize(base_dir).map_err(|error| {
        AppError::config(format!(
            "unable to resolve project directory {}: {error}",
            base_dir.display()
        ))
    })?;
    let config_path = canonical_base_dir.join(PROJECT_CONFIG_FILE_NAME);
    if config_path.is_file() {
        Ok(Some(config_path))
    } else {
        Ok(None)
    }
}

fn find_config_in_dir_or_parents(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();
    loop {
        let candidate = current.join(PROJECT_CONFIG_FILE_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Walk up from the current working directory to the filesystem root
/// looking for `diavola.yml`. Returns the first match, or `None`.
pub fn find_config_in_cwd_or_parents() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    find_config_in_dir_or_parents(&current_dir)
}

pub fn validate_graph(config: &DiavolaConfig) -> Result<(), AppError> {
    if config.processes.is_empty() {
        return Err(AppError::validation_with_code(
            "configuration must declare at least one process",
            crate::error::ErrorCode::ConfigValidationFailed,
        ));
    }

    for (name, process) in &config.processes {
        if name.trim().is_empty() {
            return Err(AppError::validation_with_code(
                "process name cannot be empty",
                crate::error::ErrorCode::ConfigValidationFailed,
            ));
        }
        if process.cmd.trim().is_empty() {
            return Err(AppError::validation_with_code(
                format!("process `{name}` must declare a non-empty cmd"),
                crate::error::ErrorCode::ConfigValidationFailed,
            ));
        }
        if let Some(ready) = &process.ready {
            validate_ready_config(name, ready)?;
        }
        for dependency_name in process.depends_on.keys() {
            if !config.processes.contains_key(dependency_name) {
                return Err(AppError::validation_with_code(
                    format!("process `{name}` depends on unknown process `{dependency_name}`"),
                    crate::error::ErrorCode::ConfigUnknownProcess,
                ));
            }
        }
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for name in config.processes.keys() {
        visit_process(name, config, &mut visiting, &mut visited)?;
    }

    Ok(())
}

fn validate_ready_config(process_name: &str, ready: &ReadyConfig) -> Result<(), AppError> {
    match ready {
        ReadyConfig::Http(config) => {
            let url = reqwest::Url::parse(&config.url).map_err(|error| {
                AppError::validation_with_code(
                    format!(
                        "process `{process_name}` has invalid http readiness URL `{}`: {error}",
                        config.url
                    ),
                    crate::error::ErrorCode::ConfigValidationFailed,
                )
            })?;
            if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
                return Err(AppError::validation_with_code(
                    format!(
                        "process `{process_name}` http readiness URL must be an absolute http(s) URL"
                    ),
                    crate::error::ErrorCode::ConfigValidationFailed,
                ));
            }
        }
        ReadyConfig::Log(config) if config.pattern.trim().is_empty() => {
            return Err(AppError::validation_with_code(
                format!("process `{process_name}` log readiness pattern cannot be empty"),
                crate::error::ErrorCode::ConfigValidationFailed,
            ));
        }
        ReadyConfig::Command(config) if config.cmd.trim().is_empty() => {
            return Err(AppError::validation_with_code(
                format!("process `{process_name}` command readiness cmd cannot be empty"),
                crate::error::ErrorCode::ConfigValidationFailed,
            ));
        }
        ReadyConfig::Delay(_) | ReadyConfig::Log(_) | ReadyConfig::Command(_) => {}
    }

    Ok(())
}

fn visit_process(
    name: &str,
    config: &DiavolaConfig,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
) -> Result<(), AppError> {
    if visited.contains(name) {
        return Ok(());
    }
    if !visiting.insert(name.to_string()) {
        return Err(AppError::validation_with_code(
            format!("dependency cycle detected at process `{name}`"),
            crate::error::ErrorCode::ConfigDependencyCycle,
        ));
    }

    let process = config.processes.get(name).ok_or_else(|| {
        AppError::validation_with_code(
            format!("unknown process `{name}`"),
            crate::error::ErrorCode::ConfigUnknownProcess,
        )
    })?;
    for dependency_name in process.depends_on.keys() {
        visit_process(dependency_name, config, visiting, visited)?;
    }

    visiting.remove(name);
    visited.insert(name.to_string());
    Ok(())
}

fn canonicalize_for_write_target(path: &Path) -> Result<PathBuf, AppError> {
    if path.exists() {
        return fs::canonicalize(path).map_err(AppError::from);
    }

    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    let parent = absolute_path
        .parent()
        .ok_or_else(|| AppError::config("config path must have a parent directory"))?;
    let canonical_parent = fs::canonicalize(parent).map_err(|error| {
        AppError::config(format!(
            "unable to resolve config parent directory {}: {error}",
            parent.display()
        ))
    })?;
    let file_name = absolute_path
        .file_name()
        .ok_or_else(|| AppError::config("config path must include a file name"))?;

    Ok(canonical_parent.join(file_name))
}

pub fn serialize_config(config: &DiavolaConfig) -> Result<String, AppError> {
    serde_yaml::to_string(config).map_err(|error| AppError::config(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    fn parse(raw_yaml: &str) -> Result<LoadedProjectConfig, AppError> {
        let base_dir = std::env::temp_dir().join("diavola-config-loader");
        fs::create_dir_all(&base_dir).expect("create config loader temp dir");
        parse_config_document(&base_dir.join("diavola.yml"), raw_yaml)
    }

    #[test]
    fn parses_realistic_yaml_document() {
        let loaded = parse(
            r#"
processes:
  setup:
    kind: task
    cmd: deno task check
  web:
    kind: service
    cmd: deno task dev
    env:
      PORT: "5173"
    dependsOn:
      setup: success
    ready:
      type: http
      url: http://127.0.0.1:5173/health
      intervalMs: 25
      timeoutMs: 500
"#,
        )
        .expect("config should parse");

        assert_eq!(loaded.base_dir, Path::new("/tmp/diavola-config-loader"));
        assert_eq!(loaded.config.processes["web"].env["PORT"], "5173");
        assert!(loaded.config.processes.contains_key("setup"));
        assert_eq!(
            loaded.config.processes["web"].depends_on["setup"],
            crate::domain::config::DependencyCondition::Success
        );
    }

    #[test]
    fn parses_top_level_env() {
        let loaded = parse(
            r#"
env:
  NODE_ENV: production
  LOG_LEVEL: debug
processes:
  web:
    kind: service
    cmd: deno task dev
  worker:
    kind: service
    cmd: deno task worker
    env:
      LOG_LEVEL: trace
"#,
        )
        .expect("config with top-level env should parse");

        assert_eq!(loaded.config.env.len(), 2);
        assert_eq!(
            loaded.config.env.get("NODE_ENV"),
            Some(&"production".to_string())
        );
        assert_eq!(
            loaded.config.env.get("LOG_LEVEL"),
            Some(&"debug".to_string())
        );
        // Per-process env is unchanged
        assert_eq!(
            loaded.config.processes["worker"].env.get("LOG_LEVEL"),
            Some(&"trace".to_string())
        );
    }

    #[test]
    fn top_level_env_defaults_to_empty_when_missing() {
        let loaded = parse(
            r#"
processes:
  web:
    kind: service
    cmd: deno task dev
"#,
        )
        .expect("config without env should parse");

        assert!(loaded.config.env.is_empty());
    }

    #[test]
    fn rejects_unknown_dependency() {
        let error = parse(
            r#"
processes:
  web:
    kind: service
    cmd: deno task dev
    dependsOn:
      missing: ready
"#,
        )
        .expect_err("unknown dependency should fail");

        assert!(error
            .to_string()
            .contains("depends on unknown process `missing`"));
    }

    #[test]
    fn rejects_dependency_cycle() {
        let error = parse(
            r#"
processes:
  api:
    kind: service
    cmd: cargo run
    dependsOn:
      web: ready
  web:
    kind: service
    cmd: deno task dev
    dependsOn:
      api: ready
"#,
        )
        .expect_err("cycle should fail");

        assert!(error.to_string().contains("dependency cycle detected"));
    }

    #[test]
    fn rejects_relative_http_readiness_url() {
        let error = parse(
            r#"
processes:
  web:
    kind: service
    cmd: deno task dev
    ready:
      type: http
      url: /health
"#,
        )
        .expect_err("relative readiness URL should fail");

        assert!(error.to_string().contains("invalid http readiness URL"));
    }

    #[test]
    fn finds_project_diavola_yml() {
        let root =
            std::env::temp_dir().join(format!("diavola-config-loader-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create temp project");
        let config_path = root.join(PROJECT_CONFIG_FILE_NAME);
        fs::write(&config_path, "processes: {}\n").expect("write config");

        assert_eq!(
            find_project_config(&root).expect("find config"),
            Some(config_path)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn finds_config_in_given_directory() {
        let root =
            std::env::temp_dir().join(format!("diavola-config-loader-cwd-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create temp dir");
        let config_path = root.join(PROJECT_CONFIG_FILE_NAME);
        fs::write(&config_path, "processes: {}\n").expect("write config");

        let found = find_config_in_dir_or_parents(&root).expect("should find config");
        assert!(found.is_file());
        assert_eq!(found.file_name().unwrap(), PROJECT_CONFIG_FILE_NAME);
        assert_eq!(found, config_path);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn finds_config_in_parent_directory() {
        let root = std::env::temp_dir().join(format!(
            "diavola-config-loader-parent-{}",
            uuid::Uuid::new_v4()
        ));
        let child = root.join("child");
        fs::create_dir_all(&child).expect("create temp dirs");
        let config_path = root.join(PROJECT_CONFIG_FILE_NAME);
        fs::write(&config_path, "processes: {}\n").expect("write config");

        let found = find_config_in_dir_or_parents(&child).expect("should find config in parent");
        assert!(found.is_file());
        assert_eq!(found.file_name().unwrap(), PROJECT_CONFIG_FILE_NAME);
        assert_eq!(found, config_path);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn returns_none_when_no_config_found() {
        let root =
            std::env::temp_dir().join(format!("diavola-config-loader-none-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create temp dir");

        let result = find_config_in_dir_or_parents(&root);
        assert_eq!(result, None);
        let _ = fs::remove_dir_all(root);
    }
}
