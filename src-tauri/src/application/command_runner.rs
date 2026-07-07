use std::{collections::HashMap, path::Path};
#[cfg(windows)]
use std::sync::Arc;

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdout, Command},
};

use crate::{
    error::{AppError, ErrorCode},
};

#[cfg(windows)]
use crate::infrastructure::shell::command_for_shell;

pub struct SpawnedProcess {
    pub child: Child,
    pub stdout: ChildStdout,
    pub stderr: ChildStderr,
    #[cfg(windows)]
    pub job: Option<Arc<crate::infrastructure::job::Job>>,
}

pub fn spawn_process(
    cmd: &str,
    current_dir: &Path,
    env: &HashMap<String, String>,
) -> Result<SpawnedProcess, AppError> {
    // On Unix the command is launched indirectly through the watchdog (see
    // `src/watchdog.rs`) so the whole process tree is guaranteed to die if
    // Diavola is killed or crashes. On Windows the equivalent guarantee comes
    // from Job Objects assigned below, so the command is launched directly.
    #[cfg(unix)]
    let mut command = {
        use std::os::unix::process::CommandExt as _;

        let exe = std::env::current_exe().map_err(|error| {
            AppError::runtime_with_code(
                format!("failed to resolve current exe for watchdog: {error}"),
                ErrorCode::ProcessStartFailed,
            )
        })?;
        let parent_pid = std::process::id();
        let mut command = Command::new(exe);
        command.arg("--diavola-watchdog");
        command.arg("--parent-pid");
        command.arg(parent_pid.to_string());
        command.arg("--cmd");
        command.arg(cmd);
        // The watchdog becomes its own process-group leader; its `sh` child and
        // all descendants stay in that group, so `kill(-watchdog_pid)` reaches
        // the whole tree.
        command.as_std_mut().arg0("diavola-watchdog");
        command.as_std_mut().process_group(0);
        command
    };
    #[cfg(windows)]
    let mut command = command_for_shell(cmd);

    command.current_dir(current_dir);
    command.envs(env.iter());
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.as_std_mut().creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .map_err(|error| AppError::runtime_with_code(format!("failed to spawn `{cmd}`: {error}"), ErrorCode::ProcessStartFailed))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::runtime_with_code("missing child stdout pipe", ErrorCode::ProcessStartFailed))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::runtime_with_code("missing child stderr pipe", ErrorCode::ProcessStartFailed))?;

    #[cfg(windows)]
    let job = {
        let pid = child.id().ok_or_else(|| {
            AppError::runtime_with_code("missing child pid for job assignment", ErrorCode::ProcessStartFailed)
        })?;
        let job = crate::infrastructure::job::Job::new()?;
        job.assign_pid(pid)
            .map_err(|error| {
                AppError::runtime_with_code(
                    format!("failed to assign job: {error}"),
                    ErrorCode::ProcessStartFailed,
                )
            })?;
        Some(Arc::new(job))
    };

    Ok(SpawnedProcess {
        child,
        stdout,
        stderr,
        #[cfg(windows)]
        job,
    })
}

pub async fn read_lines(
    reader: impl tokio::io::AsyncRead + Unpin,
) -> Result<Vec<String>, AppError> {
    let mut lines = BufReader::new(reader).lines();
    let mut output = Vec::new();
    while let Some(line) = lines.next_line().await? {
        output.push(line);
    }
    Ok(output)
}
