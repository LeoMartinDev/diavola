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
) where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    F: Fn(ProcessLogPayload) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = log_tx.send(line.clone());
            let payload = ProcessLogPayload {
                session_id: session_id.clone(),
                runtime_id: runtime_id.clone(),
                process_name: process_name.clone(),
                stream,
                line,
                timestamp: Utc::now(),
            };
            append_log_fn(payload.clone()).await;
            let _ =
                app_handle.emit_to(&window_key, PROCESS_LOG_EVENT, ProcessLogEvent { payload });
        }
    });
}
