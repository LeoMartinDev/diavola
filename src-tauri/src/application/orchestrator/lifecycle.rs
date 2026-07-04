use std::collections::HashMap;

use indexmap::IndexMap;
use tokio::sync::mpsc;

use crate::domain::{
    process::ProcessStatus,
};

use super::ManagedProcess;

pub(super) fn build_process_env(
    global_env: &IndexMap<String, String>,
    process_env: &IndexMap<String, String>,
) -> HashMap<String, String> {
    let mut env: HashMap<String, String> = std::env::vars().collect();
    for (key, value) in global_env {
        env.insert(key.clone(), value.clone());
    }
    for (key, value) in process_env {
        env.insert(key.clone(), value.clone());
    }
    env
}

/// Resets a managed process to a clean `Pending` state so it can be re-spawned.
/// Extracted from `reset_process` to make the state transition unit-testable.
pub(super) fn reset_managed_process(process: &mut ManagedProcess) {
    process.child = None;
    process.pid = None;
    process.kill_tx = None;
    process.stop_notify_tx = None;
    process.terminating = false;
    process.generation += 1;
    process.snapshot.status = ProcessStatus::Pending;
    process.snapshot.started_at = None;
    process.snapshot.exited_at = None;
    process.snapshot.exit_code = None;
}

