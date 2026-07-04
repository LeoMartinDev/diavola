use std::collections::HashMap;

use crate::domain::{
    config::{DependencyCondition, ProcessConfig},
    process::ProcessStatus,
};

use super::ManagedProcess;

pub(super) fn dependencies_satisfied(
    processes: &HashMap<String, ManagedProcess>,
    config: &ProcessConfig,
) -> bool {
    config
        .depends_on
        .iter()
        .all(|(dependency_name, condition)| {
            let Some(dependency) = processes.get(dependency_name) else {
                return false;
            };
            match condition {
                DependencyCondition::Success => {
                    matches!(dependency.snapshot.status, ProcessStatus::Succeeded)
                }
                DependencyCondition::Ready => {
                    matches!(dependency.snapshot.status, ProcessStatus::Ready)
                }
            }
        })
}

pub(super) fn runnable_names(
    processes: &HashMap<String, ManagedProcess>,
    stop_requested: bool,
) -> Vec<String> {
    if stop_requested {
        return Vec::new();
    }
    processes
        .iter()
        .filter_map(|(name, process)| {
            if process.child.is_some()
                || !matches!(
                    process.snapshot.status,
                    ProcessStatus::Pending | ProcessStatus::Blocked | ProcessStatus::Stopped
                )
            {
                return None;
            }
            if dependencies_satisfied(processes, &process.config) {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use std::collections::HashMap;
    use tokio::sync::broadcast;

    use super::super::ManagedProcess;
    use crate::domain::{
        config::{DependencyCondition, ProcessConfig, ProcessKind},
        process::ProcessStatus,
        runtime::{ProcessRuntimeId, ProcessSnapshot},
    };

    use super::*;

    fn managed_process(name: &str, kind: ProcessKind, status: ProcessStatus) -> ManagedProcess {
        let (log_tx, _) = broadcast::channel(4);
        ManagedProcess {
            config: ProcessConfig {
                kind: kind.clone(),
                cmd: format!("run {name}"),
                env: IndexMap::new(),
                depends_on: IndexMap::new(),
                ready: None,
            },
            snapshot: ProcessSnapshot {
                runtime_id: ProcessRuntimeId::new(),
                name: name.to_string(),
                kind,
                status,
                started_at: None,
                exited_at: None,
                exit_code: None,
            },
            child: None,
            pid: None,
            kill_tx: None,
            log_tx,
            terminating: false,
            generation: 0,
            stop_notify_tx: None,
        }
    }

    #[test]
    fn dependencies_satisfied_requires_success_for_task_dependency() {
        let mut processes = HashMap::new();
        processes.insert(
            "setup".to_string(),
            managed_process("setup", ProcessKind::Task, ProcessStatus::Succeeded),
        );
        let mut depends_on = IndexMap::new();
        depends_on.insert("setup".to_string(), DependencyCondition::Success);
        let config = ProcessConfig {
            kind: ProcessKind::Service,
            cmd: "deno task dev".to_string(),
            env: IndexMap::new(),
            depends_on,
            ready: None,
        };

        assert!(dependencies_satisfied(&processes, &config));

        processes
            .get_mut("setup")
            .expect("setup process")
            .snapshot
            .status = ProcessStatus::Ready;
        assert!(!dependencies_satisfied(&processes, &config));
    }

    #[test]
    fn dependencies_satisfied_requires_ready_for_service_dependency() {
        let mut processes = HashMap::new();
        processes.insert(
            "api".to_string(),
            managed_process("api", ProcessKind::Service, ProcessStatus::Running),
        );
        let mut depends_on = IndexMap::new();
        depends_on.insert("api".to_string(), DependencyCondition::Ready);
        let config = ProcessConfig {
            kind: ProcessKind::Service,
            cmd: "deno task dev".to_string(),
            env: IndexMap::new(),
            depends_on,
            ready: None,
        };

        assert!(!dependencies_satisfied(&processes, &config));

        processes
            .get_mut("api")
            .expect("api process")
            .snapshot
            .status = ProcessStatus::Ready;
        assert!(dependencies_satisfied(&processes, &config));
    }
}
