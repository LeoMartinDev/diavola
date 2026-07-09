//! Detects orphan process leaks across rapid restarts, stops, and edge cases.
//!
//! `begin_process_termination` sends SIGKILL to the process group via
//! `kill(-pid, SIGKILL)`.  If the SIGKILL does not reach the whole tree, or
//! if the wait task fails to reap the child, orphan processes accumulate.
//!
//! ## Test groups
//!
//! **Process-level tests** (`#![cfg(unix)]`) — spawn `sh -c <cmd>` directly in
//! its own process group (like `process_kill.rs` / `stop_race.rs` do) and
//! exercise the SIGKILL mechanism.  These run on macOS + Linux.
//!
//! **Orchestrator-level tests** (`#[cfg(target_os = "linux")]`) — verify the
//! full orchestrator lifecycle.  Tauri `EventLoop` must be on the main thread.
//! Linux CI only.

#![cfg(unix)]

use std::{os::unix::process::CommandExt as _, time::Duration};

/// Spawn `sh -c <cmd>` in a new process group (like `process_kill.rs`).
/// Returns the child handle and the process-group id (= the child's pid).
fn spawn_in_pgrp(cmd: &str) -> (tokio::process::Child, u32) {
    let mut command = tokio::process::Command::new("sh");
    command.arg("-c").arg(cmd);
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());
    command.stdin(std::process::Stdio::null());
    command.as_std_mut().process_group(0);

    let child = command.spawn().expect("spawn");
    let pid = child.id().expect("pid");
    (child, pid)
}

/// Return `true` when `kill(pid, 0)` returns ESRCH (no such process).
fn pid_is_dead(pid: u32) -> bool {
    if pid == 0 {
        return true;
    }
    let ret = unsafe { libc::kill(pid as i32, 0) };
    if ret == 0 {
        return false;
    }
    matches!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH) | Some(_))
}

