use std::{
    collections::HashMap,
    io::{Read, Write},
    path::Path,
    sync::Arc,
    thread,
};

use tokio::sync::Mutex;

use chrono::Utc;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use tauri::{AppHandle, Emitter};

use crate::{
    application::events::{
        TerminalEvent, TerminalOutputEvent, TERMINAL_OUTPUT_EVENT, TERMINAL_SNAPSHOT_EVENT,
    },
    domain::terminal::{TerminalOutputPayload, TerminalSessionId, TerminalSnapshot},
    error::AppError,
    infrastructure::shell::user_shell_program,
};

#[derive(Clone)]
pub struct TerminalManager {
    inner: Arc<Mutex<HashMap<TerminalSessionId, ManagedTerminal>>>,
}

struct ManagedTerminal {
    window_key: String,
    snapshot: TerminalSnapshot,
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn open_terminal(
        &self,
        app_handle: AppHandle,
        window_key: String,
        title: String,
        cwd: &Path,
        cols: u16,
        rows: u16,
    ) -> Result<TerminalSnapshot, AppError> {
        let pty_system = native_pty_system();
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        let pair = pty_system
            .openpty(size)
            .map_err(|error| AppError::terminal(error.to_string()))?;

        let shell = user_shell_program();
        let mut command = CommandBuilder::new(shell);
        command.cwd(cwd);

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| AppError::terminal(error.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| AppError::terminal(error.to_string()))?;
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| AppError::terminal(error.to_string()))?;

        let terminal_id = TerminalSessionId::new();
        let snapshot = TerminalSnapshot {
            terminal_id: terminal_id.clone(),
            title,
            cwd: cwd.to_path_buf(),
            created_at: Utc::now(),
            is_open: true,
        };

        {
            let mut terminals = self.inner.lock().await;
            terminals.insert(
                terminal_id.clone(),
                ManagedTerminal {
                    window_key: window_key.clone(),
                    snapshot: snapshot.clone(),
                    master: pair.master,
                    writer,
                    child,
                },
            );
        }

        let output_terminal_id = terminal_id.clone();
        let output_app_handle = app_handle.clone();
        let output_window_key = window_key.clone();
        thread::spawn(move || {
            let mut buffer = [0_u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read_len) => {
                        let payload = TerminalOutputPayload {
                            terminal_id: output_terminal_id.clone(),
                            chunk: String::from_utf8_lossy(&buffer[..read_len]).into_owned(),
                            timestamp: Utc::now(),
                        };
                        let _ = output_app_handle.emit_to(
                            &output_window_key,
                            TERMINAL_OUTPUT_EVENT,
                            TerminalOutputEvent { payload },
                        );
                    }
                    Err(_) => break,
                }
            }
        });

        app_handle
            .emit_to(
                &window_key,
                TERMINAL_SNAPSHOT_EVENT,
                TerminalEvent {
                    snapshot: snapshot.clone(),
                },
            )
            .map_err(|error| AppError::terminal(error.to_string()))?;

        Ok(snapshot)
    }

    pub async fn write_terminal(
        &self,
        terminal_id: &TerminalSessionId,
        data: &str,
    ) -> Result<(), AppError> {
        let mut terminals = self.inner.lock().await;
        let terminal = terminals
            .get_mut(terminal_id)
            .ok_or_else(|| AppError::terminal("terminal not found"))?;
        terminal
            .writer
            .write_all(data.as_bytes())
            .map_err(|error| AppError::terminal(error.to_string()))?;
        terminal
            .writer
            .flush()
            .map_err(|error| AppError::terminal(error.to_string()))?;
        Ok(())
    }

    pub async fn resize_terminal(
        &self,
        terminal_id: &TerminalSessionId,
        cols: u16,
        rows: u16,
    ) -> Result<(), AppError> {
        let terminals = self.inner.lock().await;
        let terminal = terminals
            .get(terminal_id)
            .ok_or_else(|| AppError::terminal("terminal not found"))?;
        terminal
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| AppError::terminal(error.to_string()))?;
        Ok(())
    }

    pub async fn close_terminal(
        &self,
        app_handle: AppHandle,
        terminal_id: &TerminalSessionId,
    ) -> Result<Option<TerminalSnapshot>, AppError> {
        let maybe_closed = {
            let mut terminals = self.inner.lock().await;
            terminals.remove(terminal_id).map(|mut terminal| {
                let _ = terminal.child.kill();
                terminal.snapshot.is_open = false;
                (terminal.window_key, terminal.snapshot)
            })
        };

        if let Some((window_key, snapshot)) = maybe_closed.clone() {
            app_handle
                .emit_to(
                    &window_key,
                    TERMINAL_SNAPSHOT_EVENT,
                    TerminalEvent {
                        snapshot: snapshot.clone(),
                    },
                )
                .map_err(|error| AppError::terminal(error.to_string()))?;
        }

        Ok(maybe_closed.map(|(_, snapshot)| snapshot))
    }

    pub async fn close_all_for_window(
        &self,
        app_handle: AppHandle,
        window_key: &str,
    ) -> Result<(), AppError> {
        let ids = {
            let terminals = self.inner.lock().await;
            terminals
                .iter()
                .filter_map(|(id, terminal)| {
                    if terminal.window_key == window_key {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        for terminal_id in ids {
            let _ = self.close_terminal(app_handle.clone(), &terminal_id);
        }
        Ok(())
    }
}
