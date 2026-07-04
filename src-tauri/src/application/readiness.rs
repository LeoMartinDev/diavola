use std::{collections::HashMap, path::Path, time::Duration};

use tokio::{
    sync::broadcast,
    time::{sleep, Instant},
};

use crate::{
    domain::config::{CommandReadyConfig, ReadyConfig},
    error::{AppError, ErrorCode},
    infrastructure::{
        ready_checks::{DEFAULT_READY_INTERVAL_MS, DEFAULT_READY_TIMEOUT_MS},
        shell::command_for_shell,
    },
};

pub async fn wait_until_ready(
    ready: &ReadyConfig,
    base_dir: &Path,
    env: &HashMap<String, String>,
    log_rx: Option<broadcast::Receiver<String>>,
) -> Result<(), AppError> {
    match ready {
        ReadyConfig::Delay(config) => {
            sleep(Duration::from_millis(config.duration_ms)).await;
            Ok(())
        }
        ReadyConfig::Http(config) => {
            let deadline = Instant::now()
                + Duration::from_millis(config.timeout_ms.unwrap_or(DEFAULT_READY_TIMEOUT_MS));
            let interval =
                Duration::from_millis(config.interval_ms.unwrap_or(DEFAULT_READY_INTERVAL_MS));
            loop {
                if Instant::now() > deadline {
                    return Err(AppError::runtime_with_code(
                        format!("http readiness timed out for {}", config.url),
                        crate::error::ErrorCode::ReadinessTimeout,
                    ));
                }
                if let Ok(response) = reqwest::get(&config.url).await {
                    let status = response.status();
                    if (200..=299).contains(&status.as_u16()) {
                        return Ok(());
                    }
                }
                sleep(interval).await;
            }
        }
        ReadyConfig::Log(config) => {
            let timeout =
                Duration::from_millis(config.timeout_ms.unwrap_or(DEFAULT_READY_TIMEOUT_MS));
            let deadline = Instant::now() + timeout;
            let mut receiver = log_rx.ok_or_else(|| {
                AppError::runtime_with_code(
                    "log readiness requires an active log subscription",
                    ErrorCode::ReadinessCheckFailed,
                )
            })?;
            let matcher = if config.regex {
                Some(regex::Regex::new(&config.pattern).map_err(|error| {
                    AppError::validation(format!(
                        "invalid readiness regex `{}`: {error}",
                        config.pattern
                    ))
                })?)
            } else {
                None
            };

            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(AppError::runtime_with_code(
                        format!(
                            "log readiness timed out waiting for `{}`",
                            config.pattern
                        ),
                        ErrorCode::ReadinessTimeout,
                    ));
                }
                let line = tokio::time::timeout(remaining, receiver.recv())
                    .await
                    .map_err(|_| {
                        AppError::runtime_with_code(
                            format!(
                                "log readiness timed out waiting for `{}`",
                                config.pattern
                            ),
                            ErrorCode::ReadinessTimeout,
                        )
                    })?
                    .map_err(|error| AppError::runtime_with_code(error.to_string(), ErrorCode::ReadinessCheckFailed))?;
                if let Some(regex) = &matcher {
                    if regex.is_match(&line) {
                        return Ok(());
                    }
                } else if line.contains(&config.pattern) {
                    return Ok(());
                }
            }
        }
        ReadyConfig::Command(config) => wait_until_command_ready(config, base_dir, env).await,
    }
}

