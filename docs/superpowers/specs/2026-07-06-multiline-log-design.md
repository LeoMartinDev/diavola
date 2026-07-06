# Multi-line Log Rendering & One-click Copy

**Date**: 2026-07-06
**Status**: Design approved

## Problem

Multi-line log entries (stack traces, JSON dumps, SQL queries) are rendered with a timestamp at the beginning of every line. This makes them hard to read and visually distinguish from separate log entries. Additionally, there is no easy way to copy a single log entry to clipboard.

## Goals

1. **Multi-line detection**: Buffer and merge consecutive log lines that belong to the same logical entry
2. **Visual grouping**: Render multi-line entries as compact visual groups (no repeated timestamp, grouped borders, subtle background)
3. **One-click copy**: Copy button on hover for each log entry

## Non-goals

- Changing the virtual scroll strategy (stays fixed-height ROW_HEIGHT=22px)
- Supporting multi-line entries in the terminal widget (xterm.js handles its own rendering)
- Merging across different streams (stdout/stderr stay separate)

---

## Design

### Approach: Backend merging + Frontend flatten to fixed-height rows

The Rust backend buffers lines and emits complete log entries. The frontend flattens each entry into fixed-height rows with metadata for visual grouping. The virtual scroll operates on the flattened rows unchanged.

### Data Flow

```
diavola.yml                 Rust (log_task)                Tauri event              Frontend store             LogViewer
───────────                 ────────────────                ──────────              ──────────────             ─────────
logTimestampPattern ──────> LineBuffer                     process-log ──────────> FlatRow[] ───────────────> renders each
(global)                    │                                payload:                flattened from              row per its
                             │ reads line by line            {                       ProcessLogPayload           metadata
process.logTimestampPattern─┤ buffers                        sessionId,
(override)                  │ detects timestamp regex        runtimeId,
                            │ emits complete entry            processName,
                            ▼                                 stream,
                            ProcessLogPayload {               timestamp,
                              ...                            lines: string[]
                              lines: string[],              }
                              timestamp: DateTime,
                            }
```

### New ProcessLogPayload

```rust
// src-tauri/src/domain/runtime.rs
pub struct ProcessLogPayload {
    pub session_id: RunSessionId,
    pub runtime_id: ProcessRuntimeId,
    pub process_name: String,
    pub stream: LogStream,
    pub lines: Vec<String>,       // was: pub line: String
    pub timestamp: DateTime<Utc>, // timestamp of the FIRST line only
}
```

```ts
// src/lib/types.ts
export type ProcessLogPayload = {
  sessionId: RunSessionId;
  runtimeId: ProcessRuntimeId;
  processName: string;
  stream: LogStream;
  lines: string[];       // was: line: string
  timestamp: string;     // timestamp of the FIRST line only
};
```

### Frontend Internal Model

```ts
type FlatRow = {
  entryId: number;        // unique per entry (incrementing counter)
  lineIndex: number;      // 0-based index within the entry
  isFirstLine: boolean;   // true for the first line of an entry
  isContinuation: boolean;// true for lines 1..N (not the first)
  text: string;           // the actual log text
  stream: LogStream;      // stdout | stderr | system
  timestamp: string;      // only used when isFirstLine
};
```

Each `ProcessLogPayload` received by the store is flattened into `lines.length` FlatRow objects sharing the same `entryId`.

### Virtual Scroll

Unchanged. `computeVirtualScroll()` works on `FlatRow[]` with `ROW_HEIGHT = 22px`. Each FlatRow is one virtual item regardless of the entry it belongs to. No variable-height scrolling needed.

---

## Implementation

### 1. Config (`diavola.yml`)

Two new optional fields:

```yaml
# Root level — global pattern applied to all processes
logTimestampPattern: "^\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}"

processes:
  api:
    command: "cargo run"
    # Per-process override
    logTimestampPattern: "^\\[\\d{2}:\\d{2}:\\d{2}\\]"
```

- `logTimestampPattern`: a regex string. If a line matches → new entry. If not → continuation.
- Global (root) → applied to all processes.
- Per-process → overrides global.
- Absent (both levels) → fallback to current behavior (every line is its own entry).

Regex is compiled once on the Rust side at session start.

### 2. Backend (Rust)

#### `src-tauri/src/domain/runtime.rs`

- `ProcessLogPayload.line: String` → `lines: Vec<String>`

#### `src-tauri/src/application/orchestrator/log.rs`

`spawn_log_task()` modified to buffer lines:

