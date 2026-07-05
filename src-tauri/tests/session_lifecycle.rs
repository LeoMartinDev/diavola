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

#[tokio::test]
#[cfg_attr(
    target_os = "linux",
    ignore = "Tauri GTK event loop requires a main thread on Linux"
)]
async fn stop_sends_sigterm_and_honors_graceful_exit() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let yaml = r#"
processes:
  polite:
    kind: service
    cmd: sh -c "trap 'exit 0' TERM; sleep 60"
    stopTimeoutMs: 5000
    ready:
      type: delay
      durationMs: 50
"#;
    let loaded = write_config(&dir, yaml);
    let project = ProjectRecord {
        id: ProjectId::new(),
        name: "polite-project".to_string(),
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
        .start_session(app.handle().clone(), "polite-window".to_string(), project, loaded)
        .await
        .expect("start");

    // Wait until Running/Ready.
    let mut attempts = 0;
    loop {
        let snap = orchestrator
            .snapshot("polite-window")
            .await
            .expect("snapshot")
            .expect("session");
        if matches!(snap.processes[0].status, ProcessStatus::Ready | ProcessStatus::Running) {
            break;
        }
        attempts += 1;
        if attempts > 50 {
            panic!("never reached ready");
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let start = std::time::Instant::now();
    let stopped = orchestrator
        .stop_session(app.handle().clone(), "polite-window")
        .await
        .expect("stop")
        .expect("stopped snapshot");
    let elapsed = start.elapsed();

    // Graceful exit (SIGTERM trapped -> exit 0) must happen well before the 5s
    // grace window elapses, so escalation is NOT triggered.
    assert!(elapsed < std::time::Duration::from_secs(4), "took {elapsed:?}");
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
async fn stop_escalates_to_sigkill_after_grace() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let yaml = r#"
processes:
  stubborn:
    kind: service
    cmd: sh -c "trap '' TERM; sleep 60"
    stopTimeoutMs: 1000
    ready:
      type: delay
      durationMs: 50
"#;
    let loaded = write_config(&dir, yaml);
    let project = ProjectRecord {
        id: ProjectId::new(),
        name: "stubborn-project".to_string(),
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
        .start_session(app.handle().clone(), "stubborn-window".to_string(), project, loaded)
        .await
        .expect("start");

    let mut attempts = 0;
    loop {
        let snap = orchestrator
            .snapshot("stubborn-window")
            .await
            .expect("snapshot")
            .expect("session");
        if matches!(snap.processes[0].status, ProcessStatus::Ready | ProcessStatus::Running) {
            break;
        }
        attempts += 1;
        if attempts > 50 {
            panic!("never reached ready");
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let start = std::time::Instant::now();
    orchestrator
        .stop_session(app.handle().clone(), "stubborn-window")
        .await
        .expect("stop");
    let elapsed = start.elapsed();

    // SIGTERM ignored; SIGKILL must fire after the 1s grace. Bounded well
    // below the trap's 60s sleep.
    assert!(elapsed < std::time::Duration::from_secs(5), "took {elapsed:?}");
    let snap = orchestrator
        .snapshot("stubborn-window")
        .await
        .expect("snapshot")
        .expect("session");
    assert!(snap.stopped_at.is_some());
}

#[cfg(windows)]
#[tokio::test]
async fn windows_stop_kills_process_tree_via_job() {
    use std::net::{TcpListener, TcpStream};
    use std::process::Command;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);

    let dir = tempfile::tempdir().expect("temp dir");
    let yaml = format!(
        r#"
processes:
  holder:
    kind: service
    cmd: cmd /C "node -e \"require('net').createServer().listen({port})\""
    stopTimeoutMs: 2000
    ready:
      type: delay
      durationMs: 300
"#
    );
    std::fs::write(dir.path().join("diavola.yml"), &yaml).expect("write config");
    let loaded = config_loader::load_config(&dir.path().join("diavola.yml")).expect("load");

    let project = ProjectRecord {
        id: ProjectId::new(),
        name: "port-holder".to_string(),
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
        .start_session(app.handle().clone(), "win-window".to_string(), project, loaded)
        .await
        .expect("start");

    let mut attempts = 0;
    while TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
        attempts += 1;
        if attempts > 50 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    orchestrator
        .stop_session(app.handle().clone(), "win-window")
        .await
        .expect("stop");

    let freed = TcpListener::bind(format!("127.0.0.1:{port}")).is_ok();
    assert!(freed, "port {port} should be free after stop (no orphan)");
    let _ = Command::new("taskkill").args(["/IM", "node.exe", "/F"]).status();
}
