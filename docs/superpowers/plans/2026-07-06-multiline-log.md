# Multi-line Log Rendering & One-click Copy — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Merge consecutive log lines into logical entries using a configurable timestamp regex, render them as compact visual groups, and add per-entry copy buttons.

**Architecture:** The Rust backend buffers lines and emits merged `ProcessLogPayload` with `lines: Vec<String>`. The Svelte frontend flattens each payload into `FlatRow[]` (one row per physical line with grouping metadata). The LogViewer renders each FlatRow, visually grouping same-entry rows and showing a copy button on hover.

**Tech Stack:** Rust (Tauri v2, Tokio, regex), Svelte 5 runes, Tailwind CSS v4, Vitest + Testing Library

## Global Constraints

- `MAX_LOG_LINES_PER_PROCESS = 10_000` applies to FlatRow count (not entries)
- Virtual scroll stays fixed-height (ROW_HEIGHT=22px) on FlatRow array
- When no `logTimestampPattern` configured → fallback to current behavior (every line = one entry)
- Copy strips ANSI codes before writing to clipboard
- `diavola.yml` schema: `logTimestampPattern` at root (global) and per-process (override)

---

## File Structure

| File | Change |
|------|--------|
| `src-tauri/src/domain/config.rs` | Modify: add `log_timestamp_pattern` fields |
| `src-tauri/src/domain/runtime.rs` | Modify: `line` → `lines` |
| `src-tauri/src/application/orchestrator/log.rs` | Modify: buffer lines, emit merged payloads |
| `src-tauri/src/application/orchestrator/mod.rs` | Modify: pass timestamp regex to `spawn_log_task` |
| `src-tauri/src/application/readiness.rs` | Modify: iterate `lines` when checking log readiness |
| `src-tauri/src/infrastructure/log_store.rs` | Modify: trait/impl to match new payload |
| `src/lib/types.ts` | Modify: `ProcessLogPayload`, add `FlatRow` |
| `src/lib/config/editorModel.ts` | Modify: add `logTimestampPattern` fields |
| `src/lib/stores/runtime.svelte.ts` | Modify: flatten payloads into FlatRow, track truncation |
| `src/lib/components/LogViewer.svelte` | Modify: FlatRow rendering, copy button, border logic |
| `src/lib/components/LogViewer.test.ts` | Modify: update test helpers for FlatRow |
| `src/lib/components/LogToolbar.test.ts` | Modify: update test helpers for FlatRow |

---

### Task 1: Config types — add `logTimestampPattern` to Rust & TypeScript

**Files:**
- Modify: `src-tauri/src/domain/config.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/config/editorModel.ts`

**Interfaces:**
- Produces: `DiavolaConfig.log_timestamp_pattern: Option<String>`, `ProcessConfig.log_timestamp_pattern: Option<String>` (Rust)
- Produces: `DiavolaConfig.logTimestampPattern?: string`, `ProcessConfig.logTimestampPattern?: string` (TS)
- Produces: `ConfigFormState.globalLogTimestampPattern: string`, `ProcessForm.logTimestampPattern: string` (editor)

- [ ] **Step 1: Add fields to Rust config**

```rust
// src-tauri/src/domain/config.rs — DiavolaConfig
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiavolaConfig {
    #[serde(default)]
    pub env: IndexMap<String, String>,
    #[serde(default)]
    pub grace_period_ms: Option<u64>,
    #[serde(default)]
    pub log_timestamp_pattern: Option<String>,
    pub processes: IndexMap<String, ProcessConfig>,
}

// ProcessConfig — add field
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessConfig {
    pub kind: ProcessKind,
    pub cmd: String,
    #[serde(default)]
    pub env: IndexMap<String, String>,
    #[serde(default, rename = "dependsOn")]
    pub depends_on: IndexMap<String, DependencyCondition>,
    #[serde(default)]
    pub ready: Option<ReadyConfig>,
    #[serde(default)]
    pub grace_period_ms: Option<u64>,
    #[serde(default)]
    pub log_timestamp_pattern: Option<String>,
}
```

- [ ] **Step 2: Add fields to TypeScript types**

