//! Self-re-exec watchdog that guarantees a supervised command dies when Diavola
//! dies — including when Diavola is `kill -9`'d or hard-crashes, which normal
//! signal handling cannot intercept.
//!
//! Windows already solves this with Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`
//! makes the kernel kill the whole tree when Diavola dies). Unix has no such
//! primitive, so on Unix `spawn_process` does NOT launch `sh -c <cmd>` directly.
//! Instead it re-execs the Diavola binary in watchdog mode:
//!
//! ```text
//! <diavola> --diavola-watchdog --parent-pid <PID> --cmd <SHELL_CMD>
//! ```
//!
//! The watchdog is spawned in its own process group (`process_group(0)`), so the
//! watchdog's pid == the process-group id. It runs `sh -c <SHELL_CMD>` as its
//! own child in that same group, relaying stdin/stdout/stderr by inheritance.
//!
//!   * When the command exits, the watchdog exits with its code.
//!   * When the parent (Diavola) exits, the watchdog is reparented to init / a
//!     subreaper, so `getppid()` changes. The watchdog notices and kills the
//!     whole process group, reaching `sh` and all descendants that did not
//!     `setsid()` into a new session (a fundamental limitation).
//!
//! Parent-death detection uses `getppid()` polling rather than kqueue/pidfd/
//! `prctl(PR_SET_PDEATHSIG)`: those primitives are either restricted (macOS
//! `EVFILT_PROC` returns ESRCH for processes outside one's own session) or
//! racy, while polling is identical on Linux and macOS, race-free, and its
//! ~100 ms latency is irrelevant for cleanup.

#![cfg(unix)]

use std::{process::Command, thread, time::Duration};

struct WatchdogArgs {
    parent_pid: i32,
    cmd: String,
}

fn parse_args(args: &[String]) -> Option<WatchdogArgs> {
    let mut parent_pid = None;
    let mut cmd = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--parent-pid" => {
                parent_pid = args.get(i + 1).and_then(|v| v.parse().ok());
                i += 2;
            }
            "--cmd" => {
                cmd = args.get(i + 1).cloned();
                i += 2;
            }
            _ => i += 1,
        }
    }
    Some(WatchdogArgs {
        parent_pid: parent_pid?,
        cmd: cmd?,
    })
}

/// Kills the watchdog's own process group with `SIGKILL`. Since the watchdog is
/// the process-group leader, this reaches the supervised `sh`, the command it
/// runs, and any descendants that stayed in the group. The watchdog itself dies
/// in this call, so any code after it is best-effort.
fn kill_process_group() {
    unsafe {
        let pgrp = libc::getpgrp();
        let _ = libc::killpg(pgrp, libc::SIGKILL);
    }
}

/// The graceful-stop `SIGTERM` Diavola sends to the process group is meant for
/// the supervised command, not for this supervisor. If the watchdog died on
/// `SIGTERM`, a stubborn command that ignores it would be orphaned: the
/// orchestrator would observe the watchdog's exit and never reach its
/// grace→`SIGKILL` escalation. Ignoring `SIGTERM` keeps the watchdog alive
/// until the command exits or Diavola escalates to `SIGKILL` (which cannot be
/// caught or ignored).
unsafe fn ignore_sigterm() {
    let mut sa: libc::sigaction = std::mem::zeroed();
    sa.sa_sigaction = libc::SIG_IGN;
    libc::sigemptyset(&mut sa.sa_mask);
    libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
}

const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Entry point invoked from `main` when `argv[1] == "--diavola-watchdog"`.
/// `args` is everything after the `--diavola-watchdog` token.
pub fn run(args: &[String]) -> i32 {
    let Some(parsed) = parse_args(args) else {
        return 2;
    };

    unsafe { ignore_sigterm() };

    let child = match Command::new("sh").arg("-c").arg(&parsed.cmd).spawn() {
        Ok(c) => c,
        Err(_) => return 1,
    };
    let child_pid = child.id() as i32;
    // The child is reaped via raw `waitpid` in `supervise`; forget the std
    // handle so its destructor does not fight us for the child.
    std::mem::forget(child);

    supervise(parsed.parent_pid, child_pid)
}

fn supervise(parent_pid: i32, child_pid: i32) -> i32 {
    loop {
        // Parent death: we were reparented to init / a subreaper. This also
        // covers the spawn race (Diavola died between fork and this check).
        if unsafe { libc::getppid() } != parent_pid {
            kill_process_group();
            let mut status: libc::c_int = 0;
            unsafe {
                libc::waitpid(child_pid, &mut status, libc::WNOHANG);
            }
            return 0;
        }

        let mut status: libc::c_int = 0;
        let reaped = unsafe { libc::waitpid(child_pid, &mut status, libc::WNOHANG) };
        if reaped == child_pid {
            return if libc::WIFEXITED(status) {
                libc::WEXITSTATUS(status)
            } else {
                1
            };
        }

        thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::parse_args;

    #[test]
    fn parses_parent_pid_and_cmd() {
        let args = [
            "--parent-pid".to_string(),
            "4242".to_string(),
            "--cmd".to_string(),
            "echo hi".to_string(),
        ];
        let parsed = parse_args(&args).expect("parsed");
        assert_eq!(parsed.parent_pid, 4242);
        assert_eq!(parsed.cmd, "echo hi");
    }

    #[test]
    fn returns_none_when_required_args_missing() {
        assert!(parse_args(&["--parent-pid".to_string(), "1".to_string()]).is_none());
        assert!(parse_args(&["--cmd".to_string(), "x".to_string()]).is_none());
        let empty: Vec<String> = Vec::new();
        assert!(parse_args(&empty).is_none());
    }

    #[test]
    fn ignores_unknown_flags() {
        let args = [
            "--bogus".to_string(),
            "--parent-pid".to_string(),
            "7".to_string(),
            "--cmd".to_string(),
            "run".to_string(),
        ];
        let parsed = parse_args(&args).expect("parsed");
        assert_eq!(parsed.parent_pid, 7);
        assert_eq!(parsed.cmd, "run");
    }
}
