use std::{collections::HashMap, path::Path};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdout},
};

use crate::{
    error::{AppError, ErrorCode},
    infrastructure::shell::command_for_shell,
};

pub struct SpawnedProcess {
    pub child: Child,
    pub stdout: ChildStdout,
    pub stderr: ChildStderr,
}

pub fn spawn_process(
    cmd: &str,
    current_dir: &Path,
    env: &HashMap<String, String>,
) -> Result<SpawnedProcess, AppError> {
    let mut command = command_for_shell(cmd);
    command.current_dir(current_dir);
    command.envs(env.iter());
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

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

    Ok(SpawnedProcess {
        child,
        stdout,
        stderr,
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