```ts
// src/lib/types.ts — DiavolaConfig
export type DiavolaConfig = {
  env?: Record<string, string>;
  gracePeriodMs?: number;
  logTimestampPattern?: string;
  processes: Record<string, ProcessConfig>;
};

// ProcessConfig
export type ProcessConfig = {
  kind: ProcessKind;
  cmd: string;
  env: Record<string, string>;
  dependsOn: Record<string, DependencyCondition>;
  ready?: ReadyConfig;
  gracePeriodMs?: number;
  logTimestampPattern?: string;
};
```

- [ ] **Step 3: Update editor model**

```ts
// src/lib/config/editorModel.ts

// ConfigFormState — add field
export type ConfigFormState = {
  globalEnvRows: EnvRow[];
  globalGracePeriodMs: number | string | null;
  globalLogTimestampPattern: string;
  processes: ProcessForm[];
};

// ProcessForm — add field
export type ProcessForm = {
  // ... existing fields ...
  logTimestampPattern: string;
};

// createProcess — add default
export function createProcess(name = "api", nextId: IdFactory = defaultNextId): ProcessForm {
  return {
    // ... existing fields ...
    logTimestampPattern: "",    // empty = inherit global or fallback to single-line
  };
}

// toProcessForm — extract from config
export function toProcessForm(name: string, config: ProcessConfig, nextId: IdFactory = defaultNextId): ProcessForm {
  // ... existing returns ...
  return {
    ...createProcess(name, nextId),
    // ... existing fields ...
    logTimestampPattern: config.logTimestampPattern ?? "",
  };
}

// buildConfig — include in output
export function buildConfig(form: ConfigFormState): DiavolaConfig {
  // ... existing ...
  return {
    env: Object.keys(globalEnv).length > 0 ? globalEnv : undefined,
    gracePeriodMs: globalGracePeriodMs,
    logTimestampPattern: form.globalLogTimestampPattern || undefined,
    processes: Object.fromEntries(processEntries),
  };
}

// buildProcessConfig — include per-process
export function buildProcessConfig(process: ProcessForm): ProcessConfig {
  return {
    // ... existing ...
    ready: process.readyEnabled ? buildReadyConfig(process) : undefined,
    gracePeriodMs,
    logTimestampPattern: process.logTimestampPattern || undefined,
  };
}

// serializeConfig — add logTimestampPattern output
export function serializeConfig(config: DiavolaConfig) {
  // ... after gracePeriodMs output ...
  if (config.logTimestampPattern) {
    lines.push(`logTimestampPattern: ${yamlScalar(config.logTimestampPattern)}`);
  }
  // ... in process block, before "ready:" ...
  if (process.logTimestampPattern) {
    lines.push(`    logTimestampPattern: ${yamlScalar(process.logTimestampPattern)}`);
  }
}
```

- [ ] **Step 4: Compile check**

Run: `deno task build`
Expected: compiles (config fields may need updating in test helpers — fix any compile errors)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/domain/config.rs src/lib/types.ts src/lib/config/editorModel.ts
git commit -m "feat(config): add logTimestampPattern to config schema"
```

---

### Task 2: Rust — ProcessLogPayload `line` → `lines`

**Files:**
- Modify: `src-tauri/src/domain/runtime.rs:64`
- Modify: `src-tauri/src/infrastructure/log_store.rs` (trait + impl remain same, field access changes)

**Interfaces:**
- Changes: `ProcessLogPayload { line: String }` → `ProcessLogPayload { lines: Vec<String> }`

- [ ] **Step 1: Change domain type**

```rust
// src-tauri/src/domain/runtime.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessLogPayload {
    pub session_id: RunSessionId,
    pub runtime_id: ProcessRuntimeId,
    pub process_name: String,
    pub stream: LogStream,
    pub lines: Vec<String>,
    pub timestamp: DateTime<Utc>,
}
```

- [ ] **Step 2: Fix log_store compile errors** (field name change propagates, no logic change needed — `LogStore` trait stores `ProcessLogPayload`, impl is generic over field names)

Run: `cargo check` in `src-tauri/`
Expected: compile errors only in files we haven't updated yet (log.rs, readiness.rs, mod.rs orchestrator tests)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/domain/runtime.rs
git commit -m "refactor(runtime): change ProcessLogPayload.line to lines: Vec<String>"
```

---

### Task 3: Rust — Buffer lines in `log_task`

