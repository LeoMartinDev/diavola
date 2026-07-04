use std::io;

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    ConfigNotFound,
    ConfigParseFailed,
    ConfigValidationFailed,

    ConfigDependencyCycle,
    ConfigUnknownProcess,
    ProjectNotFound,
    ProjectAlreadyRunning,
    LaunchLocked,
    ProcessNotFound,
    ProcessStartFailed,
    ProcessCannotRestart,
    ReadinessTimeout,
    ReadinessCheckFailed,
    IoError,
    TerminalError,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("config error: {0}")]
    Config(String, ErrorCode),
    #[error("io error: {0}")]
    Io(String, ErrorCode),
    #[error("validation error: {0}")]
    Validation(String, ErrorCode),
    #[error("runtime error: {0}")]
    Runtime(String, ErrorCode),
    #[error("project store error: {0}")]
    ProjectStore(String, ErrorCode),
    #[error("terminal error: {0}")]
    Terminal(String, ErrorCode),
    #[error("project is launch-locked and cannot be modified")]
    LaunchLocked,
}

impl AppError {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into(), ErrorCode::ConfigParseFailed)
    }

    pub fn config_with_code(msg: impl Into<String>, code: ErrorCode) -> Self {
        Self::Config(msg.into(), code)
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into(), ErrorCode::ConfigValidationFailed)
    }

    pub fn validation_with_code(msg: impl Into<String>, code: ErrorCode) -> Self {
        Self::Validation(msg.into(), code)
    }

    pub fn runtime(msg: impl Into<String>) -> Self {
        Self::Runtime(msg.into(), ErrorCode::ProcessStartFailed)
    }

    pub fn runtime_with_code(msg: impl Into<String>, code: ErrorCode) -> Self {
        Self::Runtime(msg.into(), code)
    }

    pub fn project_store(msg: impl Into<String>) -> Self {
        Self::ProjectStore(msg.into(), ErrorCode::ProjectNotFound)
    }

    pub fn project_store_with_code(msg: impl Into<String>, code: ErrorCode) -> Self {
        Self::ProjectStore(msg.into(), code)
    }

    pub fn terminal(msg: impl Into<String>) -> Self {
        Self::Terminal(msg.into(), ErrorCode::TerminalError)
    }

    pub fn terminal_with_code(msg: impl Into<String>, code: ErrorCode) -> Self {
        Self::Terminal(msg.into(), code)
    }

    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Config(_, c)
            | Self::Io(_, c)
            | Self::Validation(_, c)
            | Self::Runtime(_, c)
            | Self::ProjectStore(_, c)
            | Self::Terminal(_, c) => *c,
            Self::LaunchLocked => ErrorCode::LaunchLocked,
        }
    }
}

impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self {
        Self::Io(e.to_string(), ErrorCode::IoError)
    }
}

impl From<serde_yaml::Error> for AppError {
    fn from(e: serde_yaml::Error) -> Self {
        Self::Config(e.to_string(), ErrorCode::ConfigParseFailed)
    }
}

impl From<AppError> for String {
    fn from(v: AppError) -> Self {
        v.to_string()
    }
}
