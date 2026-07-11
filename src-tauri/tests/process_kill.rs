//! Verifies that tokio's Child::wait() completes after an external
//! SIGKILL is sent to the process group — the mechanism that
//! `begin_process_termination` relies on.

use std::time::Duration;

#[cfg(unix)]
#[tokio::test]
async fn child_wait_returns_after_external_sigkill_to_process_group() {
    use std::os::unix::process::CommandExt;

    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c").arg("sleep 999");
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    cmd.as_std_mut().process_group(0);

    let mut child = cmd.spawn().expect("spawn");
    let pid = child.id().expect("pid");

    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }

    let result = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;

    match result {
        Ok(Ok(status)) => {
            assert!(
                !status.success(),
                "process should have been killed, got success"
            );
        }
        Ok(Err(e)) => panic!("child.wait() returned error: {e}"),
        Err(_) => panic!("child.wait() did not return within 5s after SIGKILL"),
    }
}

#[cfg(unix)]
#[tokio::test]
async fn child_wait_returns_after_select_kill_branch() {
    use std::os::unix::process::CommandExt;

    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c").arg("sleep 999");
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    cmd.as_std_mut().process_group(0);

    let child = cmd.spawn().expect("spawn");
    let pid = child.id().expect("pid");

    let (kill_tx, mut kill_rx) = tokio::sync::mpsc::channel::<()>(1);

    let child = std::sync::Arc::new(tokio::sync::Mutex::new(child));
    let child_clone = child.clone();

    let wait_task = tokio::spawn(async move {
        let exit_status = {
            let mut child = child_clone.lock().await;
            tokio::select! {
                result = child.wait() => result,
                _ = kill_rx.recv() => {
                    let _ = child.kill().await;
                    match tokio::time::timeout(Duration::from_secs(10), child.wait()).await {
                        Ok(result) => result,
                        Err(_) => Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "process did not exit within kill timeout",
                        )),
                    }
                }
            }
        };
        exit_status
    });

    // Send SIGKILL to process group (like begin_process_termination)
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }

    // Then send kill signal (like stop_process does after begin_process_termination)
    tokio::time::sleep(Duration::from_millis(50)).await;
    let _ = kill_tx.send(()).await;

    let result = tokio::time::timeout(Duration::from_secs(5), wait_task).await;
    match result {
        Ok(Ok(Ok(status))) => {
            assert!(
                !status.success(),
                "process should have been killed, got success"
            );
        }
        Ok(Ok(Err(e))) => panic!("wait_task child.wait() returned error: {e}"),
        Ok(Err(_)) => panic!("wait_task panicked or was cancelled"),
        Err(_) => panic!("wait_task did not complete within 5s"),
    }
}