**Files:**
- Modify: `src-tauri/src/application/orchestrator/log.rs`
- Modify: `src-tauri/src/application/orchestrator/mod.rs` (pass regex to spawn_log_task)

**Interfaces:**
- Consumes: `ProcessLogPayload { lines: Vec<String> }`
- Produces: `spawn_log_task` now accepts `Option<regex::Regex>` for timestamp detection

- [ ] **Step 1: Add regex dependency to Cargo.toml if not already present**

Check `src-tauri/Cargo.toml` for `regex` crate (it's used in readiness.rs). The crate is already present.

- [ ] **Step 2: Rewrite `log.rs` with buffering**

```rust
// src-tauri/src/application/orchestrator/log.rs

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
    F: Fn(ProcessLogPayload) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines_reader = BufReader::new(reader).lines();
        let mut buffer: Vec<String> = Vec::new();
        let mut first_timestamp: Option<chrono::DateTime<Utc>> = None;

        fn emit(
            buffer: &mut Vec<String>,
            first_timestamp: &mut Option<chrono::DateTime<Utc>>,
            app_handle: &AppHandle,
            window_key: &str,
            session_id: &RunSessionId,
            runtime_id: &ProcessRuntimeId,
            process_name: &str,
            stream: LogStream,
            log_tx: &broadcast::Sender<String>,
            append_log_fn: &(dyn Fn(ProcessLogPayload) -> Pin<Box<dyn Future<Output = ()> + Send>>),
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
            let _ = append_log_fn(payload);
            *first_timestamp = None;
        }

        while let Ok(Some(line)) = lines_reader.next_line().await {
            let _ = log_tx.send(line.clone());

            let is_new_entry = match &timestamp_pattern {
                Some(re) => re.is_match(&line),
                None => true, // no pattern → every line is a new entry
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
                    &log_tx,
                    &append_log_fn,
                );
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
            &log_tx,
            &append_log_fn,
        );
    });
}
```

- [ ] **Step 3: Update orchestrator to compile and pass regex**

In `src-tauri/src/application/orchestrator/mod.rs`, the `spawn_named_process` function calls `spawn_log_task` twice (stdout, stderr). We need to resolve the timestamp regex from config and pass it.

Before `spawn_log_task` calls, add regex resolution:

```rust
// in spawn_named_process, after the let generation = { ... } block, before log::spawn_log_task:

let timestamp_pattern = {
    let state = self.inner.lock().await;
    state
        .sessions
        .get(window_key)
        .and_then(|active| {
            let process_config = active.processes.get(&*process_name)?;
            let pattern = process_config
                .config
                .log_timestamp_pattern
                .as_deref()
                .or(active.loaded_config.config.log_timestamp_pattern.as_deref());
            pattern.and_then(|p| regex::Regex::new(p).ok())
        })
        .flatten()
};
```

Both `spawn_log_task` calls need the extra parameter: add `, timestamp_pattern.clone()` as the last argument before the closing paren.

Actually, the orchestrator code currently gets process config at the top of `spawn_named_process` and the config is used later. The regex creation can fail silently (invalid regex = no pattern = fallback to single-line). Let me look at the existing code more carefully...

The orchestrator clones `config` at line 334 and 438. We can get the pattern from there. Let me adjust:

```rust
// After line 334 (config clone), add:
let timestamp_regex = config
    .log_timestamp_pattern
    .as_deref()
    .or(global_config.log_timestamp_pattern.as_deref())  // need to pass this
    .and_then(|p| regex::Regex::new(p).ok());
```

But `global_config` isn't directly available. We need to resolve the effective pattern. Let me look at how to get the global config...

Looking at `spawn_named_process`, the global grace period has `global_grace_period_ms` captured. We need to similarly capture the global log pattern.

Actually, let me look at what's available. The `spawn_named_process` function destructures from the lock:

```rust
let (session_id, _, base_dir, env, config, runtime_id, log_tx, global_grace_period_ms) = {
    // ...
};
```

The global config pattern isn't captured here. I need to add it. Let me look at what the loaded config looks like...

From the orchestrator mod.rs, `active.loaded_config.config` is `DiavolaConfig`. So `active.loaded_config.config.log_timestamp_pattern` would be the global one.

Let me add it to the destructure:

```rust
let (session_id, _, base_dir, env, config, runtime_id, log_tx, global_grace_period_ms, global_log_timestamp_pattern) = {
    // ...
    (
        session_id,
        active.project.name.clone(),
        active.loaded_config.base_dir.clone(),
        env,
        process.config.clone(),
        process.snapshot.runtime_id.clone(),
        process.log_tx.clone(),
        global_grace_period_ms,
        active.loaded_config.config.log_timestamp_pattern.clone(),
    )
};
```

Then:

```rust
let timestamp_pattern = config
    .log_timestamp_pattern
    .as_deref()
    .or(global_log_timestamp_pattern.as_deref())
    .and_then(|p| regex::Regex::new(p).ok());
```

And pass `timestamp_pattern` to both `spawn_log_task` calls.

Let me write this more precisely for the plan.

- [ ] **Step 3: Update `spawn_named_process` to capture and pass regex**

In `src-tauri/src/application/orchestrator/mod.rs`:

**3a.** Add `global_log_timestamp_pattern` to the destructure tuple (around line 300):

```rust
// Change the destructure from:
let (session_id, _, base_dir, env, config, runtime_id, log_tx, global_grace_period_ms) = {
// To:
let (session_id, _, base_dir, env, config, runtime_id, log_tx, global_grace_period_ms, global_log_timestamp_pattern) = {
```

**3b.** Add the field to the returned tuple (around line 329):

```rust
// Add as the last element:
active.loaded_config.config.log_timestamp_pattern.clone(),
```

**3c.** After the destructure (around line 340), add regex resolution:

```rust
let timestamp_pattern = config
    .log_timestamp_pattern
    .as_deref()
    .or(global_log_timestamp_pattern.as_deref())
    .and_then(|p| regex::Regex::new(p).ok());
```

**3d.** Update both `spawn_log_task` calls to pass `timestamp_pattern.clone()`:

The first (stdout, around line 400):
```rust
log::spawn_log_task(
    app_handle.clone(),
    window_key.to_string(),
    session_id.clone(),
    process_name.to_string(),
    runtime_id.clone(),
    LogStream::Stdout,
    spawned.stdout,
    log_tx.clone(),
    append_fn,
    timestamp_pattern.clone(),
);
```

The second (stderr, around line 426):
```rust
log::spawn_log_task(
    app_handle.clone(),
    window_key.to_string(),
    session_id.clone(),
    process_name.to_string(),
    runtime_id.clone(),
    LogStream::Stderr,
    spawned.stderr,
    log_tx.clone(),
    append_fn,
    timestamp_pattern.clone(),
);
```

- [ ] **Step 4: Compile check**

```bash
cd src-tauri && cargo check 2>&1
```
Expected: no compile errors. Fix any type mismatches.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/orchestrator/log.rs src-tauri/src/application/orchestrator/mod.rs
git commit -m "feat(log): buffer lines with timestamp regex detection"
```

---

### Task 4: Rust — Readiness checker iterate `lines`

**Files:**
- Modify: `src-tauri/src/application/readiness.rs:49-101`

**Interfaces:**
- Consumes: `ProcessLogPayload { lines: Vec<String> }` (broadcast channel still sends individual lines)
- Change: the broadcast channel sends individual lines (unchanged), readiness receives individual lines (unchanged). No change needed!

Wait — the broadcast channel `log_tx.send(line.clone())` still sends individual lines (see new log.rs). So the readiness checker receives lines one at a time, exactly as before. **No changes needed to readiness.rs.**

- [ ] **Step 1: Verify no change needed**

Run: `cd src-tauri && cargo check 2>&1`
Expected: `readiness.rs` compiles without errors (it only uses the broadcast receiver, not `ProcessLogPayload`)

---

### Task 5: TypeScript types — `ProcessLogPayload` + `FlatRow`

**Files:**
- Modify: `src/lib/types.ts`

**Interfaces:**
- Changes: `ProcessLogPayload.line: string` → `ProcessLogPayload.lines: string[]`
- Produces: `FlatRow` type for the flattened model

- [ ] **Step 1: Update ProcessLogPayload and add FlatRow**

```ts
// src/lib/types.ts — replace line 123
export type ProcessLogPayload = {
  sessionId: RunSessionId;
  runtimeId: ProcessRuntimeId;
  processName: string;
  stream: LogStream;
  lines: string[];
  timestamp: string;
};

// After ProcessLogPayload, add:
export type FlatRow = {
  entryId: number;
  lineIndex: number;
  isFirstLine: boolean;
  isContinuation: boolean;
  text: string;
  stream: LogStream;
  timestamp: string;
};
```

- [ ] **Step 2: Fix any TypeScript compile errors**

Run: `deno task build`
Expected: compile errors only in files that still use `entry.line` (store, LogViewer, tests). That's expected — we fix them in subsequent tasks.

- [ ] **Step 3: Commit**

```bash
git add src/lib/types.ts
git commit -m "refactor(types): ProcessLogPayload.line -> lines, add FlatRow"
```

---

### Task 6: Frontend runtime store — flatten payloads

**Files:**
- Modify: `src/lib/stores/runtime.svelte.ts`

**Interfaces:**
- Consumes: `ProcessLogPayload { lines: string[] }`
- Produces: `processLogs: Record<string, FlatRow[]>`, `logsForSelectedProcess()` returns `FlatRow[]`
- Changes internal state type from `Record<string, ProcessLogPayload[]>` to `Record<string, FlatRow[]>`

- [ ] **Step 1: Update imports and state type**

```ts
// src/lib/stores/runtime.svelte.ts
// Change import:
import type {
  // ... existing imports ...
  FlatRow,          // add
  ProcessLogEvent,
  ProcessLogPayload, // keep
  // ...
} from "$lib/types";

// Change state declaration (line 64):
processLogs = $state<Record<string, FlatRow[]>>({});
```

- [ ] **Step 2: Update the process-log event listener**

```ts
// Replace the event listener at lines 558-571:

let entryCounter = 0;

function flattenPayload(payload: ProcessLogPayload): FlatRow[] {
  const entryId = entryCounter++;
  return payload.lines.map((text, i) => ({
    entryId,
    lineIndex: i,
    isFirstLine: i === 0,
    isContinuation: i > 0,
    text,
    stream: payload.stream,
    timestamp: i === 0 ? payload.timestamp : '',
  }));
}

// In #attachEventListeners, replace the processLog listener:
this.#unlisteners.push(
  await listen<ProcessLogEvent>(TAURI_EVENTS.processLog, (event) => {
    const payload = event.payload.payload;
    if (this.session && payload.sessionId !== this.session.sessionId) {
      return;
    }
    const runtimeId = payload.runtimeId;
    const current = this.processLogs[runtimeId] ?? [];
    const newRows = flattenPayload(payload);
    const appended = [...current, ...newRows];
    const overflow = Math.max(0, appended.length - MAX_LOG_LINES_PER_PROCESS);
    this.processLogs[runtimeId] = overflow > 0 ? appended.slice(overflow) : appended;
    this.processLogTruncation[runtimeId] =
      (this.processLogTruncation[runtimeId] ?? 0) + overflow;
  }),
);
```

- [ ] **Step 3: Update `logsForSelectedProcess()` return type**

```ts
logsForSelectedProcess(): FlatRow[] {
  return this.selectedProcessRuntimeId
    ? (this.processLogs[this.selectedProcessRuntimeId] ?? [])
    : [];
}
```

- [ ] **Step 4: Update `truncatedLogCountForSelectedProcess()` — keep "lines" in the banner text for user readability**

```ts
// The banner text currently says "X older lines hidden" — keep it.
// No change needed here since overflow counts FlatRow items (physical lines)
// and "lines" is what users understand.
```

- [ ] **Step 5: Clear on new session preserves type**

```ts
// In startCurrentProject() (line 354):
this.processLogs = {};   // Record<string, FlatRow[]> — unchanged, valid
```

- [ ] **Step 6: Compile check**

Run: `deno task build`
Expected: still errors in LogViewer.svelte and test files (they still reference `entry.line`). Proceed to next task.

- [ ] **Step 7: Commit**

```bash
git add src/lib/stores/runtime.svelte.ts
git commit -m "refactor(store): flatten ProcessLogPayload into FlatRow array"
```

---

### Task 7: Frontend LogViewer — FlatRow rendering, visual grouping, copy button

**Files:**
- Modify: `src/lib/components/LogViewer.svelte`

**Interfaces:**
- Consumes: `logs: FlatRow[]` (instead of `ProcessLogPayload[]`)
- Uses row properties: `entryId`, `lineIndex`, `isFirstLine`, `isContinuation`, `text`, `stream`, `timestamp`

- [ ] **Step 1: Update imports and props type**

```svelte
<!-- src/lib/components/LogViewer.svelte — script section -->

