#![cfg(windows)]

use std::{io, ptr};

use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
    System::Console::{
        AttachConsole, GenerateConsoleCtrlEvent, FreeConsole, CTRL_BREAK_EVENT,
    },
    System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
        TerminateJobObject, JobObjectExtendedLimitInformation,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOBOBJECT_BASIC_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    },
    System::Threading::OpenProcess,
};

use crate::error::AppError;

const PROCESS_SET_QUOTA: u32 = 0x0100;
const PROCESS_TERMINATE: u32 = 0x0001;

pub struct Job {
    handle: HANDLE,
}

impl Job {
    pub fn new() -> Result<Self, AppError> {
        unsafe {
            let handle = CreateJobObjectW(ptr::null(), ptr::null());
            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                return Err(AppError::runtime_with_code(
                    "CreateJobObjectW failed",
                    crate::error::ErrorCode::ProcessStartFailed,
                ));
            }
            let mut basic: JOBOBJECT_BASIC_LIMIT_INFORMATION = std::mem::zeroed();
            basic.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let mut extended: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            extended.BasicLimitInformation = basic;
            let result = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &extended as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if result == 0 {
                CloseHandle(handle);
                return Err(AppError::runtime_with_code(
                    "SetInformationJobObject failed",
                    crate::error::ErrorCode::ProcessStartFailed,
                ));
            }
            Ok(Self { handle })
        }
    }

    /// Assign a running child (by its OS process id) to this job. Opens a fresh
    /// handle with the rights `AssignProcessToJobObject` requires.
    pub fn assign_pid(&self, pid: u32) -> Result<(), AppError> {
        unsafe {
            let process = OpenProcess(
                PROCESS_SET_QUOTA | PROCESS_TERMINATE,
                0,
                pid,
            );
            if process.is_null() || process == INVALID_HANDLE_VALUE {
                return Err(AppError::runtime_with_code(
                    format!("OpenProcess({pid}) failed"),
                    crate::error::ErrorCode::ProcessStartFailed,
                ));
            }
            let result = AssignProcessToJobObject(self.handle, process);
            CloseHandle(process);
            if result == 0 {
                return Err(AppError::runtime_with_code(
                    "AssignProcessToJobObject failed",
                    crate::error::ErrorCode::ProcessStartFailed,
                ));
            }
            Ok(())
        }
    }

    /// Forcefully terminate every process in the job. Guaranteed to kill the
    /// whole tree by construction.
    pub fn terminate(&self) -> io::Result<()> {
        unsafe {
            if TerminateJobObject(self.handle, 1) == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle) };
    }
}

unsafe impl Send for Job {}
unsafe impl Sync for Job {}

/// Best-effort graceful signal. Attaches the calling thread to the target
/// process group's console and sends CTRL_BREAK. Many servers (Node, Python)
/// install a handler and shut down; raw binaries ignore it and the grace
/// window falls through to `Job::terminate`. Failure is logged by the caller,
/// not fatal.
pub fn send_ctrl_break(process_group_id: u32) {
    unsafe {
        if AttachConsole(process_group_id) == 0 {
            return;
        }
        let _ = GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, process_group_id);
        FreeConsole();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_can_be_created_and_terminated() {
        let job = Job::new().expect("create job");
        job.terminate().expect("terminate empty job");
    }

    #[test]
    fn send_ctrl_break_does_not_panic_without_console() {
        send_ctrl_break(0xDEAD_BEEF);
    }
}