async fn wait_until_command_ready(
    config: &CommandReadyConfig,
    base_dir: &Path,
    env: &HashMap<String, String>,
) -> Result<(), AppError> {
    let deadline = Instant::now()
        + Duration::from_millis(config.timeout_ms.unwrap_or(DEFAULT_READY_TIMEOUT_MS));
    let interval = Duration::from_millis(config.interval_ms.unwrap_or(DEFAULT_READY_INTERVAL_MS));

    loop {
        if Instant::now() > deadline {
            return Err(AppError::runtime_with_code(
                format!("command readiness timed out for `{}`", config.cmd),
                ErrorCode::ReadinessTimeout,
            ));
        }

        let mut command = command_for_shell(&config.cmd);
        command.current_dir(base_dir);
        command.envs(env.iter());
        let status = command.status().await.map_err(|error| {
            AppError::runtime_with_code(
                format!("failed to run readiness command: {error}"),
                ErrorCode::ReadinessCheckFailed,
            )
        })?;
        if status.success() {
            return Ok(());
        }

        sleep(interval).await;
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::Path};

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::broadcast,
        time::{timeout, Duration},
    };

    use super::*;
    use crate::domain::config::{
        CommandReadyConfig, DelayReadyConfig, HttpReadyConfig, LogReadyConfig,
    };

    async fn serve_once(status_code: u16) -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let url = format!("http://{}", listener.local_addr().expect("local addr"));
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept request");
            let mut buffer = [0; 1024];
            let _ = socket.read(&mut buffer).await;
            let response = format!(
                "HTTP/1.1 {status_code} Test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });
        url
    }

    #[tokio::test]
    async fn http_readiness_accepts_large_2xx_status() {
        let url = serve_once(299).await;
        let ready = ReadyConfig::Http(HttpReadyConfig {
            url,
            interval_ms: Some(5),
            timeout_ms: Some(250),
        });

        wait_until_ready(&ready, Path::new("."), &HashMap::new(), None)
            .await
            .expect("299 should be ready");
    }

    #[tokio::test]
    async fn log_readiness_matches_plain_pattern() {
        let (tx, rx) = broadcast::channel(4);
        let ready = ReadyConfig::Log(LogReadyConfig {
            pattern: "server ready".to_string(),
            regex: false,
            timeout_ms: Some(250),
        });

        tokio::spawn(async move {
            tx.send("listening: server ready".to_string())
                .expect("send readiness line");
        });

        wait_until_ready(&ready, Path::new("."), &HashMap::new(), Some(rx))
            .await
            .expect("log pattern should be ready");
    }

    #[tokio::test]
    async fn log_readiness_matches_regex_pattern() {
        let (tx, rx) = broadcast::channel(4);
        let ready = ReadyConfig::Log(LogReadyConfig {
            pattern: r"listening on \d+".to_string(),
            regex: true,
            timeout_ms: Some(250),
        });

        tokio::spawn(async move {
            tx.send("listening on 5173".to_string())
                .expect("send readiness line");
        });

        wait_until_ready(&ready, Path::new("."), &HashMap::new(), Some(rx))
            .await
            .expect("regex pattern should be ready");
    }

    #[tokio::test]
    async fn delay_readiness_waits_for_configured_duration() {
        let ready = ReadyConfig::Delay(DelayReadyConfig { duration_ms: 25 });

        timeout(
            Duration::from_millis(250),
            wait_until_ready(&ready, Path::new("."), &HashMap::new(), None),
        )
        .await
        .expect("delay should finish before timeout")
        .expect("delay should be ready");
    }

    #[tokio::test]
    async fn command_readiness_uses_environment_and_base_dir() {
        let base_dir =
            std::env::temp_dir().join(format!("trame-readiness-command-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create temp base dir");
        std::fs::write(base_dir.join("marker.txt"), "ready").expect("write marker");
        let mut env = HashMap::new();
        env.insert("EXPECTED_READY".to_string(), "ready".to_string());
        let ready = ReadyConfig::Command(CommandReadyConfig {
            cmd: "test -f marker.txt && test \"$EXPECTED_READY\" = ready".to_string(),
            interval_ms: Some(5),
            timeout_ms: Some(250),
        });

        wait_until_ready(&ready, &base_dir, &env, None)
            .await
            .expect("command should be ready");

        let _ = std::fs::remove_dir_all(base_dir);
    }
}