<script lang="ts">
  import { MAX_LOG_LINES_PER_PROCESS } from "$lib/stores/runtime.svelte";
  import type { FlatRow, ProcessLogPayload } from "$lib/types";  // add FlatRow
  import { isTypingTarget } from "$lib/utils/dom";
  import { computeVirtualScroll, isAtBottom } from "$lib/utils/virtualScroll";
  import {
    buildMatcher,
    highlightLine,
    lineMatches,
    type SearchOptions,
  } from "$lib/utils/searchHighlight";
  import { parseAnsi, stripAnsi, styleToCss } from "$lib/utils/ansi";
  import Icon from "$lib/components/ui/Icon.svelte";
  import LogToolbar from "$lib/components/LogToolbar.svelte";

  type Props = {
    logs: FlatRow[];       // was: ProcessLogPayload[]
    processName: string | null;
    truncatedCount: number;
    onClear: () => void;
    onActions?: (actions: { copy: () => void; clear: () => void }) => void;
  };

  let { logs, processName, truncatedCount, onClear, onActions }: Props = $props();
```

- [ ] **Step 2: Update state variables types**

```ts
let pausedLogs = $state<FlatRow[] | null>(null);  // was: ProcessLogPayload[] | null
```

- [ ] **Step 3: Update `filteredLogs` — use `row.text` instead of `entry.line`**

```ts
let filteredLogs = $derived.by(() => {
    const base = paused ? (pausedLogs ?? logs) : logs;
    if (matcher === null || "error" in matcher) return base;
    return base.filter((row) => lineMatches(matcher, `${row.stream} ${stripAnsi(row.text)}`));
  });