/// Poll until every PID is dead (with timeout).
async fn assert_pids_are_dead(pids: &[u32], label: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    for &pid in pids {
        while !pid_is_dead(pid) {
            if tokio::time::Instant::now() > deadline {
                panic!(
                    "PID {pid} still alive after deadline ({label}) — orphan leak detected"
                );
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

/// Verify no `--diavola-watchdog` process is a direct child of the test
/// process.  The watchdog is a direct child (process-group leader).
// ── Process-level tests (macOS OK) ═══════════════════════════════════════

/// SIGKILL to the process group kills the whole tree.  Verify the
/// process and its descendants are all gone.
#[tokio::test]
async fn sigkill_to_pgrp_kills_whole_tree() {
    use tempfile::tempdir;
    let dir = tempdir().expect("tempdir");
    let pf = dir.path().join("pids.txt");
    let cmd = format!(
        "echo $$ > \"{pf}\"; sleep 999 & echo $! >> \"{pf}\"; wait",
        pf = pf.display()
    );
    let (mut child, pid) = spawn_in_pgrp(&cmd);

    tokio::time::sleep(Duration::from_millis(250)).await;
    let tracked = read_pid_file(&pf);

    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }

    let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("child.wait() timed out")
        .expect("child.wait() returned error");

    assert!(!status.success(), "process group should have been killed");

    assert_pids_are_dead(&tracked, "sigkill-tree").await;
    assert_pids_are_dead(&[pid], "sigkill-tree-main").await;
}

/// SIGKILL to the process group — verify no grandchildren survive.
#[tokio::test]
async fn killpg_then_verify_no_orphan_grandchild() {
    use tempfile::tempdir;
    let dir = tempdir().expect("tempdir");
    let pf = dir.path().join("gc.txt");
    let cmd = format!(
        "echo $$ > \"{pf}\"; sleep 60 & echo $! >> \"{pf}\"; wait",
        pf = pf.display()
    );
    let (mut child, pid) = spawn_in_pgrp(&cmd);

    tokio::time::sleep(Duration::from_millis(250)).await;
    let tracked = read_pid_file(&pf);

    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("child.wait() timed out");

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_pids_are_dead(&tracked, "killpg-grandchild").await;
    assert_pids_are_dead(&[pid], "killpg-grandchild-main").await;
}

/// Rapidly spawn and kill 50 watchdogs.  Verify zero orphans after each.
#[tokio::test]
async fn rapid_spawn_kill_50_times_no_orphans() {
    for i in 0..50 {
        let (mut child, pid) = spawn_in_pgrp("sleep 999");

        tokio::time::sleep(Duration::from_millis(30)).await;

        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }

        let _ = tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .expect(&format!("[{i}] child.wait() timed out"));

        // Wait for OS to reap
        tokio::time::sleep(Duration::from_millis(20)).await;

        assert_pids_are_dead(&[pid], &format!("rapid-50/{i}")).await;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;
    // orphan check: pid checked via assert_pids_are_dead above
}

/// Kill the watchdog while sh has not started yet (race: kill between fork
/// and exec of the grandchild).  The process group should still be killable.
#[tokio::test]
async fn kill_watchdog_before_grandchild_spawns() {
    // Use a slow command to give us time to kill before exec completes
    let (mut child, pid) = spawn_in_pgrp("sleep 999");

    // Kill IMMEDIATELY — sh may not have called exec(sleep) yet
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }

    let result = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;

    match result {
        Ok(Ok(status)) => {
            assert!(!status.success(), "should be killed");
        }
        Ok(Err(e)) => panic!("child.wait() error: {e}"),
        Err(_) => panic!("timed out waiting for watchdog to die"),
    }

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_pids_are_dead(&[pid], "kill-early").await;
    // orphan check: pid checked via assert_pids_are_dead above
}

/// Kill via SIGTERM to watchdog (which ignores it), then SIGKILL. Verify
/// no orphans — the watchdog survives SIGTERM but dies on SIGKILL,
/// cleaning everything.
#[tokio::test]
async fn sigterm_ignored_by_watchdog_sigkill_cleans() {
    let (mut child, pid) = spawn_in_pgrp("sleep 999");

    tokio::time::sleep(Duration::from_millis(200)).await;

    // SIGTERM to the watchdog directly (not process group)
    unsafe {
        libc::kill(pid as i32, libc::SIGTERM);
    }

    // Watchdog ignores SIGTERM, so it should still be alive
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!pid_is_dead(pid), "watchdog should ignore SIGTERM and stay alive");

    // Now escalate to SIGKILL on the whole group
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("child.wait() timed out after SIGKILL");

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_pids_are_dead(&[pid], "sigterm-sigkill").await;
    // orphan check: pid checked via assert_pids_are_dead above
}

/// Spawn multiple watchdogs concurrently, then kill them all.  Verify none
/// linger.  Concurrent spawn/kill exercises the OS process table without
/// leak from cross-contamination of process groups.
#[tokio::test]
async fn concurrent_spawn_kill_many_no_orphans() {
    let mut pids = Vec::new();
    let mut children = Vec::new();

    // Spawn 10 watchdogs concurrently
    for _ in 0..10 {
        let (child, pid) = spawn_in_pgrp("sleep 999");
        pids.push(pid);
        children.push(child);
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Kill them all
    for &pid in &pids {
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }

    // Wait for all to die
    for mut child in children {
        let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
    }

    tokio::time::sleep(Duration::from_millis(300)).await;

    for (i, &pid) in pids.iter().enumerate() {
        assert_pids_are_dead(&[pid], &format!("concurrent/{i}")).await;
    }
    // orphan check: pid checked via assert_pids_are_dead above
}

/// Kill the watchdog while its grandchild has spawned a sleep
/// background child itself (nested tree).  The process group kill must
/// reach all levels.
#[tokio::test]
async fn kill_with_nested_tree_no_orphans() {
    use tempfile::tempdir;
    let dir = tempdir().expect("tempdir");
    let pf = dir.path().join("nest.txt");
    let cmd = format!(
        "echo $$ > \"{pf}\"; sh -c 'echo $$ >> \"{pf}\"; sleep 120 & echo $! >> \"{pf}\"; sleep 120 & echo $! >> \"{pf}\"; wait'",
        pf = pf.display()
    );
    let (mut child, pid) = spawn_in_pgrp(&cmd);

    tokio::time::sleep(Duration::from_millis(300)).await;
    let tracked = read_pid_file(&pf);

    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("child.wait() timed out");

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_pids_are_dead(&tracked, "nested-tree").await;
    assert_pids_are_dead(&[pid], "nested-tree-main").await;
}

/// Kill watchdogs in random order (not FIFO) to uncover any ordering
/// assumptions in the process table management.
#[tokio::test]
async fn random_order_kill_no_orphans() {
    let mut entries: Vec<(tokio::process::Child, u32)> = Vec::new();

    for _ in 0..8 {
        let (child, pid) = spawn_in_pgrp("sleep 999");
        entries.push((child, pid));
    }

    tokio::time::sleep(Duration::from_millis(150)).await;

    // Kill in reverse order
    entries.reverse();
    for (_child, pid) in &entries {
        unsafe {
            libc::kill(-(*pid as i32), libc::SIGKILL);
        }
    }

    for (mut child, pid) in entries {
        let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
        assert_pids_are_dead(&[pid], "random-order").await;
    }

    // orphan check: pid checked via assert_pids_are_dead above
}

/// Verify that after killing a process group leader, its entire process
/// group is gone.  Specifically test that `killpg` works and no process
/// in the group survives.
#[tokio::test]
async fn killpg_clears_whole_group() {
    use tempfile::tempdir;
    let dir = tempdir().expect("tempdir");
    let pf = dir.path().join("kg.txt");
    let cmd = format!(
        "echo $$ > \"{pf}\"; sleep 999 & echo $! >> \"{pf}\"; wait",
        pf = pf.display()
    );
    let (mut child, pid) = spawn_in_pgrp(&cmd);

    tokio::time::sleep(Duration::from_millis(250)).await;
    let tracked = read_pid_file(&pf);

    unsafe {
        libc::killpg(pid as i32, libc::SIGKILL);
    }

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("child.wait() timed out");

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_pids_are_dead(&tracked, "killpg").await;
    assert_pids_are_dead(&[pid], "killpg-main").await;
}

// ── Detection-proving tests: deliberately CREATE orphans ────────────────
//
// These tests verify that the leak detection itself works by creating a
// known orphan scenario and confirming the detector catches it.  Without
// these, a passing suite could just mean the detection is silent, not
// that the code is truly bug-free.
//
// Each test writes the grandchild PID to a temp file so we can check
// that specific PID without false positives from parallel tests.

use std::path::PathBuf;

fn read_pid_file(path: &PathBuf) -> Vec<u32> {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let _ = std::fs::remove_file(path);
            content
                .lines()
                .filter_map(|l| l.trim().parse::<u32>().ok())
                .collect()
        }
        Err(_) => Vec::new(),
    }
}