/// Marks a managed process for termination and returns its kill signal
/// sender if the child is still alive. Extracted from `stop_process` /
/// `finish_session` to make the state transition unit-testable.
///
/// The deadlock that prompted this helper: the background wait task holds
/// the child mutex for the child's whole lifetime (because `Child::wait`
/// borrows `&mut Child`), so the kill path MUST NOT lock that mutex to call
/// `Child::kill`. Instead it takes `kill_tx` here and the wait task performs
/// the kill inside the lock it already holds, signaled via the channel.
///
/// On Unix the process group is killed synchronously here (the pid is stored
/// separately from the child mutex so no deadlock is possible). This ensures
/// the process exits before `stop_process` returns its snapshot, eliminating
/// the "stuck at Stopping" race.
pub(super) fn begin_process_termination(process: &mut ManagedProcess) -> Option<mpsc::Sender<()>> {
    process.terminating = true;

    #[cfg(unix)]
    if let Some(pid) = process.pid {
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }

    if matches!(
        process.snapshot.status,
        ProcessStatus::Starting | ProcessStatus::Running | ProcessStatus::Ready
    ) {
        process.snapshot.status = ProcessStatus::Stopping;
        process.kill_tx.take()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use indexmap::IndexMap;
    use tokio::sync::{broadcast, mpsc};

    use super::super::ManagedProcess;
    use crate::domain::{
        config::{ProcessConfig, ProcessKind},
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
    fn build_process_env_merges_global_with_process_override() {
        let mut global_env = IndexMap::new();
        global_env.insert("NODE_ENV".to_string(), "development".to_string());
        global_env.insert("LOG_LEVEL".to_string(), "info".to_string());
        let mut process_env = IndexMap::new();
        process_env.insert("LOG_LEVEL".to_string(), "debug".to_string());
        process_env.insert("PORT".to_string(), "3000".to_string());

        let merged = build_process_env(&global_env, &process_env);

        assert_eq!(merged.get("NODE_ENV"), Some(&"development".to_string()));
        assert_eq!(merged.get("LOG_LEVEL"), Some(&"debug".to_string()));
        assert_eq!(merged.get("PORT"), Some(&"3000".to_string()));
    }

    #[test]
    fn build_process_env_empty_global_returns_only_process_env() {
        let global_env = IndexMap::new();
        let mut process_env = IndexMap::new();
        process_env.insert("PORT".to_string(), "3000".to_string());

        let merged = build_process_env(&global_env, &process_env);

        assert_eq!(merged.get("PORT"), Some(&"3000".to_string()));
    }

    #[test]
    fn build_process_env_empty_process_env_returns_only_global() {
        let mut global_env = IndexMap::new();
        global_env.insert("CI".to_string(), "true".to_string());
        let process_env = IndexMap::new();

        let merged = build_process_env(&global_env, &process_env);

        assert_eq!(merged.get("CI"), Some(&"true".to_string()));
    }

    #[test]
    fn build_process_env_inherits_system_environment() {
        let global_env = IndexMap::new();
        let process_env = IndexMap::new();
        let merged = build_process_env(&global_env, &process_env);
        assert!(
            merged.contains_key("PATH") || merged.contains_key("Path"),
            "merged env must inherit system PATH"
        );
    }

    #[test]
    fn build_process_env_process_override_wins_over_system() {
        let mut global_env = IndexMap::new();
        global_env.insert("HOME".to_string(), "/custom".to_string());
        let process_env = IndexMap::new();
        let merged = build_process_env(&global_env, &process_env);
        assert_eq!(merged.get("HOME"), Some(&"/custom".to_string()));
    }

    #[test]
    fn reset_managed_process_restores_pending_state() {
        let mut process = managed_process("api", ProcessKind::Service, ProcessStatus::Failed);
        process.terminating = true;
        process.snapshot.exit_code = Some(1);
        process.snapshot.exited_at = Some(Utc::now());

        reset_managed_process(&mut process);

        assert_eq!(process.snapshot.status, ProcessStatus::Pending);
        assert!(!process.terminating);
        assert!(process.child.is_none());
        assert_eq!(process.snapshot.exit_code, None);
        assert_eq!(process.snapshot.exited_at, None);
        assert_eq!(process.snapshot.started_at, None);
    }

    fn managed_process_with_kill_tx(
        name: &str,
        kind: ProcessKind,
        status: ProcessStatus,
    ) -> (ManagedProcess, mpsc::Receiver<()>) {
        let mut process = managed_process(name, kind.clone(), status);
        let (kill_tx, kill_rx) = mpsc::channel::<()>(1);
        process.kill_tx = Some(kill_tx);
        (process, kill_rx)
    }

    #[tokio::test]
    async fn begin_process_termination_transitions_running_to_stopping() {
        for status in [ProcessStatus::Starting, ProcessStatus::Running, ProcessStatus::Ready] {
            let (mut process, mut kill_rx) =
                managed_process_with_kill_tx("api", ProcessKind::Service, status);

            let kill_tx = begin_process_termination(&mut process);

            assert_eq!(process.snapshot.status, ProcessStatus::Stopping);
            assert!(process.terminating, "terminating flag should be set for {status:?}");
            let kill_tx = kill_tx.expect("kill signal should be returned for {status:?}");
            assert!(
                process.kill_tx.is_none(),
                "kill_tx must be taken (not cloned) so the channel drains"
            );
            // Mirrors `stop_process`: send, then drop. The wait task must
            // observe the signal.
            kill_tx.send(()).await.expect("kill signal should send");
            drop(kill_tx);
            assert_eq!(kill_rx.recv().await, Some(()));
        }
    }

    #[test]
    fn begin_process_termination_leaves_terminal_states_untouched() {
        // A process that already exited (Stopped/Failed/Succeeded) must not
        // produce a kill signal, otherwise stop would hang waiting on a child
        // that no longer exists.
        for status in [
            ProcessStatus::Pending,
            ProcessStatus::Blocked,
            ProcessStatus::Stopping,
            ProcessStatus::Stopped,
            ProcessStatus::Failed,
            ProcessStatus::Succeeded,
        ] {
            let (mut process, _kill_rx) =
                managed_process_with_kill_tx("api", ProcessKind::Service, status);

            let kill_tx = begin_process_termination(&mut process);

            assert!(
                kill_tx.is_none(),
                "no kill signal expected for terminal/pending state {status:?}"
            );
            assert_eq!(
                process.snapshot.status,
                status,
                "status must be unchanged for {status:?}"
            );
        }
    }
}