```

- [ ] **Step 4: Update `copyLogs` to work with FlatRow (multi-line entries)**

```ts
async function copyLogs() {
    const text = filteredLogs
      .map(
        (row) =>
          `${row.timestamp ? new Date(row.timestamp).toLocaleTimeString() + ' ' : ''}${row.stream} ${stripAnsi(row.text)}`,
      )
      .join("\n");
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      if (copyTimer !== null) {
        clearTimeout(copyTimer);
      }
      copyTimer = window.setTimeout(() => {
        copied = false;
        copyTimer = null;
      }, 1400);
    } catch {
      // clipboard unavailable — fail silently
    }
  }
```

- [ ] **Step 5: Add per-entry copy function**

```ts
let copiedEntryId = $state<number | null>(null);
let entryCopyTimer = $state<number | null>(null);

async function copyEntry(entryId: number) {
    const entryLines = filteredLogs
      .filter((row) => row.entryId === entryId)
      .map((row) => stripAnsi(row.text));
    const text = entryLines.join("\n");
    try {
      await navigator.clipboard.writeText(text);
      copiedEntryId = entryId;
      if (entryCopyTimer !== null) {
        clearTimeout(entryCopyTimer);
      }
      entryCopyTimer = window.setTimeout(() => {
        copiedEntryId = null;
        entryCopyTimer = null;
      }, 1400);
    } catch {
      // clipboard unavailable — fail silently
    }
  }