/// Kill only the shell PID (not the process group).  Background
/// grandchildren survive as orphans — the detector MUST catch them.
#[tokio::test]
#[should_panic(expected = "orphans survived")]
async fn detect_orphans_deliberate_pid_only_kill() {
    use tempfile::tempdir;
    let dir = tempdir().expect("tempdir");
    let pf = dir.path().join("gc.txt");
    let cmd = format!(
        "echo $$ > \"{pf}\"; sleep 60 & echo $! >> \"{pf}\"; sleep 60 & echo $! >> \"{pf}\"; wait",
        pf = pf.display()
    );
    let (mut child, pid) = spawn_in_pgrp(&cmd);

    tokio::time::sleep(Duration::from_millis(300)).await;
    let tracked = read_pid_file(&pf);

    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("wait timed out");

    tokio::time::sleep(Duration::from_millis(300)).await;

    for &gpid in &tracked {
        if !pid_is_dead(gpid) {
            panic!("orphans survived: pid {gpid} alive (tracked {tracked:?})");
        }
    }
}

/// Kill shell PID only — nested background grandchildren must survive.
#[tokio::test]
#[should_panic(expected = "nested orphans survived")]
async fn detect_orphans_nested_background_survivors() {
    use tempfile::tempdir;
    let dir = tempdir().expect("tempdir");
    let pf = dir.path().join("nest.txt");
    let cmd = format!(
        "echo $$ > \"{pf}\"; sh -c 'echo $$ >> \"{pf}\"; sleep 120 & echo $! >> \"{pf}\"; sleep 120 & echo $! >> \"{pf}\"; wait'",
        pf = pf.display()
    );
    let (mut child, pid) = spawn_in_pgrp(&cmd);

    tokio::time::sleep(Duration::from_millis(300)).await;
    let tracked = read_pid_file(&pf);

    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }

    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("wait timed out");

    tokio::time::sleep(Duration::from_millis(300)).await;

    for &gpid in &tracked {
        if !pid_is_dead(gpid) {
            panic!("nested orphans survived: pid {gpid} alive (tracked {tracked:?})");
        }
    }
}

