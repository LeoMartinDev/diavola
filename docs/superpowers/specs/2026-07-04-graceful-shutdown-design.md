# Graceful Multi-Platform Process Shutdown

**Status:** Approved
**Date:** 2026-07-04
**Scope:** Phase 1 — Reliability hardening (slice a: termination)

## Motivation

Diavola is a process supervisor; reliable shutdown is the core promise. Two
critical bugs break that promise today:

1. **No graceful phase.** `begin_process_termination`
   (`src-tauri/src/application/orchestrator/lifecycle.rs:55-74`) sends `SIGKILL`
   to the process group immediately. Servers that need cleanup (flush DB, close
   connections, run migration rollback) are killed net. Risk of DB corruption
   during a migration.

2. **Orphans on Windows.** `command_for_shell`
   (`src-tauri/src/infrastructure/shell.rs:6-11`) uses `cmd /C` with no process
   group or job object. `Child::kill` only kills `cmd.exe`, leaving grandchildren
   (`node`, the actual server) alive holding ports.

Unix is partially correct (process-group SIGKILL reaches the tree) but lacks a
graceful phase. Windows is wrong on both counts.

## Goals

- Send a graceful signal before force-killing, with a configurable grace window.
- Kill the entire process tree on Windows (no orphans).
- Make the forceful path guaranteed on both platforms.
- Keep the race-free termination design documented at
  `lifecycle.rs:44-54` intact.

## Non-Goals (Phase 2+)

- Crash recovery / PID tracking / orphan detection at startup.
- Permanent healthchecks, hot config reload, auto-restart on crash.
- Surfacing graceful-vs-forceful outcome to the UI (a future enhancement).

The Windows Job Object introduced here uses `JOB_OBJECT_LIMIT_KILL_ON_JOB`,
which *incidentally* lets the OS reap the tree if Diavola itself dies — this
benefits future crash recovery but is not built upon here.

## Config

### Schema (`src-tauri/src/domain/config.rs`)

Add an optional field at two levels:

```rust
// DiavolaConfig — global default:
#[serde(default)]
pub stop_timeout_ms: Option<u64>,

// ProcessConfig — per-process override:
#[serde(default)]
pub stop_timeout_ms: Option<u64>,
```

Both deserialize as milliseconds. Neither is required.

### Resolution

A pure function in `src-tauri/src/application/orchestrator/lifecycle.rs`:

```rust
pub const DEFAULT_STOP_TIMEOUT_MS: u64 = 10_000;

pub(super) fn resolve_stop_timeout(
    process: Option<u64>,
    global: Option<u64>,
) -> Duration {
    Duration::from_millis(process.or(global).unwrap_or(DEFAULT_STOP_TIMEOUT_MS))
}
```

Precedence: **process override > global > 10s default.**

### Validation (`src-tauri/src/infrastructure/config_loader.rs`)

In `validate_graph`, when `stop_timeout_ms` is `Some(v)` (global or per-process),
require `v >= 1000`. No upper bound (users may legitimately set long windows for
slow DB drains). Reject with `ConfigValidationFailed`.

## Termination Contract

The `kill_tx` channel's meaning changes from "force-kill now" to **"begin
supervised shutdown."** The wait task — which already holds the child mutex —
owns the timed escalation. This keeps the deadlock-aware design intact: no
new locking is introduced, and `child.wait()` during the grace window runs in
the task that already borrows `&mut Child`.

### `begin_process_termination`
(`src-tauri/src/application/orchestrator/lifecycle.rs`)

```
begin_process_termination(process):
  process.terminating = true
  if status in {Starting, Running, Ready}:
    process.snapshot.status = Stopping
    send GRACEFUL signal synchronously and immediately:   # race-free for the
      Unix:  libc::kill(-(pid as i32), libc::SIGTERM)      #   graceful phase
      Windows: GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT,  #   best-effort
                                        process_group_id)
    return process.kill_tx.take()        # meaning shifts to "supervised stop"
  else:
    return None     # terminal/pending states unchanged (existing behavior)
```

The synchronous graceful signal preserves the property that motivated the
original immediate-SIGKILL: by the time `begin_process_termination` returns,
the child has been signaled, so no "stuck at Stopping" window exists *for the
graceful phase*. The forceful escalation is the wait task's responsibility.

### Wait task
(`src-tauri/src/application/orchestrator/mod.rs`, the `tokio::select!` at lines
~451-499)

