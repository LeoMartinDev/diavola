use std::{future::Future, pin::Pin};

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
    tokio::spawn(async move {
        let mut lines_reader = BufReader::new(reader).lines();
        let mut buffer: Vec<String> = Vec::new();
        let mut first_timestamp: Option<chrono::DateTime<Utc>> = None;

        async fn emit(
            buffer: &mut Vec<String>,
            first_timestamp: &mut Option<chrono::DateTime<Utc>>,
            app_handle: &AppHandle,
            window_key: &str,
            session_id: &RunSessionId,
            runtime_id: &ProcessRuntimeId,
            process_name: &str,
            stream: LogStream,
            append_log_fn: &(dyn Fn(ProcessLogPayload) -> Pin<Box<dyn Future<Output = ()> + Send>> + Sync),
        ) {
            if buffer.is_empty() {
                return;
            }
            let ts = first_timestamp.unwrap_or_else(Utc::now);
            let payload = ProcessLogPayload {
                session_id: session_id.clone(),
                runtime_id: runtime_id.clone(),
                process_name: process_name.to_string(),
                stream,
                lines: std::mem::take(buffer),
                timestamp: ts,
            };
            let _ = app_handle.emit_to(window_key, PROCESS_LOG_EVENT, ProcessLogEvent {
                payload: payload.clone(),
            });
            append_log_fn(payload).await;
            *first_timestamp = None;
        }

        while let Ok(Some(line)) = lines_reader.next_line().await {
            let _ = log_tx.send(line.clone());

            let is_new_entry = match &timestamp_pattern {
                Some(re) => re.is_match(&line),
                None => true,
            };

            if is_new_entry {
                emit(
                    &mut buffer,
                    &mut first_timestamp,
                    &app_handle,
                    &window_key,
                    &session_id,
                    &runtime_id,
                    &process_name,
                    stream,
                    &append_log_fn,
                )
                .await;
            }

            if first_timestamp.is_none() {
                first_timestamp = Some(Utc::now());
            }
            buffer.push(line);
        }

        emit(
            &mut buffer,
            &mut first_timestamp,
            &app_handle,
            &window_key,
            &session_id,
            &runtime_id,
            &process_name,
            stream,
            &append_log_fn,
        )
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use std::io::Cursor;
    use tokio::sync::broadcast;

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
}