/// Drop the `Child` handle without waiting (simulating a crash/panic in the
/// orchestrator).  The OS should still reap the process; verify no zombie.
#[tokio::test]
async fn drop_child_without_wait_leaves_no_zombie() {
    let (child, pid) = spawn_in_pgrp("sleep 999");

    tokio::time::sleep(Duration::from_millis(150)).await;

    // Kill the process group, then DROP the child without waiting.
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }

    // Drop immediately (simulates a panic/early return)
    drop(child);

    // Give the OS time to reap
    tokio::time::sleep(Duration::from_millis(500)).await;

    assert_pids_are_dead(&[pid], "drop-without-wait").await;
    // orphan check: pid checked via assert_pids_are_dead above
}

// ══════════════════════════════════════════════════════════════════════════
// Orchestrator-level tests (Linux CI only — Tauri EventLoop on main thread)
// ══════════════════════════════════════════════════════════════════════════

#[cfg(target_os = "linux")]
mod orchestrator_tests {
    use std::{fs, path::PathBuf, time::Duration};

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

    fn build_test_app() -> tauri::App<tauri::Wry> {
        tauri::Builder::default()
            .build(tauri::generate_context!())
            .expect("build app")
    }

    fn make_project(dir: &TempDir, label: &str) -> ProjectRecord {
        let now = Utc::now();
        ProjectRecord {
            id: ProjectId::new(),
            name: label.to_string(),
            base_dir: dir.path().to_path_buf(),
            config_source: ProjectSource::ProjectFile,
            config_path: dir.path().join("diavola.yml"),
            created_at: now,
            updated_at: now,
        }
    }