```
spawn wait task:
  let exit_status = tokio::select! {
    result = child.wait() => result,                  # natural exit

    _ = kill_rx.recv() => {                            # supervised stop
      # graceful signal already sent by begin_process_termination
      match tokio::time::timeout(grace, child.wait()).await {
        Ok(result) => result,                          # exited within grace
        Err(_) => {
          # FORCEFUL escalation after grace elapses
          Unix:  libc::kill(-(pid as i32), libc::SIGKILL)
          Windows: terminate the Job Object
          child.kill().await.ok();
          child.wait().await
        }
      }
    }
  }
  # then handle_process_exit / handle_process_failure as today
```

`grace` is the resolved `Duration` captured by the closure at spawn time
(read from the process config + global config). After escalation, the existing
`child.kill()` + `child.wait()` belt-and-suspenders remains.

### `stop_process`
(`src-tauri/src/application/orchestrator/mod.rs:147-191`)

Change the 10s hard cap (`mod.rs:187`) to `grace + 5s`, where `grace` is the
*same* `resolve_stop_timeout` value the wait task captured for this process.
The wait task guarantees escalation after `grace`, so `done_rx` *will* fire; the
+5s margin guards against unforeseen scheduler latency. Without this change, a
process configured with `stopTimeoutMs: 30000` would be falsely reported
"Stopped" at 10s.

```
let _ = tokio::time::timeout(grace + Duration::from_secs(5), done_rx).await;
```

### `finish_session`
(`src-tauri/src/application/orchestrator/mod.rs:675-736`)

Unchanged in structure: it still collects `kill_tx`s via
`begin_process_termination` and sends `()` to each. Each send now triggers a
*graceful* supervised shutdown per process. Sequential sends remain acceptable
(wait tasks poll `kill_rx` continuously and drain promptly). A single
`finish_session` does not impose its own session-wide timeout — each process's
own grace window governs its escalation.

## Platform Specifics

### Unix

Straightforward. Two line-level changes:

- `begin_process_termination`: `SIGKILL` -> `SIGTERM` to `-(pid)`.
- Wait task escalation branch: `SIGKILL` to `-(pid)` (the existing line, now on
  the timeout branch instead of the immediate-receive branch).

No new dependencies; `libc` is already in `Cargo.toml`.

### Windows

This is the real fix.

**New module `src-tauri/src/infrastructure/job.rs`:**

Wraps the Win32 Job Object API via `windows-sys` (add to `Cargo.toml`,
`[target.'cfg(windows)'.dependencies] windows-sys = { version = "0.59",
features = ["Win32_System_JobObjects", "Win32_System_Threading",
"Win32_Foundation"] }`). Exposes:

- `pub struct Job(Handle)` — owns a job created with
  `JOB_OBJECT_LIMIT_KILL_ON_JOB | JOB_OBJECT_LIMIT_BREAKAWAY_OK`.
- `pub fn assign_process(&self, process_handle: BorrowedHandle) -> io::Result<()>`
- `pub fn terminate(&self) -> io::Result<()>` — `TerminateJobObject`, kills the
  entire tree.
- `impl Drop`: closes the handle; with `KILL_ON_JOB`, the OS reaps the tree.

**Spawn path** (`src-tauri/src/application/command_runner.rs`):

On Windows only, change the spawn sequence:

`std::process::Command` does not expose the thread handle that
`CREATE_SUSPENDED` requires for `ResumeThread`, so the Windows spawn must call
`CreateProcessW` directly inside `job.rs` (the Unix path keeps using
`command_for_shell`/tokio unchanged). The Windows spawn sequence:

1. `CreateProcessW` with `CREATE_SUSPENDED | CREATE_NEW_PROCESS_GROUP`.
   `CREATE_NEW_PROCESS_GROUP` lets us target the group for
   `GenerateConsoleCtrlEvent`; `CREATE_SUSPENDED` closes the race where a
   grandchild could escape the job before assignment. `PROCESS_INFORMATION`
   yields both `hProcess` and `hThread`.
2. Create the `Job`, `AssignProcessToJobObject(hProcess)`.
3. `ResumeThread(hThread)`; close `hThread`.
4. Wrap `hProcess` into a tokio-compatible child handle for stdout/stderr piping
   (`Command`-based pipe setup is replaced by manual `CreatePipe` +
   `SetHandleInformation` + passing the write ends as `STARTUPINFO.hStdOutput`/
   `hStdError`). The read ends feed the existing log tasks unchanged.
5. Return `{ child-equivalent, stdout, stderr, job: Job }` in `SpawnedProcess`.

The pipe plumbing is the bulk of the Windows-specific code; it is contained in
`job.rs::spawn_suspended` and returns the same `SpawnedProcess` shape the
orchestrator already consumes, so `spawn_named_process` stays platform-agnostic.

