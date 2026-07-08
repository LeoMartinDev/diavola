use std::{future::Future, pin::Pin, sync::Arc};

use chrono::Utc;
use tauri::{AppHandle, Emitter};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::broadcast,
};

use crate::{
    application::events::{ProcessLogEvent, PROCESS_LOG_EVENT},
    domain::{
        process::LogStream,
        runtime::{ProcessLogPayload, ProcessRuntimeId, RunSessionId},
    },
};

async fn append_reader_lines<R, F>(
    reader: R,
    stream: LogStream,
    session_id: &RunSessionId,
    runtime_id: &ProcessRuntimeId,
    process_name: &str,
    entry_pattern: Option<regex::Regex>,
    append_line_fn: &F,
) -> std::io::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    F: Fn(ProcessLogPayload) -> Pin<Box<dyn Future<Output = ()> + Send>>,
{
    let mut lines_reader = BufReader::new(reader).lines();
    let mut buffer: Vec<String> = Vec::new();
    let mut first_timestamp: Option<chrono::DateTime<Utc>> = None;

    while let Some(line) = lines_reader.next_line().await? {
        let is_new_entry = entry_pattern
            .as_ref()
            .map(|re| re.is_match(&line))
            .unwrap_or(true);

        if is_new_entry && !buffer.is_empty() {
            let payload = ProcessLogPayload {
                session_id: session_id.clone(),
                runtime_id: runtime_id.clone(),
                process_name: process_name.to_string(),
                stream,
                lines: std::mem::take(&mut buffer),
                timestamp: first_timestamp.take().unwrap_or_else(Utc::now),
            };
            append_line_fn(payload).await;
        }

        if is_new_entry {
            first_timestamp = Some(Utc::now());
        } else if first_timestamp.is_none() {
            first_timestamp = Some(Utc::now());
        }
        buffer.push(line);
    }

    if !buffer.is_empty() {
        let payload = ProcessLogPayload {
            session_id: session_id.clone(),
            runtime_id: runtime_id.clone(),
            process_name: process_name.to_string(),
            stream,
            lines: std::mem::take(&mut buffer),
            timestamp: first_timestamp.take().unwrap_or_else(Utc::now),
        };
        append_line_fn(payload).await;
    }

    Ok(())
}