```

- [ ] **Step 6: Update cleanup effect**

```ts
// Add entryCopyTimer cleanup:
$effect(() => {
    return () => {
      if (copyTimer !== null) {
        clearTimeout(copyTimer);
      }
      if (entryCopyTimer !== null) {
        clearTimeout(entryCopyTimer);
      }
    };
  });
```

- [ ] **Step 7: Update border corner logic for entryId-based grouping**

```ts
const borderCornerClass = $derived((indexInLogs: number): string => {
    const row = filteredLogs[indexInLogs];
    if (!row) return "";
    const prev = indexInLogs > 0 ? filteredLogs[indexInLogs - 1] : null;
    const next =
      indexInLogs < filteredLogs.length - 1
        ? filteredLogs[indexInLogs + 1]
        : null;
    const sameAsPrev = prev !== null && prev.entryId === row.entryId;
    const sameAsNext = next !== null && next.entryId === row.entryId;

    if (!sameAsPrev && !sameAsNext) return "rounded-tl rounded-bl";
    if (!sameAsPrev) return "rounded-tl";
    if (!sameAsNext) return "rounded-bl";
    return "";
  });
```

- [ ] **Step 8: Update the template — render FlatRow with visual grouping and copy button**

Replace the `{#each visibleItems as entry...}` block (lines 306-338):

```svelte
        {#each visibleItems as row, index (`${row.entryId}-${row.lineIndex}-${startIndex + index}`)}
          <div
            style="position: absolute; top: {(startIndex + index) *
              ROW_HEIGHT}px; left: 0; right: 0; height: {ROW_HEIGHT}px;"
            class="group flex items-center gap-3 px-3 hover:bg-surface-hover/40 border-l-[3px] {borderByStream[
              row.stream
            ] ?? 'border-l-transparent'} {borderCornerClass(
              startIndex + index,
            )} {row.isContinuation ? 'bg-surface-muted/40' : ''} {startIndex + index === activeMatchIndex && matcherActive ? 'bg-surface-hover/60' : ''}"
          >
            <span class="flex shrink-0 items-center gap-1 whitespace-nowrap text-[10px] text-text-subtle w-[70px]">
              {#if row.isFirstLine}
                {new Date(row.timestamp).toLocaleTimeString()}
              {/if}
            </span>
            <span
              class={`whitespace-nowrap ${toneByStream[row.stream] ?? "text-text"}`}
            >
              {#if row.isFirstLine && row.stream === "system" && /ready|listening/i.test(stripAnsi(row.text))}
                <span class="mr-1">&#9679;</span>
              {/if}
              {#each parseAnsi(row.text) as ansiSeg}
                <span style={styleToCss(ansiSeg.style) ?? undefined}>
                  {#each highlightLine(ansiSeg.text, matcher) as seg}
                    {#if seg.match}
                      <mark
                        class={`text-text rounded-[2px] ${startIndex + index === activeMatchIndex && matcherActive ? "bg-warning/60" : "bg-warning/30"}`}
                        >{seg.text}</mark
                      >
                    {:else}
                      {seg.text}
                    {/if}
                  {/each}
                </span>
              {/each}
            </span>
            {#if row.isFirstLine}
              <button
                type="button"
                onclick={(e: MouseEvent) => { e.stopPropagation(); copyEntry(row.entryId); }}
                class="ml-auto hidden shrink-0 grid h-5 w-5 place-items-center rounded text-text-subtle hover:text-text group-hover:grid"
                title="Copy entry"
              >
                <Icon name={copiedEntryId === row.entryId ? "check" : "copy"} size="xs" />
              </button>
            {/if}
          </div>
        {/each}
```

- [ ] **Step 9: Compile check**

Run: `deno task build`
Expected: compiles (test files will have type errors — that's next task).

- [ ] **Step 10: Commit**

```bash
git add src/lib/components/LogViewer.svelte
git commit -m "feat(log-viewer): FlatRow rendering, visual grouping, per-entry copy button"
```

---

### Task 8: Tests — update existing tests and add new ones

**Files:**
- Modify: `src/lib/components/LogViewer.test.ts`
- Modify: `src/lib/components/LogToolbar.test.ts`

**Interfaces:**
- Consumes: `FlatRow` type
- Test helpers change from `ProcessLogPayload` to `FlatRow`

- [ ] **Step 1: Update test helpers in `LogViewer.test.ts`**

```ts
// src/lib/components/LogViewer.test.ts

import type { FlatRow } from "$lib/types";

// Replace makeLine:
function makeRow(text: string, i: number, overrides: Partial<FlatRow> = {}): FlatRow {
  return {
    entryId: i,
    lineIndex: 0,
    isFirstLine: true,
    isContinuation: false,
    text,
    stream: "stdout" as const,
    timestamp: `2026-01-01T00:00:${i.toString().padStart(2, "0")}Z`,
    ...overrides,
  };
}

// Replace logs function:
const logs = (lines: string[]): FlatRow[] => lines.map((text, i) => makeRow(text, i));
```

- [ ] **Step 2: Add test for multi-line entry rendering**

```ts
it("renders multi-line entries without timestamps on continuation lines", async () => {
    const multiLineLogs: FlatRow[] = [
      makeRow("Error: something failed", 0),
      makeRow("  at app.ts:42", 0, { lineIndex: 1, isFirstLine: false, isContinuation: true, timestamp: '' }),
      makeRow("  at db.ts:15", 0, { lineIndex: 2, isFirstLine: false, isContinuation: true, timestamp: '' }),
    ];
    const { container } = render(LogViewer, {
      props: makeProps({ logs: multiLineLogs }),
    });
    expect(container.textContent).toContain("Error: something failed");
    expect(container.textContent).toContain("at app.ts:42");
  });
```

- [ ] **Step 3: Add test for copy entry on button click**

```ts
it("copies the full entry when copy button is clicked", async () => {
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
    const entryLines: FlatRow[] = [
      makeRow("line 1", 0),
      makeRow("line 2 continuation", 0, { lineIndex: 1, isFirstLine: false, isContinuation: true, timestamp: '' }),
    ];
    const { container } = render(LogViewer, {
      props: makeProps({ logs: entryLines }),
    });
    const copyBtn = container.querySelector('[title="Copy entry"]') as HTMLButtonElement;
    if (copyBtn) {
      await fireEvent.click(copyBtn);
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith("line 1\nline 2 continuation");
    }
  });
```

- [ ] **Step 4: Run existing tests**

```bash
npx vitest run src/lib/components/LogViewer.test.ts src/lib/components/LogToolbar.test.ts
```
Expected: all tests pass. Fix any failures.

- [ ] **Step 5: Run full test suite**

```bash
npx vitest run
```
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/LogViewer.test.ts src/lib/components/LogToolbar.test.ts
git commit -m "test: update log tests for FlatRow and multi-line entries"
```

---

### Task 9: Rust tests for log buffering

**Files:**
- Create: `src-tauri/src/application/orchestrator/log_test.rs` (or add tests inline)
- Modify: Check if there are orchestrator tests that need updating

**Interfaces:**
- Tests: `spawn_log_task` with and without timestamp pattern

- [ ] **Step 1: Add inline test module to `log.rs`**

```rust
// src-tauri/src/application/orchestrator/log.rs — at end of file

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use std::io::Cursor;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn without_pattern_every_line_is_own_entry() {
        // This test validates the contract: without a pattern, each line = one entry.
        // We verify via the lines field length.
        let payload = ProcessLogPayload {
            session_id: crate::domain::runtime::RunSessionId::new(),
            runtime_id: crate::domain::runtime::ProcessRuntimeId::new(),
            process_name: "test".into(),
            stream: crate::domain::process::LogStream::Stdout,
            lines: vec!["line1".into()],
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(payload.lines.len(), 1);
    }

    #[test]
    fn pattern_detects_timestamp() {
        let re = Regex::new(r"^\d{4}-\d{2}-\d{2}").unwrap();
        assert!(re.is_match("2026-07-06 12:00:00 INFO starting"));
        assert!(!re.is_match("  at com.example.Main.main(Main.java:42)"));
    }

    #[test]
    fn empty_pattern_means_no_grouping() {
        // Absence of pattern → every line is a new entry (handled by log_task)
        let re: Option<Regex> = None;
        // The log_task uses `timestamp_pattern.is_none()` to skip merging
        assert!(re.is_none());
    }
}
```

- [ ] **Step 2: Run Rust tests**

```bash
cd src-tauri && cargo test
```
Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/application/orchestrator/log.rs
git commit -m "test(log): add timestamp pattern detection tests"
```

---

### Task 10: Integration — build and verify

- [ ] **Step 1: Full build**

```bash
deno task build
```
Expected: clean build, no errors.

- [ ] **Step 2: Run all tests**

```bash
npx vitest run && cd src-tauri && cargo test
```
Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git commit -m "chore: finalize multi-line log implementation" --allow-empty
```

---

### Task 11: Visual Companion — present UI changes

**Deliverable:** Screenshot/render of the before/after for the LogViewer.

- [ ] **Step 1:** Show how a multi-line stack trace renders:
  - Before: each line with its own timestamp, no visual grouping
  - After: grouped entry with single timestamp, border corners by entry, continuation lines muted background, copy button on first line hover