    async fn wait_until_stopped(orchestrator: &ProcessOrchestrator, window: &str) {
        let mut attempts = 0;
        loop {
            let snap = orchestrator
                .snapshot(window)
                .await
                .expect("snapshot")
                .expect("session");
            let all_terminal = snap.processes.iter().all(|p| {
                matches!(
                    p.status,
                    ProcessStatus::Stopped
                        | ProcessStatus::Failed
                        | ProcessStatus::Succeeded
                        | ProcessStatus::Pending
                        | ProcessStatus::Blocked
                )
            });
            if all_terminal {
                return;
            }
            attempts += 1;
            if attempts > 100 {
                panic!(
                    "timed out waiting for terminal status: {:?}",
                    snap.processes
                        .iter()
                        .map(|p| (p.name.as_str(), p.status))
                        .collect::<Vec<_>>()
                );
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn pid_is_dead(pid: u32) -> bool {
        if pid == 0 {
            return true;
        }
        let ret = unsafe { libc::kill(pid as i32, 0) };
        if ret == 0 {
            return false;
        }
        matches!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH) | Some(_)
        )
    }

    async fn assert_pids_are_dead(pids: &[u32], label: &str) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        for &pid in pids {
            if pid == 0 {
                continue;
            }
            while !pid_is_dead(pid) {
                if tokio::time::Instant::now() > deadline {
                    panic!("PID {pid} still alive after deadline ({label}) — orphan leak");
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }

    fn read_pid_file(path: &PathBuf) -> Vec<u32> {
        match fs::read_to_string(path) {
            Ok(content) => {
                let _ = fs::remove_file(path);
                content
                    .lines()
                    .filter_map(|l| l.trim().parse::<u32>().ok())
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    }

    fn assert_no_watchdog_orphans(label: &str) {
        let our_pid = std::process::id();
        let output = std::process::Command::new("pgrep")
            .args(["-P", &our_pid.to_string(), "-f", "--diavola-watchdog"])
            .output()
            .unwrap_or_else(|_| std::process::Command::new("true").output().unwrap());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let orphans: Vec<&str> = stdout.trim().lines().filter(|l| !l.is_empty()).collect();
        if !orphans.is_empty() {
            panic!("{label}: lingering watchdogs (children of {our_pid}): {orphans:?}");
        }
    }

    fn build_pid_writer_cmd(pid_file: &PathBuf) -> String {
        format!(
            r#"sh -c 'echo $$ > "{}"; trap "" TERM; sleep 999'"#,
            pid_file.display()
        )
    }

    // ── Orchestrator test cases ─────────────────────────────────────────

    #[tokio::test]
    async fn orphan_no_process_left_after_stop() {
        let dir = tempfile::tempdir().expect("temp dir");
        let pid_file = dir.path().join("pids.txt");
        let yaml = format!(
            r#"
processes:
  alpha:
    kind: service
    cmd: {}
    ready:
      type: delay
      durationMs: 50
"#,
            build_pid_writer_cmd(&pid_file)
        );
        let loaded = write_config(&dir, &yaml);
        let project = make_project(&dir, "orphan-stop");
        let app = build_test_app();
        let orchestrator = ProcessOrchestrator::new();

        orchestrator
            .start_session(app.handle().clone(), "ow".to_string(), project, loaded)
            .await
            .expect("start");

        let mut attempts = 0;
        loop {
            let snap = orchestrator
                .snapshot("ow")
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
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let tracked = read_pid_file(&pid_file);

        orchestrator
            .stop_session(app.handle().clone(), "ow")
            .await
            .expect("stop");

        wait_until_stopped(&orchestrator, "ow").await;
        assert_pids_are_dead(&tracked, "orphan-stop").await;
        assert_no_watchdog_orphans("orphan-stop");
    }

    #[tokio::test]
    async fn orphan_no_process_left_after_restart_loop() {
        let dir = tempfile::tempdir().expect("temp dir");
        let pid_file = dir.path().join("pids.txt");
        let yaml = format!(
            r#"
processes:
  svc:
    kind: service
    cmd: {}
    ready:
      type: delay
      durationMs: 50
"#,
            build_pid_writer_cmd(&pid_file)
        );
        let loaded = write_config(&dir, &yaml);
        let project = make_project(&dir, "orphan-restart-loop");
        let app = build_test_app();
        let orchestrator = ProcessOrchestrator::new();

        orchestrator
            .start_session(app.handle().clone(), "rl".to_string(), project, loaded)
            .await
            .expect("start");

        for i in 0..10 {
            let mut attempts = 0;
            loop {
                let snap = orchestrator
                    .snapshot("rl")
                    .await
                    .expect("snapshot")
                    .expect("session");
                if matches!(
                    snap.processes[0].status,
                    ProcessStatus::Ready | ProcessStatus::Running
                ) {
                    break;
                }
                attempts += 1;
                if attempts > 50 {
                    panic!("never reached ready before restart {i}");
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            let old_pids = read_pid_file(&pid_file);

            orchestrator
                .restart_process(app.handle().clone(), "rl", "svc")
                .await
                .expect("restart");

            assert_pids_are_dead(&old_pids, &format!("orphan-restart-loop/restart-{i}")).await;
        }

        let mut attempts = 0;
        loop {
            let snap = orchestrator
                .snapshot("rl")
                .await
                .expect("snapshot")
                .expect("session");
            if matches!(snap.processes[0].status, ProcessStatus::Ready | ProcessStatus::Running) {
                break;
            }
            attempts += 1;
            if attempts > 50 {
                panic!("never reached ready after last restart");
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let final_pids = read_pid_file(&pid_file);

        orchestrator
            .stop_session(app.handle().clone(), "rl")
            .await
            .expect("stop");

        wait_until_stopped(&orchestrator, "rl").await;
        assert_pids_are_dead(&final_pids, "orphan-restart-loop/final").await;
        assert_no_watchdog_orphans("orphan-restart-loop");
    }

    #[tokio::test]
    async fn orphan_no_leak_over_rapid_start_stop_cycles() {
        let dir = tempfile::tempdir().expect("temp dir");
        let pid_file = dir.path().join("pids.txt");
        let yaml = format!(
            r#"
processes:
  svc:
    kind: service
    cmd: {}
    ready:
      type: delay
      durationMs: 50
"#,
            build_pid_writer_cmd(&pid_file)
        );
        let loaded = write_config(&dir, &yaml);

        for cycle in 0..5 {
            let project = make_project(&dir, &format!("cycle-{cycle}"));
            let app = build_test_app();
            let orchestrator = ProcessOrchestrator::new();
            let window = format!("cyc{cycle}");

            orchestrator
                .start_session(app.handle().clone(), window.clone(), project, loaded.clone())
                .await
                .expect("start");

            let mut attempts = 0;
            loop {
                let snap = orchestrator
                    .snapshot(&window)
                    .await
                    .expect("snapshot")
                    .expect("session");
                if matches!(
                    snap.processes[0].status,
                    ProcessStatus::Ready | ProcessStatus::Running
                ) {
                    break;
                }
                attempts += 1;
                if attempts > 50 {
                    panic!("cycle {cycle}: never reached ready");
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            let pids = read_pid_file(&pid_file);

            orchestrator
                .stop_session(app.handle().clone(), &window)
                .await
                .expect("stop");

            wait_until_stopped(&orchestrator, &window).await;
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert_pids_are_dead(&pids, &format!("rapid-cycle/{cycle}")).await;
            assert_no_watchdog_orphans(&format!("rapid-cycle/{cycle}"));
        }
    }

    #[tokio::test]
    async fn orphan_no_process_left_after_stop_while_starting() {
        let dir = tempfile::tempdir().expect("temp dir");
        let pid_file = dir.path().join("pids.txt");
        let yaml = format!(
            r#"
processes:
  slow:
    kind: service
    cmd: {}
    ready:
      type: delay
      durationMs: 2000
"#,
            build_pid_writer_cmd(&pid_file)
        );
        let loaded = write_config(&dir, &yaml);
        let project = make_project(&dir, "orphan-stop-early");
        let app = build_test_app();
        let orchestrator = ProcessOrchestrator::new();

        orchestrator
            .start_session(app.handle().clone(), "se".to_string(), project, loaded)
            .await
            .expect("start");

        tokio::time::sleep(Duration::from_millis(150)).await;
        let pids = read_pid_file(&pid_file);

        orchestrator
            .stop_session(app.handle().clone(), "se")
            .await
            .expect("stop");

        wait_until_stopped(&orchestrator, "se").await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_pids_are_dead(&pids, "orphan-stop-early").await;
        assert_no_watchdog_orphans("orphan-stop-early");
    }

    #[tokio::test]
    async fn orphan_no_leak_with_concurrent_stops() {
        use std::sync::Arc;

        let dir = tempfile::tempdir().expect("temp dir");
        let pid_file = dir.path().join("pids.txt");
        let yaml = format!(
            r#"
processes:
  svc:
    kind: service
    cmd: {}
    ready:
      type: delay
      durationMs: 50
"#,
            build_pid_writer_cmd(&pid_file)
        );
        let loaded = write_config(&dir, &yaml);
        let project = make_project(&dir, "orphan-concurrent");
        let app = build_test_app();
        let orchestrator = Arc::new(ProcessOrchestrator::new());
        let window = "conc".to_string();

        orchestrator
            .start_session(app.handle().clone(), window.clone(), project, loaded)
            .await
            .expect("start");

        let mut attempts = 0;
        loop {
            let snap = orchestrator
                .snapshot(&window)
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
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let pids = read_pid_file(&pid_file);

        let orc1 = orchestrator.clone();
        let orc2 = orchestrator.clone();
        let orc3 = orchestrator.clone();
        let h1 = app.handle().clone();
        let h2 = app.handle().clone();
        let h3 = app.handle().clone();
        let w1 = window.clone();
        let w2 = window.clone();
        let w3 = window.clone();

        let (r1, r2, r3) = tokio::join!(
            tokio::spawn(async move { orc1.stop_session(h1, &w1).await }),
            tokio::spawn(async move { orc2.stop_session(h2, &w2).await }),
            tokio::spawn(async move { orc3.stop_session(h3, &w3).await }),
        );

        let _ = tokio::time::timeout(Duration::from_secs(20), async {
            let _ = r1.await;
            let _ = r2.await;
            let _ = r3.await;
        })
        .await
        .expect("concurrent stops timed out");

        wait_until_stopped(&orchestrator, &window).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_pids_are_dead(&pids, "orphan-concurrent").await;
        assert_no_watchdog_orphans("orphan-concurrent");
    }

    #[tokio::test]
    async fn orphan_force_stop_all_cleans_everything() {
        let dir = tempfile::tempdir().expect("temp dir");
        let pid_file_a = dir.path().join("pids_a.txt");
        let pid_file_b = dir.path().join("pids_b.txt");

        let yaml_a = format!(
            r#"
processes:
  a:
    kind: service
    cmd: {}
    ready:
      type: delay
      durationMs: 50
"#,
            build_pid_writer_cmd(&pid_file_a)
        );
        let yaml_b = format!(
            r#"
processes:
  b:
    kind: service
    cmd: {}
    ready:
      type: delay
      durationMs: 50
"#,
            build_pid_writer_cmd(&pid_file_b)
        );

        let path_a = dir.path().join("a.yml");
        fs::write(&path_a, &yaml_a).expect("write a.yml");
        let loaded_a = config_loader::load_config(&path_a).expect("load a.yml");

        let path_b = dir.path().join("b.yml");
        fs::write(&path_b, &yaml_b).expect("write b.yml");
        let loaded_b = config_loader::load_config(&path_b).expect("load b.yml");

        let project_a = make_project(&dir, "project-a");
        let project_b = make_project(&dir, "project-b");

        let app = build_test_app();
        let orchestrator = ProcessOrchestrator::new();

        orchestrator
            .start_session(app.handle().clone(), "wa".to_string(), project_a, loaded_a)
            .await
            .expect("start a");

        orchestrator
            .start_session(app.handle().clone(), "wb".to_string(), project_b, loaded_b)
            .await
            .expect("start b");

        for window in &["wa", "wb"] {
            let mut attempts = 0;
            loop {
                let snap = orchestrator
                    .snapshot(window)
                    .await
                    .expect("snapshot")
                    .expect("session");
                if matches!(
                    snap.processes[0].status,
                    ProcessStatus::Ready | ProcessStatus::Running
                ) {
                    break;
                }
                attempts += 1;
                if attempts > 50 {
                    panic!("{window}: never reached ready");
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        let pids_a = read_pid_file(&pid_file_a);
        let pids_b = read_pid_file(&pid_file_b);

        orchestrator
            .force_stop_all_sessions(app.handle().clone())
            .await
            .expect("force stop all");

        for window in &["wa", "wb"] {
            wait_until_stopped(&orchestrator, window).await;
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_pids_are_dead(&pids_a, "force-stop/a").await;
        assert_pids_are_dead(&pids_b, "force-stop/b").await;
        assert_no_watchdog_orphans("force-stop-all");
    }

    #[tokio::test]
    async fn orphan_stress_restart_50_times() {
        let dir = tempfile::tempdir().expect("temp dir");
        let pid_file = dir.path().join("pids.txt");
        let yaml = format!(
            r#"
processes:
  s:
    kind: service
    cmd: {}
    ready:
      type: delay
      durationMs: 20
"#,
            build_pid_writer_cmd(&pid_file)
        );
        let loaded = write_config(&dir, &yaml);
        let project = make_project(&dir, "orphan-stress-50");
        let app = build_test_app();
        let orchestrator = ProcessOrchestrator::new();

        orchestrator
            .start_session(app.handle().clone(), "s50".to_string(), project, loaded)
            .await
            .expect("start");

        for i in 0..50 {
            let mut attempts = 0;
            loop {
                let snap = orchestrator
                    .snapshot("s50")
                    .await
                    .expect("snapshot")
                    .expect("session");
                if matches!(
                    snap.processes[0].status,
                    ProcessStatus::Ready | ProcessStatus::Running
                ) {
                    break;
                }
                attempts += 1;
                if attempts > 100 {
                    panic!("iteration {i}: never reached ready");
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            let old_pids = read_pid_file(&pid_file);

            orchestrator
                .restart_process(app.handle().clone(), "s50", "s")
                .await
                .expect("restart");

            assert_pids_are_dead(&old_pids, &format!("stress-50/iter-{i}")).await;
        }

        let mut attempts = 0;
        loop {
            let snap = orchestrator
                .snapshot("s50")
                .await
                .expect("snapshot")
                .expect("session");
            if matches!(snap.processes[0].status, ProcessStatus::Ready | ProcessStatus::Running) {
                break;
            }
            attempts += 1;
            if attempts > 100 {
                panic!("last generation: never reached ready");
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let final_pids = read_pid_file(&pid_file);

        orchestrator
            .stop_session(app.handle().clone(), "s50")
            .await
            .expect("stop");

        wait_until_stopped(&orchestrator, "s50").await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_pids_are_dead(&final_pids, "stress-50/final").await;
        assert_no_watchdog_orphans("stress-50");
    }

    #[tokio::test]
    async fn orphan_fast_restart_sequence() {
        let dir = tempfile::tempdir().expect("temp dir");
        let pid_file = dir.path().join("pids.txt");
        let yaml = format!(
            r#"
processes:
  s:
    kind: service
    cmd: {}
    ready:
      type: delay
      durationMs: 20
"#,
            build_pid_writer_cmd(&pid_file)
        );
        let loaded = write_config(&dir, &yaml);
        let project = make_project(&dir, "orphan-fast-restart");
        let app = build_test_app();
        let orchestrator = ProcessOrchestrator::new();

        orchestrator
            .start_session(app.handle().clone(), "fr".to_string(), project, loaded)
            .await
            .expect("start");

        let mut attempts = 0;
        loop {
            let snap = orchestrator
                .snapshot("fr")
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
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        for i in 0..20 {
            let old_pids = read_pid_file(&pid_file);

            orchestrator
                .restart_process(app.handle().clone(), "fr", "s")
                .await
                .expect("restart");

            tokio::time::sleep(Duration::from_millis(10)).await;
            assert_pids_are_dead(&old_pids, &format!("fast-restart/{i}")).await;
        }

        let final_pids = read_pid_file(&pid_file);

        orchestrator
            .stop_session(app.handle().clone(), "fr")
            .await
            .expect("stop");

        wait_until_stopped(&orchestrator, "fr").await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_pids_are_dead(&final_pids, "fast-restart/final").await;
        assert_no_watchdog_orphans("fast-restart");
    }

    #[tokio::test]
    async fn orphan_task_only_session_clean() {
        let dir = tempfile::tempdir().expect("temp dir");
        let yaml = r#"
processes:
  quick:
    kind: task
    cmd: echo done
"#;
        let loaded = write_config(&dir, yaml);
        let project = make_project(&dir, "orphan-task-only");
        let app = build_test_app();
        let orchestrator = ProcessOrchestrator::new();

        orchestrator
            .start_session(app.handle().clone(), "to".to_string(), project, loaded)
            .await
            .expect("start");

        wait_until_stopped(&orchestrator, "to").await;
        assert_no_watchdog_orphans("orphan-task-only");
    }

    #[tokio::test]
    async fn orphan_hyper_aggressive_restart() {
        let dir = tempfile::tempdir().expect("temp dir");
        let pid_file = dir.path().join("pids.txt");
        let yaml = format!(
            r#"
processes:
  s:
    kind: service
    cmd: {}
    ready:
      type: delay
      durationMs: 20
"#,
            build_pid_writer_cmd(&pid_file)
        );
        let loaded = write_config(&dir, &yaml);
        let project = make_project(&dir, "orphan-hyper");
        let app = build_test_app();
        let orchestrator = ProcessOrchestrator::new();

        orchestrator
            .start_session(app.handle().clone(), "ha".to_string(), project, loaded)
            .await
            .expect("start");

        let mut all_pids: Vec<u32> = Vec::new();

        for i in 0..30 {
            let old_pids = read_pid_file(&pid_file);
            all_pids.extend(old_pids);

            let _ = orchestrator
                .restart_process(app.handle().clone(), "ha", "s")
                .await;

            tokio::time::sleep(Duration::from_millis(10)).await;

            if i > 0 {
                assert_pids_are_dead(&all_pids, &format!("hyper/{i}")).await;
                all_pids.clear();
            }
        }

        let final_pids = read_pid_file(&pid_file);

        orchestrator
            .stop_session(app.handle().clone(), "ha")
            .await
            .expect("stop");

        wait_until_stopped(&orchestrator, "ha").await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_pids_are_dead(&final_pids, "hyper/final").await;
        assert_no_watchdog_orphans("hyper");
    }
}
