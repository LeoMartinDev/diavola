//! Reproduces the exact stop_process race: SIGKILL sent to process group,
//! then kill_tx.send(()) immediately after (no delay), mimicking the
//! orchestrator's begin_process_termination + stop_process flow.

#![cfg(unix)]

use std::{sync::Arc, time::Duration};

#[tokio::test]
async fn stop_race_kill_rx_fires_before_child_exits() {
    use std::os::unix::process::CommandExt;

    // Spawn a long-running process in its own process group (like the watchdog)
    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c").arg("sleep 999");
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    unsafe {
        cmd.as_std_mut().process_group(0);
    }

    let mut child = cmd.spawn().expect("spawn");
    let pid = child.id().expect("pid");

    let child = Arc::new(tokio::sync::Mutex::new(child));
    let (kill_tx, mut kill_rx) = tokio::sync::mpsc::channel::<()>(1);

    // --- This mimics EXACTLY what the orchestrator does ---

    // 1. begin_process_termination: SIGKILL to process group
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }

    // 2. Take kill_tx (like begin_process_termination does)
    let kill_tx = kill_tx;

    let child_clone = child.clone();
    let wait_handle = tokio::spawn(async move {
        let exit_status = {
            let mut child = child_clone.lock().await;
            tokio::select! {
                result = child.wait() => {
                    eprintln!("[wait] child.wait() branch won");
                    result
                }
                _ = kill_rx.recv() => {
                    eprintln!("[wait] kill_rx.recv() branch won");
                    let _ = child.kill().await;
                    eprintln!("[wait] child.kill() completed");
                    match tokio::time::timeout(Duration::from_secs(10), child.wait()).await {
                        Ok(result) => {
                            eprintln!("[wait] child.wait() after kill returned Ok");
                            result
                        }
                        Err(_) => {
                            eprintln!("[wait] TIMEOUT after kill");
                            Err(std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "process did not exit within kill timeout",
                            ))
                        }
                    }
                }
            }
        };
        eprintln!("[wait] exit_status = {:?}", exit_status);
        exit_status
    });

    // 3. stop_process: send kill signal IMMEDIATELY (no delay)
    let _ = kill_tx.send(()).await;

    let result = tokio::time::timeout(Duration::from_secs(15), wait_handle).await;

    match result {
        Ok(Ok(Ok(status))) => {
            assert!(
                !status.success(),
                "process should have been killed, got success status"
            );
            eprintln!("[test] PASSED: process exited with {status:?}");
        }
        Ok(Ok(Err(e))) => panic!("wait_task child.wait() returned error: {e}"),
        Ok(Err(_)) => panic!("wait_task panicked or was cancelled"),
        Err(_) => panic!("wait_task did not complete within 15s — STUCK AT STOPPING"),
    }
}

#[tokio::test]
async fn stop_race_repeat_100_times() {
    for i in 0..100 {
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            stop_race_single(),
        ).await;

        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => panic!("Iteration {i} failed: {e}"),
            Err(_) => panic!("Iteration {i} timed out — STUCK AT STOPPING"),
        }
    }
}

async fn stop_race_single() -> Result<(), String> {
    use std::os::unix::process::CommandExt;

    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c").arg("sleep 999");
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    unsafe {
        cmd.as_std_mut().process_group(0);
    }

    let mut child = cmd.spawn().map_err(|e| format!("spawn: {e}"))?;
    let pid = child.id().ok_or("no pid")?;

    let child = Arc::new(tokio::sync::Mutex::new(child));
    let (kill_tx, mut kill_rx) = tokio::sync::mpsc::channel::<()>(1);

    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }

    let child_clone = child.clone();
    let wait_handle = tokio::spawn(async move {
        let exit_status = {
            let mut child = child_clone.lock().await;
            tokio::select! {
                result = child.wait() => result,
                _ = kill_rx.recv() => {
                    let _ = child.kill().await;
                    tokio::time::timeout(Duration::from_secs(10), child.wait())
                        .await
                        .unwrap_or(Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "timeout",
                        )))
                }
            }
        };
        exit_status
    });

    let _ = kill_tx.send(()).await;

    let result = tokio::time::timeout(Duration::from_secs(10), wait_handle)
        .await
        .map_err(|_| "wait_task timed out".to_string())?;

    result
        .map_err(|e| format!("wait returned err: {e}"))?
        .map_err(|e| format!("io error: {e}"))?;

    Ok(())
}