pub(super) fn spawn_log_task<R, F>(
    app_handle: AppHandle,
    window_key: String,
    session_id: RunSessionId,
    process_name: String,
    runtime_id: ProcessRuntimeId,
    stream: LogStream,
    reader: R,
    log_tx: broadcast::Sender<String>,
    append_log_fn: F,
    entry_pattern: Option<regex::Regex>,
) where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    F: Fn(ProcessLogPayload) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static,
{
    let append_log_fn = Arc::new(append_log_fn);

    tokio::spawn(async move {
        let _ = append_reader_lines(
            reader,
            stream,
            &session_id,
            &runtime_id,
            &process_name,
            entry_pattern,
            &move |payload| {
                let app_handle = app_handle.clone();
                let window_key = window_key.clone();
                let log_tx = log_tx.clone();
                let append_log_fn = append_log_fn.clone();
                let readiness_lines = payload.lines.clone();
                Box::pin(async move {
                    for line in readiness_lines {
                        let _ = log_tx.send(line);
                    }
                    let _ = app_handle.emit_to(
                        &window_key,
                        PROCESS_LOG_EVENT,
                        ProcessLogEvent {
                            payload: payload.clone(),
                        },
                    );
                    append_log_fn(payload).await;
                })
            },
        )
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use std::{future::Future, pin::Pin, sync::Arc};
    use tokio::{io::AsyncWriteExt, sync::Mutex};

    #[test]
    fn pattern_detects_timestamp() {
        let re = Regex::new(r"^\d{4}-\d{2}-\d{2}").unwrap();
        assert!(re.is_match("2026-07-06 12:00:00 INFO starting"));
        assert!(!re.is_match("  at com.example.Main.main(Main.java:42)"));
    }

    #[test]
    fn empty_pattern_means_no_grouping() {
        let re: Option<Regex> = None;
        assert!(re.is_none());
    }

    #[tokio::test]
    async fn no_pattern_emits_each_line_individually() {
        let session_id = RunSessionId::new();
        let runtime_id = ProcessRuntimeId::new();
        let appended = Arc::new(Mutex::new(Vec::<ProcessLogPayload>::new()));
        let appended_for_fn = appended.clone();

        let append_fn =
            move |payload: ProcessLogPayload| -> Pin<Box<dyn Future<Output = ()> + Send>> {
                let appended = appended_for_fn.clone();
                Box::pin(async move {
                    appended.lock().await.push(payload);
                })
            };

        let (mut writer, reader) = tokio::io::duplex(1024);

        let writer_task = tokio::spawn(async move {
            for line in ["line one", "line two", "line three"] {
                writer.write_all(line.as_bytes()).await.unwrap();
                writer.write_all(b"\n").await.unwrap();
            }
            drop(writer);
        });

        append_reader_lines(
            reader,
            LogStream::Stdout,
            &session_id,
            &runtime_id,
            "api",
            None,
            &append_fn,
        )
        .await
        .unwrap();

        writer_task.await.unwrap();

        let payloads = appended.lock().await;
        assert_eq!(payloads.len(), 3);
        assert_eq!(payloads[0].lines, vec!["line one"]);
        assert_eq!(payloads[1].lines, vec!["line two"]);
        assert_eq!(payloads[2].lines, vec!["line three"]);
    }

    #[tokio::test]
    async fn lines_matching_entry_pattern_start_new_entries() {
        let session_id = RunSessionId::new();
        let runtime_id = ProcessRuntimeId::new();
        let appended = Arc::new(Mutex::new(Vec::<ProcessLogPayload>::new()));
        let appended_for_fn = appended.clone();

        let append_fn =
            move |payload: ProcessLogPayload| -> Pin<Box<dyn Future<Output = ()> + Send>> {
                let appended = appended_for_fn.clone();
                Box::pin(async move {
                    appended.lock().await.push(payload);
                })
            };

        let (mut writer, reader) = tokio::io::duplex(1024);

        let writer_task = tokio::spawn(async move {
            for line in [
                "[12:00:01] INFO  Starting application...",
                "  Initializing database connection pool",
                "  Loading configuration from /etc/app/config.yml",
                "[12:00:02] INFO  Application started successfully",
                "  Server listening on 0.0.0.0:3000",
            ] {
                writer.write_all(line.as_bytes()).await.unwrap();
                writer.write_all(b"\n").await.unwrap();
            }
            drop(writer);
        });

        append_reader_lines(
            reader,
            LogStream::Stdout,
            &session_id,
            &runtime_id,
            "api",
            Some(Regex::new(r"^\[\d{2}:\d{2}:\d{2}\]").unwrap()),
            &append_fn,
        )
        .await
        .unwrap();

        writer_task.await.unwrap();

        let payloads = appended.lock().await;
        assert_eq!(payloads.len(), 2);
        assert_eq!(payloads[0].lines.len(), 3);
        assert_eq!(
            payloads[0].lines[0],
            "[12:00:01] INFO  Starting application..."
        );
        assert_eq!(
            payloads[0].lines[1],
            "  Initializing database connection pool"
        );
        assert_eq!(
            payloads[0].lines[2],
            "  Loading configuration from /etc/app/config.yml"
        );
        assert_eq!(payloads[1].lines.len(), 2);
        assert_eq!(
            payloads[1].lines[0],
            "[12:00:02] INFO  Application started successfully"
        );
        assert_eq!(
            payloads[1].lines[1],
            "  Server listening on 0.0.0.0:3000"
        );
    }

    #[tokio::test]
    async fn non_timestamp_lines_buffered_until_next_timestamp_or_eof() {
        let session_id = RunSessionId::new();
        let runtime_id = ProcessRuntimeId::new();
        let appended = Arc::new(Mutex::new(Vec::<ProcessLogPayload>::new()));
        let appended_for_fn = appended.clone();

        let append_fn =
            move |payload: ProcessLogPayload| -> Pin<Box<dyn Future<Output = ()> + Send>> {
                let appended = appended_for_fn.clone();
                Box::pin(async move {
                    appended.lock().await.push(payload);
                })
            };

        let (mut writer, reader) = tokio::io::duplex(1024);

        let writer_task = tokio::spawn(async move {
            for line in [
                r#"method: "GET""#,
                r#"clientRelease: "DEV""#,
                r#"statusCode: 500"#,
            ] {
                writer.write_all(line.as_bytes()).await.unwrap();
                writer.write_all(b"\n").await.unwrap();
            }
            drop(writer);
        });

        append_reader_lines(
            reader,
            LogStream::Stdout,
            &session_id,
            &runtime_id,
            "api",
            Some(Regex::new(r"^\[\d{2}:\d{2}:\d{2}\]").unwrap()),
            &append_fn,
        )
        .await
        .unwrap();

        writer_task.await.unwrap();

        let payloads = appended.lock().await;
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].lines.len(), 3);
        assert_eq!(payloads[0].lines[0], r#"method: "GET""#);
        assert_eq!(payloads[0].lines[1], r#"clientRelease: "DEV""#);
        assert_eq!(payloads[0].lines[2], r#"statusCode: 500"#);
    }
}
