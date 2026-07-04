use std::fs;

use chrono::Utc;
use tempfile::TempDir;

use diavola_lib::{
    application::orchestrator::ProcessOrchestrator,
    domain::{
        process::ProcessStatus,
        project::{ProjectId, ProjectRecord, ProjectSource},
    },
    infrastructure::config_loader::{self, LoadedProjectConfig},
};

fn write_config(dir: &TempDir, yaml: &str) -> LoadedProjectConfig {
    let config_path = dir.path().join("diavola.yml");
    fs::write(&config_path, yaml).expect("write diavola.yml");
    config_loader::load_config(&config_path).expect("load config")
}

#[tokio::test]
#[cfg_attr(
    target_os = "linux",
    ignore = "Tauri GTK event loop requires a main thread on Linux"
)]
async fn task_succeeds_and_session_stops() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let yaml = r#"
processes:
  hello:
    kind: task
    cmd: echo ready
"#;
    let loaded = write_config(&dir, yaml);
    let project = ProjectRecord {
        id: ProjectId::new(),
        name: "test-project".to_string(),
        base_dir: dir.path().to_path_buf(),
        config_source: ProjectSource::ProjectFile,
        config_path: dir.path().join("diavola.yml"),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let app = tauri::Builder::default()
        .build(tauri::generate_context!())
        .expect("build app");
    let orchestrator = ProcessOrchestrator::new();

    let snapshot = orchestrator
        .start_session(
            app.handle().clone(),
            "test-window".to_string(),
            project.clone(),
            loaded,
        )
        .await
        .expect("start session");
    assert_eq!(snapshot.processes.len(), 1);
    assert_eq!(snapshot.processes[0].name, "hello");

    let mut attempts = 0;
    loop {
        let snap = orchestrator
            .snapshot("test-window")
            .await
            .expect("snapshot")
            .expect("session");
        if snap.processes[0].status == ProcessStatus::Succeeded {
            break;
        }
        if snap.processes[0].status == ProcessStatus::Failed {
            panic!("task failed");
        }
        attempts += 1;
        if attempts > 50 {
            panic!("timeout");
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let stopped = orchestrator
        .stop_session(app.handle().clone(), "test-window")
        .await
        .expect("stop")
        .expect("stopped snapshot");
    assert!(stopped.stopped_at.is_some());
    assert!(matches!(
        stopped.processes[0].status,
        ProcessStatus::Stopped | ProcessStatus::Succeeded
    ));
}

#[tokio::test]
#[cfg_attr(
    target_os = "linux",
    ignore = "Tauri GTK event loop requires a main thread on Linux"
)]
async fn process_failure_stops_session() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let yaml = r#"
processes:
  bad:
    kind: task
    cmd: exit 1
"#;
    let loaded = write_config(&dir, yaml);
    let project = ProjectRecord {
        id: ProjectId::new(),
        name: "fail-project".to_string(),
        base_dir: dir.path().to_path_buf(),
        config_source: ProjectSource::ProjectFile,
        config_path: dir.path().join("diavola.yml"),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let app = tauri::Builder::default()
        .build(tauri::generate_context!())
        .expect("build app");
    let orchestrator = ProcessOrchestrator::new();

    orchestrator
        .start_session(
            app.handle().clone(),
            "fail-window".to_string(),
            project,
            loaded,
        )
        .await
        .expect("start");

    let mut attempts = 0;
    loop {
        let snap = orchestrator
            .snapshot("fail-window")
            .await
            .expect("snapshot")
            .expect("session");
        if snap.processes[0].status == ProcessStatus::Failed {
            break;
        }
        attempts += 1;
        if attempts > 50 {
            panic!("timeout");
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let snap = orchestrator
        .snapshot("fail-window")
        .await
        .expect("snapshot")
        .expect("session");
    assert!(
        snap.stopped_at.is_some(),
        "session should be stopped after failure"
    );
}