If job creation or assignment fails, treat it as `ProcessStartFailed` — we
refuse to launch a process we cannot reliably clean up. If `ResumeThread` is
reached, the child is already in the job, so a crash between assign and resume
leaks a suspended (never-run) process — acceptable and rare.

**`ManagedProcess`** gains a `#[cfg(windows)] job: Option<Job>` field.
`begin_process_termination`'s graceful phase calls
`GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, process_group_id)`. The wait task's
forceful branch calls `job.terminate()`.

**Graceful caveat:** `GenerateConsoleCtrlEvent` is honored by console
applications that install a console-control handler (Node, Python, most servers).
Raw binaries ignore it. For those, the process simply runs out the grace window
and is force-killed via the Job Object. This is correct behavior — not graceful,
but guaranteed to terminate. The `cmd /C` wrapper still participates: `cmd.exe`
itself forwards ctrl events to its child by default.

## Frontend

Minimal additions; reuses existing patterns.

- `src/lib/types.ts`: add `stopTimeoutMs?: number` to the process config type
  and the global config type.
- `src/lib/config/validation.ts`: validate `stopTimeoutMs >= 1000` when present,
  mirroring backend.
- `src/lib/components/ProcessForm.svelte`: one optional number field
  `stopTimeoutMs` (ms). Reuses `TextField`.
- `src/lib/components/config/ConfigGeneralSection.svelte`: one optional number
  field for the global `stopTimeoutMs` default.
- `src/lib/components/config/ConfigProcessDetail.svelte`: render the field.

No changes to the runtime store or event payloads — `stop_timeout_ms` is read
by the orchestrator at spawn time and never travels back to the UI.

## Testing

### Backend unit (`#[cfg(test)]`)

- `lifecycle.rs`: existing `begin_process_termination_*` tests still hold
  (Running/Starting/Ready -> Stopping, `kill_tx` taken, terminal states
  untouched). Update assertions that cared about the specific signal sent.
- New: `resolve_stop_timeout(process, global)` precedence — process wins over
  global, global wins over default, default is 10s.
- New: `config_loader::validate_graph` rejects `stop_timeout_ms < 1000` at both
  levels; accepts valid values and absent fields.
- New: `config_loader` parses `stopTimeoutMs` in global and per-process
  positions, round-trips through `DiavolaConfig`.

### Backend integration (`src-tauri/tests/session_lifecycle.rs`)

- **Graceful-then-force escalation (Unix):** spawn
  `sh -c "trap '' TERM; sleep 60"`, configured `stop_timeout_ms: 1000`. Stop.
  Assert the process reaches a terminal status within ~1.5s — proves the
  SIGTERM was ignored and SIGKILL escalation fired on schedule (rather than the
  process surviving 60s).
- **Graceful exit during grace:** spawn `sh -c "trap 'exit 0' TERM; sleep 60"`,
  `stop_timeout_ms: 5000`. Stop. Assert exit within the grace window — proves
  SIGTERM is honored and escalation is *not* triggered.
- Existing `task_succeeds_and_session_stops` and `process_failure_stops_session`
  remain green.

  Both new tests carry the existing
  `#[cfg_attr(target_os = "linux", ignore = "Tauri GTK event loop ...")]` guard.

### Windows integration

- Spawn `cmd /C "node -e \"setTimeout(()=>{},60000)\""` holding a known port.
  Stop. Assert the port is freed (the previous bug: `cmd.exe` killed, node
  orphaned, port held). Marked for the Windows CI matrix; tolerates
  `GenerateConsoleCtrlEvent` semantics.

### Frontend

- `ConfigEditor.test.ts` + `ProcessForm.test.ts`: the new field renders,
  validates `>= 1000`, and round-trips through save.

## Risk and Mitigations

- **`CREATE_SUSPENDED` resume race:** assigning the job before resume is the
  standard pattern; if the suspended child is orphaned by a crash between spawn
  and resume, it leaks as suspended. Acceptable (rare; the process never ran).
  Mitigation: keep the assign+resume window minimal and synchronous.
- **`GenerateConsoleCtrlEvent` unreliability:** documented; the Job Object
  forceful path is the correctness backstop.
- **Grace window too long for session teardown:** each process escalates
  independently on its own timer, so a 30s-configured process only blocks its
  own cleanup, not faster siblings.

## Out of Scope

- Crash recovery (PID files, startup orphan detection, session resume).
- Auto-restart policies, permanent healthchecks, hot config reload.
- Distinguishing graceful-exit vs forceful-exit in the UI/snapshot.
- Non-Windows/Unix platforms (none targeted).
