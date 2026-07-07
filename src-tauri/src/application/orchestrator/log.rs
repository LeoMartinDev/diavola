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

fn make_log_payload(
    session_id: &RunSessionId,
    runtime_id: &ProcessRuntimeId,
    process_name: &str,
    stream: LogStream,
    line: String,
) -> ProcessLogPayload {
    ProcessLogPayload {
        session_id: session_id.clone(),
        runtime_id: runtime_id.clone(),
        process_name: process_name.to_string(),
        stream,
        lines: vec![line],
        timestamp: Utc::now(),
    }
}

async fn append_reader_lines<R, F>(
    reader: R,
    stream: LogStream,
    session_id: &RunSessionId,
    runtime_id: &ProcessRuntimeId,
    process_name: &str,
    timestamp_pattern: Option<regex::Regex>,
    append_line_fn: &F,
) -> std::io::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    F: Fn(String, ProcessLogPayload) -> Pin<Box<dyn Future<Output = ()> + Send>>,
{
    let _ = timestamp_pattern;

    let mut lines_reader = BufReader::new(reader).lines();

    while let Some(line) = lines_reader.next_line().await? {
        let payload = make_log_payload(
            session_id,
            runtime_id,
            process_name,
            stream,
            line.clone(),
        );
        append_line_fn(line, payload).await;
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
    timestamp_pattern: Option<regex::Regex>,
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
            timestamp_pattern,
            &move |line, payload| {
                let app_handle = app_handle.clone();
                let window_key = window_key.clone();
                let log_tx = log_tx.clone();
                let append_log_fn = append_log_fn.clone();
                Box::pin(async move {
                    let _ = log_tx.send(line);
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
    use tokio::{io::AsyncWriteExt, sync::Mutex, time::{timeout, Duration}};

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

    #[test]
    fn make_log_payload_wraps_one_physical_line() {
        let session_id = RunSessionId::new();
        let runtime_id = ProcessRuntimeId::new();

        let payload = make_log_payload(
            &session_id,
            &runtime_id,
            "api",
            LogStream::Stdout,
            r#"statusCode: 500"#.to_string(),
        );

        assert_eq!(payload.session_id, session_id);
        assert_eq!(payload.runtime_id, runtime_id);
        assert_eq!(payload.process_name, "api");
        assert_eq!(payload.stream, LogStream::Stdout);
        assert_eq!(payload.lines, vec![r#"statusCode: 500"#]);
    }

    #[tokio::test]
    async fn non_timestamp_lines_are_emitted_without_waiting_for_next_timestamp() {
        let session_id = RunSessionId::new();
        let runtime_id = ProcessRuntimeId::new();
        let appended = Arc::new(Mutex::new(Vec::<ProcessLogPayload>::new()));
        let appended_for_fn = appended.clone();

        let append_fn =
            move |_line: String,
                  payload: ProcessLogPayload|
                  -> Pin<Box<dyn Future<Output = ()> + Send>> {
                let appended = appended_for_fn.clone();
                Box::pin(async move {
                    appended.lock().await.push(payload);
                })
            };

        let (mut writer, reader) = tokio::io::duplex(1024);
        let appended_for_writer = appended.clone();

        let writer_task = tokio::spawn(async move {
            writer.write_all(b"method: \"GET\"\n").await.unwrap();

            timeout(Duration::from_secs(1), async {
                loop {
                    if appended_for_writer.lock().await.len() == 1 {
                        break;
                    }

                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("first line should be emitted immediately");

            for line in [
                r#"clientRelease: "DEV""#,
                r#"currentFiscalYearConfiguration: {"#,
                r#""year": 2025"#,
                r#"}"#,
                r#"statusCode: 500"#,
            ] {
                writer.write_all(line.as_bytes()).await.unwrap();
                writer.write_all(b"\n").await.unwrap();
            }

            drop(writer);
        });

        timeout(
            Duration::from_secs(1),
            append_reader_lines(
                reader,
                LogStream::Stdout,
                &session_id,
                &runtime_id,
                "api",
                Some(Regex::new(r"^\[\d{2}:\d{2}:\d{2}\]").unwrap()),
                &append_fn,
            ),
        )
        .await
        .expect("reader should finish")
        .unwrap();

        writer_task.await.unwrap();

        let payloads = appended.lock().await;
        assert_eq!(payloads.len(), 6);
        assert_eq!(payloads[0].lines, vec![r#"method: "GET""#]);
        assert_eq!(payloads[5].lines, vec![r#"statusCode: 500"#]);
    }
}
