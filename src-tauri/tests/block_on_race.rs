#![cfg(unix)]

use std::{sync::Arc, time::Duration};

/// Reproduces the EXACT execution model of the orchestrator's wait task:
/// std::thread::spawn + tokio runtime block_on, with Arc<Mutex<Child>>.
#[tokio::test]
async fn wait_task_via_block_on_completes_after_stop() {
    use std::os::unix::process::CommandExt;

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
    let (kill_tx, kill_rx) = tokio::sync::mpsc::channel::<()>(1);

    let child_clone = child.clone();
    let process_name = "test-service".to_string();

    // This mimics the orchestrator: thread::spawn + block_on
    let handle = std::thread::spawn(move || {
        let mut kill_rx = kill_rx;
        tauri::async_runtime::block_on(async move {
            let exit_status = {
                let mut child = child_clone.lock().await;
                tokio::select! {
                    result = child.wait() => {
                        eprintln!("[block_on] child.wait() branch won");
                        result
                    }
                    _ = kill_rx.recv() => {
                        eprintln!("[block_on] kill_rx.recv() branch won");
                        let kill_result = child.kill().await;
                        eprintln!("[block_on] child.kill() result: {:?}", kill_result);
                        match tokio::time::timeout(Duration::from_secs(10), child.wait()).await {
                            Ok(result) => {
                                eprintln!("[block_on] child.wait() after kill: {:?}", result);
                                result
                            }
                            Err(_) => {
                                eprintln!("[block_on] TIMEOUT waiting for child.wait()");
                                Err(std::io::Error::new(
                                    std::io::ErrorKind::TimedOut,
                                    "timeout",
                                ))
                            }
                        }
                    }
                }
            };
            eprintln!("[block_on] final exit_status: {:?}", exit_status);
            exit_status
        })
    });

    // Give the wait task time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Mimic begin_process_termination: SIGKILL to process group
    eprintln!("[test] sending SIGKILL to process group -{pid}");
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }

    // Mimic stop_process: send kill signal immediately
    eprintln!("[test] sending kill_tx signal");
    let _ = kill_tx.send(()).await;

    // Wait for the thread to complete
    let result = handle.join();
    match result {
        Ok(Ok(status)) => {
            assert!(
                !status.success(),
                "process should have been killed"
            );
            eprintln!("[test] PASSED: status = {status:?}");
        }
        Ok(Err(e)) => panic!("wait task returned error: {e}"),
        Err(panic) => panic!("wait task thread panicked: {:?}", panic),
    }
}

#[tokio::test]
async fn wait_task_block_on_repeated_20_times() {
    for i in 0..20 {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    run_single_stop_race_via_block_on().await
                })
        }));
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => panic!("Iteration {i} failed: {e}"),
            Err(_) => panic!("Iteration {i} panicked"),
        }
    }
}

async fn run_single_stop_race_via_block_on() -> Result<(), String> {
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
    let (kill_tx, kill_rx) = tokio::sync::mpsc::channel::<()>(1);

    let child_clone = child.clone();
    let handle = std::thread::spawn(move || {
        let mut kill_rx = kill_rx;
        tauri::async_runtime::block_on(async move {
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
        })
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
    let _ = kill_tx.send(()).await;

    let result = handle.join().map_err(|_| "thread panicked".to_string())?;
    result.map_err(|e| format!("io: {e}"))?;
    Ok(())
}