```
buffer = Vec::new()
first_timestamp = None

for each line from BufReader:
    if line matches timestamp_pattern:
        if buffer not empty:
            emit ProcessLogPayload { lines: buffer, timestamp: first_timestamp }
        buffer = []
        first_timestamp = Some(DateTime::now())

    buffer.push(line)
    if first_timestamp is None:
        first_timestamp = Some(DateTime::now())

on stream end:
    if buffer not empty:
        emit ProcessLogPayload { lines: buffer, timestamp: first_timestamp }
```

- When no pattern is configured → emit each line as a single-element `lines: vec![line]` (backward compatible).
- Each stream (stdout, stderr) has its own independent buffer.

#### `src-tauri/src/infrastructure/log_store.rs`

- Store type changes from `VecDeque<ProcessLogPayload>` — but semantically unchanged.
- The 10,000 limit still applies to **entries**, not physical lines (one 50-line entry = one slot).

#### `src-tauri/src/application/readiness.rs`

- Log readiness checker tests each **individual line** from `payload.lines` against the readiness pattern. This ensures no readiness signal is missed inside a multi-line entry.

### 3. Frontend (Svelte)

#### `src/lib/stores/runtime.svelte.ts`

- `processLogs` changes from `Record<Id, ProcessLogPayload[]>` to `Record<Id, FlatRow[]>`
- On `process-log` event: flatten `payload.lines` into `FlatRow[]` using an incrementing entry counter
- `MAX_LOG_LINES_PER_PROCESS = 10_000` applies to `FlatRow` count (not entries)
- Truncation tracking: count truncated **entries** (not rows) for the banner message

```ts
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
```

#### `src/lib/components/LogViewer.svelte`

Visual changes for FlatRow rendering:

| Aspect | Current | New |
|--------|---------|-----|
| Timestamp display | Shown on every row | Only shown when `isFirstLine` |
| Left padding | Same for all rows | Continuation rows align text with the text column (no timestamp gap) |
| Border radius | Per-stream adjacency | Grouped by `entryId`: rounded top-left on first line, bottom-left on last line |
| Background | None | Continuation rows get a subtle muted background (`bg-surface-muted`) |
| Copy button | Global "Copy all" in toolbar | Per-entry copy button on hover (first line only, right-aligned) |

#### Copy button per entry

- Icon: `Copy` from Lucide (becomes `Check` for 1.5s on click)
- Appears on hover over the first line of an entry
- Positioned to the right of the row
- On click → `navigator.clipboard.writeText(fullEntryText)` where `fullEntryText` is all lines of the entry joined with `\n`
- ANSI codes are stripped before copying

#### Search

Unchanged. Filtering and highlighting still operates on `FlatRow.text`.

### 4. Visual Design Summary

Single-line entry:
```
[TIMESTAMP] [stderr] Error: connection refused    [📋]
```

Multi-line entry:
```
[TIMESTAMP] [stderr] Traceback (most recent call last):    [📋]
             [stderr]   File "app.py", line 42, in handle
             [stderr]   File "db.py", line 15, in connect
             [stderr] ConnectionError: timeout
```

- Continuation lines have no timestamp, indented to align with text
- Left border colored by stream spans the full entry height
- Subtle background tint on continuation lines for visual grouping
- Rounded corners: top-left on first line, bottom-left on last line of the entry

---

## Edge Cases

| Case | Behavior |
|------|----------|
| No pattern configured (global + per-process) | Every line = single-line entry (current behavior) |
| Invalid regex in config | Caught at config parsing time, shown in YAML editor |
| Very large entry (500+ lines) | One entry = many FlatRows. Virtual scroll handles this. Total 10k row limit still applies |
| Empty lines between entries | Merged with previous entry if they don't match the timestamp pattern |
| Stream dropped mid-entry | Buffer flushed on stream end, last entry emitted |
| Copy with ANSI codes | ANSI stripped before clipboard write |
| Pattern that also matches continuation lines | User error — pattern must be specific enough to only match actual log headers |

## Tests

**Rust:**
- `log_task` with timestamp pattern: 3 lines (header + 2 continuations) → 1 payload with `lines.len() == 3`
- `log_task` without pattern: each line → 1 payload with `lines.len() == 1` (regression)
- `log_task` pattern match on first line only: subsequent matches start new entries

**Frontend:**
- `flattenPayload`: 1 line → 1 FlatRow with `isFirstLine: true`, 3 lines → 3 FlatRow with same `entryId`
- LogViewer: continuation rows hide timestamp
- LogViewer: copy button appears on hover, copies correct text
- LogViewer: search still works across multi-line entries
- Config validation: invalid regex → error surfaced in YAML editor
