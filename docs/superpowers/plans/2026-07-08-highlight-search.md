# Highlight-only log search (Rust-backed) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop filtering logs on search; instead highlight matches, keep prev/next navigation + match count, never shift the UI, and compute match positions in Rust for the live-streaming case.

**Architecture:** Rust gains a flattened, FlatRow-capped (10,000) search buffer (repurposing the unused `InMemoryLogStore`) and a `search_process_logs` command returning match row positions. JS keeps all rows rendered, highlights substrings lazily per visible row, and resolves match count/positions via Rust when live + Tauri, or via a local JS scan when paused / in browser dev / in tests.

**Tech Stack:** Rust (tauri 2, `regex` 1, tokio), Svelte 5 (runes), TypeScript, Vitest + @testing-library/svelte, `cargo test`.

## Global Constraints

- `MAX_LOG_LINES_PER_PROCESS = 10_000` (flat-row cap) — Rust buffer and JS both cap at this value.
- Search target string per row is exactly `"{stream} {ansi-stripped text}"` on both sides (stream is lowercase: `stdout`/`stderr`/`system`).
- Regex escaping parity: escape the set `[.*+?^${}()|[\]\\]` when `regex == false` (matches `src/lib/utils/searchHighlight.ts:10`).
- Case-insensitive by default; case-sensitive only when the toolbar toggle is on.
- No filtering: every row stays rendered during search. No layout shift: the toolbar nav group is always mounted (visibility-toggled, not conditionally rendered).
- Command naming follows the existing convention: JS sends `{ request: { ... } }`, Rust command takes a `request: SearchProcessLogsRequest` parameter; fields are `camelCase` over the wire.
- Commits per task, conventional-commit messages (`feat:`, `refactor:`, `test:`, `chore:`).

---

## File Structure

- Create `src-tauri/src/infrastructure/ansi.rs` — ANSI escape stripper (Rust port of `ansi.ts` ESC_SEQ) + `OnceLock`-cached `Regex`.
- Rewrite `src-tauri/src/infrastructure/log_store.rs` — `InMemoryLogStore` stores flat rows; inherent `append` / `search` / `len` / `clear`; drop the unused `LogStore` trait.
- Modify `src-tauri/src/domain/process.rs` — add `LogStream::as_str`.
- Modify `src-tauri/src/infrastructure/mod.rs` — `pub mod ansi;`.
- Modify `src-tauri/src/application/orchestrator/mod.rs` — remove `LogStore` trait import; `append(&payload)` borrow; add `search_logs(...)`.
- Modify `src-tauri/src/tauri_api/commands.rs` — `search_process_logs` command + request/reply types.
- Modify `src-tauri/src/lib.rs` — register the command.
- Modify `src/lib/types.ts` — `SearchProcessLogsReply`.
- Modify `src/lib/tauri/client.ts` — `searchProcessLogs`.
- Create `src/lib/utils/scheduler.ts` — `debounceWithMaxWait` (+ tests).
- Create `src/lib/utils/logSearch.ts` — `computeMatchIndices`, `searchLogsLocally` (+ tests).
- Modify `src/lib/components/LogViewer.svelte` — remove filtering; wire Rust/local matches; add `runtimeId` prop; resets.
- Modify `src/lib/components/LogToolbar.svelte` — always-mounted nav group.
- Modify tests: `LogViewer.test.ts`, `LogToolbar.test.ts`, new `scheduler.test.ts`, `logSearch.test.ts`.
- Modify `src/routes/+page.svelte` — pass `runtimeId` to `LogViewer`.

---

## Task 1: Rust ANSI strip helper

**Files:**
- Create: `src-tauri/src/infrastructure/ansi.rs`
- Modify: `src-tauri/src/infrastructure/mod.rs:1` (add module)

**Interfaces:**
- Produces: `pub fn strip_ansi(input: &str) -> String` and `pub(crate) fn ansi_regex() -> &'static Regex`.

- [ ] **Step 1: Register the module**

Modify `src-tauri/src/infrastructure/mod.rs` — add `pub mod ansi;` (keep alphabetical-ish; place after `config_loader`):

```rust
pub mod ansi;
pub mod config_loader;
pub mod log_store;
```

- [ ] **Step 2: Write the failing test**

Create `src-tauri/src/infrastructure/ansi.rs`:

```rust
use regex::Regex;
use std::sync::OnceLock;

/// Compiled, shared ANSI escape matcher. Ports the `ESC_SEQ` regex from
/// `src/lib/utils/ansi.ts` so Rust search targets match the JS-stripped text.
///
/// Alternation order mirrors the TS source:
///   1. SGR            ESC [ <params> m
///   2. other CSI      ESC [ <params> <final letter>
///   3. OSC            ESC ] … (BEL | ST=ESC \)
///   4. charset        ESC ( ) * + . / <char>
///   5. misc           ESC <char>   (also swallows a trailing lone ESC)
pub(crate) fn ansi_regex() -> &'static Regex {
    static ANSI: OnceLock<Regex> = OnceLock::new();
    ANSI.get_or_init(|| {
        Regex::new(concat!(
            r"\x1b\[[0-9;]*m",                 // SGR
            r"|\x1b\[[0-9;?]*[A-Za-z]",        // other CSI
            r"|\x1b\](?:[^\x07]|\x1b[^\\])*(?:\x07|\x1b\\)", // OSC
            r"|\x1b[()*+./].",                 // charset
            r"|\x1b.",                         // misc / lone ESC
        ))
        .expect("ansi escape regex must compile")
    })
}

/// Remove ANSI escape sequences, matching `stripAnsi` in `src/lib/utils/ansi.ts`.
pub fn strip_ansi(input: &str) -> String {
    ansi_regex().replace_all(input, "").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_sgr_color_codes() {
        assert_eq!(strip_ansi("\x1b[32mhello\x1b[39m"), "hello");
    }

    #[test]
    fn leaves_plain_text_untouched() {
        assert_eq!(strip_ansi("listening on 3000"), "listening on 3000");
    }

    #[test]
    fn removes_csi_cursor_codes() {
        assert_eq!(strip_ansi("a\x1b[2Kb"), "ab");
    }

    #[test]
    fn removes_osc_terminated_by_bel() {
        assert_eq!(strip_ansi("\x1b]0;title\x07body"), "body");
    }

    #[test]
    fn removes_osc_terminated_by_st() {
        assert_eq!(strip_ansi("\x1b]0;title\x1b\\body"), "body");
    }
}
```

- [ ] **Step 3: Run test to verify it passes**

Run (workdir `src-tauri`): `cargo test ansi::`
Expected: PASS (5 tests). If the OSC alternation fails to compile in the `regex` crate, fall back to removing that arm and note the divergence — but it compiles with `regex` 1.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/infrastructure/ansi.rs src-tauri/src/infrastructure/mod.rs
git commit -m "feat: add Rust ANSI escape stripper for log search"
```

---

## Task 2: Flattened FlatRow-capped log store with search

**Files:**
- Rewrite: `src-tauri/src/infrastructure/log_store.rs`
- Modify: `src-tauri/src/domain/process.rs:17` (add `LogStream::as_str`)
- Modify: `src-tauri/src/application/orchestrator/mod.rs:29` (drop trait import) and `:416`,`:443` (borrow on append)

**Interfaces:**
- Produces:
  - `pub struct InMemoryLogStore` (name kept to avoid touching `session.rs`) with:
    - `pub fn default() -> Self` (cap 10_000)
    - `pub fn append(&mut self, payload: &ProcessLogPayload)`
    - `pub fn len(&self, runtime_id: &ProcessRuntimeId) -> usize`
    - `pub fn clear(&mut self, runtime_id: &ProcessRuntimeId)`
    - `pub fn search(&self, runtime_id: &ProcessRuntimeId, query: &str, regex: bool, case_sensitive: bool, up_to: usize) -> Result<SearchMatches, AppError>`
  - `pub struct SearchMatches { pub match_count: usize, pub match_indices: Vec<u32> }`
- Consumes: `crate::infrastructure::ansi::strip_ansi`, `crate::error::AppError`, `regex::Regex`.

- [ ] **Step 1: Add `LogStream::as_str`**

Modify `src-tauri/src/domain/process.rs` — append an `impl` block after the enum:

```rust
impl LogStream {
    pub fn as_str(self) -> &'static str {
        match self {
            LogStream::Stdout => "stdout",
            LogStream::Stderr => "stderr",
            LogStream::System => "system",
        }
    }
}
```

- [ ] **Step 2: Write the failing tests**

Rewrite `src-tauri/src/infrastructure/log_store.rs` with the new struct and tests (implementation + tests together because the store is pure logic — write tests first mentally, then the minimal impl below makes them pass):

```rust
use std::collections::{HashMap, VecDeque};

use regex::RegexBuilder;

use crate::{
    domain::runtime::{ProcessLogPayload, ProcessRuntimeId},
    error::AppError,
    infrastructure::ansi::strip_ansi,
};
use crate::domain::process::LogStream;

/// One searchable line: the raw text plus which stream it came from. Mirrors
/// the fields JS needs to build the `"{stream} {stripped}"` search target.
#[derive(Debug, Clone)]
pub struct FlatRowEntry {
    pub text: String,
    pub stream: LogStream,
}

#[derive(Debug, Clone, Default)]
pub struct SearchMatches {
    pub match_count: usize,
    pub match_indices: Vec<u32>,
}

/// In-memory log buffer keyed by process runtime id, stored as flat rows and
/// capped at `limit` rows. The cap matches JS `MAX_LOG_LINES_PER_PROCESS` so
/// positional row indices returned by [`search`] align with the frontend's
/// `logs` array.
#[derive(Debug, Clone)]
pub struct InMemoryLogStore {
    limit: usize,
    entries: HashMap<ProcessRuntimeId, VecDeque<FlatRowEntry>>,
}

impl Default for InMemoryLogStore {
    fn default() -> Self {
        Self::new(10_000)
    }
}

impl InMemoryLogStore {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            entries: HashMap::new(),
        }
    }

    /// Flatten a payload into one row per line (entry-pattern grouping already
    /// happened upstream in `orchestrator/log.rs`). Drop from the front beyond
    /// the cap — identical policy to JS.
    pub fn append(&mut self, payload: &ProcessLogPayload) {
        let queue = self.entries.entry(payload.runtime_id.clone()).or_default();
        for line in &payload.lines {
            queue.push_back(FlatRowEntry {
                text: line.clone(),
                stream: payload.stream,
            });
        }
        while queue.len() > self.limit {
            queue.pop_front();
        }
    }

    pub fn len(&self, runtime_id: &ProcessRuntimeId) -> usize {
        self.entries.get(runtime_id).map(|q| q.len()).unwrap_or(0)
    }

    pub fn clear(&mut self, runtime_id: &ProcessRuntimeId) {
        if let Some(queue) = self.entries.get_mut(runtime_id) {
            queue.clear();
        }
    }

    /// Return the 0-based row positions whose `"{stream} {stripped text}"`
    /// matches `query`. Only the first `min(up_to, len)` rows are considered so
    /// the frontend can constrain results to rows it has actually rendered.
    pub fn search(
        &self,
        runtime_id: &ProcessRuntimeId,
        query: &str,
        regex: bool,
        case_sensitive: bool,
        up_to: usize,
    ) -> Result<SearchMatches, AppError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(SearchMatches::default());
        }
        let source = if regex {
            trimmed.to_string()
        } else {
            escape_regex(trimmed)
        };
        let pattern = RegexBuilder::new(&source)
            .case_insensitive(!case_sensitive)
            .build()
            .map_err(|e| AppError::runtime(format!("invalid search regex: {e}")))?;

        let Some(queue) = self.entries.get(runtime_id) else {
            return Ok(SearchMatches::default());
        };
        let limit = up_to.min(queue.len());

        let mut indices = Vec::new();
        for (i, entry) in queue.iter().take(limit).enumerate() {
            let target = format!("{} {}", entry.stream.as_str(), strip_ansi(&entry.text));
            if pattern.is_match(&target) {
                indices.push(i as u32);
            }
        }
        Ok(SearchMatches {
            match_count: indices.len(),
            match_indices: indices,
        })
    }
}

/// Escape regex metacharacters for a literal search. Mirrors
/// `escapeRegExp` in `src/lib/utils/searchHighlight.ts`.
fn escape_regex(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for ch in s.chars() {
        if matches!(
            ch,
            '.' | '*' | '+' | '?' | '^' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '\\'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::process::LogStream;
    use crate::domain::runtime::{ProcessLogPayload, ProcessRuntimeId, RunSessionId};

    fn payload(runtime: &ProcessRuntimeId, stream: LogStream, lines: &[&str]) -> ProcessLogPayload {
        ProcessLogPayload {
            session_id: RunSessionId::new(),
            runtime_id: runtime.clone(),
            process_name: "api".into(),
            stream,
            lines: lines.iter().map(|s| (*s).to_string()).collect(),
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn append_flattens_lines_into_rows() {
        let store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        let mut store = store;
        store.append(&payload(&runtime, LogStream::Stdout, &["a", "b", "c"]));
        assert_eq!(store.len(&runtime), 3);
    }

    #[test]
    fn caps_at_limit_and_drops_from_front() {
        let mut store = InMemoryLogStore::new(3);
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stdout, &["a", "b", "c", "d"]));
        // 4 appended, cap 3 → front ("a") dropped.
        let matches = store.search(&runtime, "b", false, false, usize::MAX).unwrap();
        assert!(matches.match_indices.is_empty());
        let matches = store.search(&runtime, "d", false, false, usize::MAX).unwrap();
        assert_eq!(matches.match_indices, vec![2]);
    }

    #[test]
    fn literal_substring_match_is_case_insensitive_by_default() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stdout, &["Listening on 3000"]));
        let m = store.search(&runtime, "listening", false, false, usize::MAX).unwrap();
        assert_eq!(m.match_indices, vec![0]);
    }

    #[test]
    fn case_sensitive_match_respects_casing() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stdout, &["Listening on 3000", "listening on 3001"]));
        let m = store.search(&runtime, "Listening", false, true, usize::MAX).unwrap();
        assert_eq!(m.match_indices, vec![0]);
    }

    #[test]
    fn regex_mode_uses_pattern() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(
            &runtime,
            LogStream::Stdout,
            &["error 42", "warn 7", "error 99"],
        ));
        let m = store.search(&runtime, r"error \d+", true, false, usize::MAX).unwrap();
        assert_eq!(m.match_indices, vec![0, 2]);
    }

    #[test]
    fn matches_against_stream_prefixed_target() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stderr, &["boom"]));
        // Querying the stream name matches because target is "stderr boom".
        let m = store.search(&runtime, "stderr", false, false, usize::MAX).unwrap();
        assert_eq!(m.match_indices, vec![0]);
    }

    #[test]
    fn strips_ansi_before_matching() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stdout, &["\x1b[32mready\x1b[39m"]));
        let m = store.search(&runtime, "ready", false, false, usize::MAX).unwrap();
        assert_eq!(m.match_indices, vec![0]);
    }

    #[test]
    fn up_to_truncates_considered_rows() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stdout, &["hit", "hit", "hit"]));
        let m = store.search(&runtime, "hit", false, false, 2).unwrap();
        assert_eq!(m.match_indices, vec![0, 1]);
    }

    #[test]
    fn empty_query_returns_no_matches() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stdout, &["x"]));
        let m = store.search(&runtime, "   ", false, false, usize::MAX).unwrap();
        assert_eq!(m.match_count, 0);
    }

    #[test]
    fn invalid_regex_returns_error() {
        let store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        let result = store.search(&runtime, "(unclosed", true, false, usize::MAX);
        assert!(result.is_err());
    }

    #[test]
    fn unknown_runtime_returns_no_matches() {
        let store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        let m = store.search(&runtime, "anything", false, false, usize::MAX).unwrap();
        assert_eq!(m.match_count, 0);
    }

    #[test]
    fn clear_removes_rows() {
        let mut store = InMemoryLogStore::default();
        let runtime = ProcessRuntimeId::new();
        store.append(&payload(&runtime, LogStream::Stdout, &["a"]));
        store.clear(&runtime);
        assert_eq!(store.len(&runtime), 0);
    }
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run (workdir `src-tauri`): `cargo test log_store::`
Expected: PASS (all store tests).

- [ ] **Step 4: Update orchestrator to drop the trait and borrow on append**

Modify `src-tauri/src/application/orchestrator/mod.rs`:

- Line 29 — change the import to remove the now-deleted `LogStore` trait:

```rust
    infrastructure::{config_loader::LoadedProjectConfig},
```

- Line 416 — change `active.logs.append(payload);` to:

```rust
                        active.logs.append(&payload);
```

- Line 443 — change the second `active.logs.append(payload);` to:

```rust
                        active.logs.append(&payload);
```

- [ ] **Step 5: Build and run all Rust tests**

Run (workdir `src-tauri`): `cargo test`
Expected: whole crate compiles and all tests pass (including pre-existing `log.rs` and `ansi.rs` tests).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/infrastructure/log_store.rs src-tauri/src/domain/process.rs src-tauri/src/application/orchestrator/mod.rs
git commit -m "refactor: flatten log store into capped flat rows with search"
```

---

## Task 3: Orchestrator search method + Tauri command

**Files:**
- Modify: `src-tauri/src/application/orchestrator/mod.rs` (add `search_logs`)
- Modify: `src-tauri/src/tauri_api/commands.rs` (request/reply types + command)
- Modify: `src-tauri/src/lib.rs:134` (register)

**Interfaces:**
- Produces:
  - `ProcessOrchestrator::search_logs(&self, window_key: &str, runtime_id: &ProcessRuntimeId, query: &str, regex: bool, case_sensitive: bool, up_to: usize) -> Result<SearchMatches, AppError>`
  - Tauri command `search_process_logs(window, state, request) -> Result<SearchProcessLogsReply, String>`
- Consumes: `InMemoryLogStore::search`, `window_key`, `to_error_string`, `AppError`, `SearchMatches` (re-exported from `infrastructure::log_store`).

- [ ] **Step 1: Add `search_logs` to the orchestrator**

In `src-tauri/src/application/orchestrator/mod.rs`, first extend the import at line 26–30 region. Add `ProcessRuntimeId` to the `domain::runtime` import and bring `SearchMatches` in:

```rust
    domain::{
        config::{ProcessConfig, ProcessKind},
        process::{LogStream, ProcessStatus},
        project::ProjectRecord,
        runtime::{ProcessLogPayload, ProcessRuntimeId, ProcessSnapshot, RunSessionSnapshot},
    },
    error::AppError,
    infrastructure::{
        config_loader::LoadedProjectConfig,
        log_store::SearchMatches,
    },
```

Then add the method inside `impl ProcessOrchestrator`, immediately after the `snapshot` method (after line 238):

```rust
    pub async fn search_logs(
        &self,
        window_key: &str,
        runtime_id: &ProcessRuntimeId,
        query: &str,
        regex: bool,
        case_sensitive: bool,
        up_to: usize,
    ) -> Result<SearchMatches, AppError> {
        let state = self.inner.lock().await;
        let Some(active) = state.sessions.get(window_key) else {
            return Ok(SearchMatches::default());
        };
        active
            .logs
            .search(runtime_id, query, regex, case_sensitive, up_to)
    }
```

- [ ] **Step 2: Add the command + wire types**

In `src-tauri/src/tauri_api/commands.rs`:

Add imports near the top (extend the existing `domain::runtime` import to include `ProcessRuntimeId`, and add `SearchMatches`):

```rust
use crate::{
    domain::{
        config::DiavolaConfig,
        project::{ProjectId, ProjectRecord},
        runtime::{ProcessRuntimeId, RunSessionSnapshot},
        terminal::{TerminalSessionId, TerminalSnapshot},
    },
    error::{AppError, ErrorCode},
    infrastructure::{
        config_loader::{load_config_async, parse_config_document},
        git_info::{self, GitInfo},
        log_store::SearchMatches,
    },
    tauri_api::state::AppState,
};
```

Add the request/reply types near the other request structs (e.g. after `ResizeTerminalRequest`):

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchProcessLogsRequest {
    pub runtime_id: ProcessRuntimeId,
    pub query: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub up_to: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchProcessLogsReply {
    pub match_count: usize,
    pub match_indices: Vec<u32>,
}

impl From<SearchMatches> for SearchProcessLogsReply {
    fn from(m: SearchMatches) -> Self {
        Self {
            match_count: m.match_count,
            match_indices: m.match_indices,
        }
    }
}
```

Add the command (e.g. right after `get_session_snapshot`):

```rust
#[tauri::command]
pub async fn search_process_logs(
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: SearchProcessLogsRequest,
) -> Result<SearchProcessLogsReply, String> {
    state
        .orchestrator
        .search_logs(
            &window_key(&window),
            &request.runtime_id,
            &request.query,
            request.regex,
            request.case_sensitive,
            request.up_to,
        )
        .await
        .map(SearchProcessLogsReply::from)
        .map_err(to_error_string)
}
```

- [ ] **Step 3: Register the command**

Modify `src-tauri/src/lib.rs` — in the `invoke_handler` list (after line 144 `tauri_api::commands::get_session_snapshot,`), add:

```rust
            tauri_api::commands::search_process_logs,
```

- [ ] **Step 4: Build + run Rust tests**

Run (workdir `src-tauri`): `cargo test`
Expected: compiles, all tests pass. (The command itself isn't unit-tested — it's a thin wrapper over the orchestrator method and the store, both tested in Task 2. Integration is exercised via the JS flow.)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/orchestrator/mod.rs src-tauri/src/tauri_api/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add search_process_logs tauri command"
```

---

## Task 4: JS types + client transport

**Files:**
- Modify: `src/lib/types.ts` (add `SearchProcessLogsReply`)
- Modify: `src/lib/tauri/client.ts` (add `searchProcessLogs`)

**Interfaces:**
- Produces: `searchProcessLogs(request) => Promise<SearchProcessLogsReply>`.

- [ ] **Step 1: Add the reply type**

In `src/lib/types.ts`, after the `ProcessLogPayload` type (around line 127), add:

```ts
export type SearchProcessLogsReply = {
  matchCount: number;
  matchIndices: number[];
};
```

- [ ] **Step 2: Add the client function**

In `src/lib/tauri/client.ts`, extend the type import (line 4) to include `SearchProcessLogsReply`, then add the function (e.g. after `getGitInfo`):

```ts
import type {
  GitInfo,
  ProjectConfigDocument,
  ProjectId,
  ProjectRecord,
  RunSessionSnapshot,
  SearchProcessLogsReply,
  TerminalSnapshot,
} from "$lib/types";
```

```ts
export async function searchProcessLogs(request: {
  runtimeId: ProcessRuntimeId;
  query: string;
  regex: boolean;
  caseSensitive: boolean;
  upTo: number;
}): Promise<SearchProcessLogsReply> {
  return invoke<SearchProcessLogsReply>("search_process_logs", { request });
}
```

Also add `ProcessRuntimeId` to the `import type` list (it's already exported by `types.ts`).

- [ ] **Step 3: Typecheck**

Run (repo root): `deno task check`
Expected: no new errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/tauri/client.ts
git commit -m "feat: add searchProcessLogs client transport"
```

---

## Task 5: `debounceWithMaxWait` scheduler

**Files:**
- Create: `src/lib/utils/scheduler.ts`
- Create: `src/lib/utils/scheduler.test.ts`

**Interfaces:**
- Produces: `debounceWithMaxWait(fn, wait, maxWait)` → `{ schedule(...args), cancel() }`.

- [ ] **Step 1: Write the failing test**

Create `src/lib/utils/scheduler.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { debounceWithMaxWait } from "./scheduler";

describe("debounceWithMaxWait", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("coalesces calls within the wait window", () => {
    const fn = vi.fn();
    const d = debounceWithMaxWait(fn, 80, 250);
    d.schedule();
    d.schedule();
    d.schedule();
    vi.advanceTimersByTime(79);
    expect(fn).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(fn).toHaveBeenCalledOnce();
  });

  it("forces a flush after maxWait even under continuous calls", () => {
    const fn = vi.fn();
    const d = debounceWithMaxWait(fn, 80, 250);
    for (let i = 0; i < 10; i++) {
      d.schedule();
      vi.advanceTimersByTime(40);
    }
    // 10 * 40 = 400ms > maxWait 250 → flushed exactly once by now.
    expect(fn).toHaveBeenCalled();
  });

  it("cancel prevents a pending invocation", () => {
    const fn = vi.fn();
    const d = debounceWithMaxWait(fn, 80, 250);
    d.schedule();
    d.cancel();
    vi.advanceTimersByTime(1000);
    expect(fn).not.toHaveBeenCalled();
  });

  it("passes the latest args to the callback", () => {
    const fn = vi.fn();
    const d = debounceWithMaxWait(fn, 50, 200);
    d.schedule("a");
    d.schedule("b");
    vi.advanceTimersByTime(50);
    expect(fn).toHaveBeenCalledWith("b");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/lib/utils/scheduler.test.ts`
Expected: FAIL (`debounceWithMaxWait` not defined).

- [ ] **Step 3: Write minimal implementation**

Create `src/lib/utils/scheduler.ts`:

```ts
type Schedule = {
  schedule: () => void;
  cancel: () => void;
};

export function debounceWithMaxWait(
  fn: () => void,
  wait: number,
  maxWait: number,
): Schedule {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let maxTimer: ReturnType<typeof setTimeout> | null = null;

  function invoke(): void {
    cancel();
    fn();
  }

  function cancel(): void {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
    if (maxTimer !== null) {
      clearTimeout(maxTimer);
      maxTimer = null;
    }
  }

  function schedule(): void {
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(invoke, wait);
    if (maxTimer === null) {
      maxTimer = setTimeout(invoke, maxWait);
    }
  }

  return { schedule, cancel };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/lib/utils/scheduler.test.ts`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src/lib/utils/scheduler.ts src/lib/utils/scheduler.test.ts
git commit -m "feat: add debounceWithMaxWait scheduler util"
```

---

## Task 6: `logSearch` resolver (Rust when live+Tauri, else local JS)

**Files:**
- Create: `src/lib/utils/logSearch.ts`
- Create: `src/lib/utils/logSearch.test.ts`

**Interfaces:**
- Produces:
  - `searchLogsLocally(logs: FlatRow[], matcher: Matcher): number[]`
  - `computeMatchIndices(args): Promise<number[]>` where args = `{ logs, matcher, query, options, runtimeId, paused }`.

- [ ] **Step 1: Write the failing test**

Create `src/lib/utils/logSearch.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { searchLogsLocally, computeMatchIndices } from "./logSearch";
import { buildMatcher, type SearchOptions } from "./searchHighlight";
import type { FlatRow } from "$lib/types";

const opts: SearchOptions = { regex: false, caseSensitive: false };

function row(text: string, i: number, stream: FlatRow["stream"] = "stdout"): FlatRow {
  return {
    entryId: i,
    lineIndex: 0,
    isFirstLine: true,
    isContinuation: false,
    text,
    stream,
    timestamp: `2026-01-01T00:00:${i.toString().padStart(2, "0")}Z`,
  };
}

describe("searchLogsLocally", () => {
  it("returns row positions matching the query (case-insensitive)", () => {
    const logs = [row("Listening on 3000", 0), row("worker ready", 1), row("Listening on 3001", 2)];
    const m = buildMatcher("listening", opts);
    expect(searchLogsLocally(logs, m)).toEqual([0, 2]);
  });

  it("matches the stream-prefixed target", () => {
    const logs = [row("boom", 0, "stderr")];
    const m = buildMatcher("stderr", opts);
    expect(searchLogsLocally(logs, m)).toEqual([0]);
  });

  it("strips ANSI before matching", () => {
    const logs = [row("\x1b[32mready\x1b[39m", 0)];
    const m = buildMatcher("ready", opts);
    expect(searchLogsLocally(logs, m)).toEqual([0]);
  });

  it("returns [] for an empty matcher", () => {
    expect(searchLogsLocally([row("x", 0)], buildMatcher("", opts))).toEqual([]);
  });
});

describe("computeMatchIndices", () => {
  it("scans locally when paused", async () => {
    const logs = [row("hit", 0), row("miss", 1), row("hit", 2)];
    const m = buildMatcher("hit", opts);
    const indices = await computeMatchIndices({
      logs,
      matcher: m,
      query: "hit",
      options: opts,
      runtimeId: "rt-1",
      paused: true,
    });
    expect(indices).toEqual([0, 2]);
  });

  it("scans locally when runtimeId is null (dev/tests)", async () => {
    const logs = [row("alpha", 0), row("beta", 1)];
    const m = buildMatcher("alpha", opts);
    const indices = await computeMatchIndices({
      logs,
      matcher: m,
      query: "alpha",
      options: opts,
      runtimeId: null,
      paused: false,
    });
    expect(indices).toEqual([0]);
  });

  it("returns [] for an invalid matcher (regex error)", async () => {
    const logs = [row("x", 0)];
    const m = buildMatcher("(unclosed", { regex: true, caseSensitive: false });
    const indices = await computeMatchIndices({
      logs,
      matcher: m,
      query: "(unclosed",
      options: { regex: true, caseSensitive: false },
      runtimeId: null,
      paused: false,
    });
    expect(indices).toEqual([]);
  });
});

// The Rust (live + Tauri) path is exercised by the Rust store tests (Task 2)
// and the manual smoke test (Task 9); its local fallback is identical to the
// paused/local scan tested above, so it is not re-tested here.
```
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/lib/utils/logSearch.test.ts`
Expected: FAIL (module not found).

- [ ] **Step 3: Write minimal implementation**

Create `src/lib/utils/logSearch.ts`:

```ts
import type { FlatRow } from "$lib/types";
import { stripAnsi } from "$lib/utils/ansi";
import {
  buildMatcher,
  lineMatches,
  type Matcher,
  type SearchOptions,
} from "$lib/utils/searchHighlight";
import { isTauriRuntime } from "$lib/tauri/environment";
import { searchProcessLogs } from "$lib/tauri/client";

export type ComputeMatchArgs = {
  logs: FlatRow[];
  matcher: Matcher;
  query: string;
  options: SearchOptions;
  runtimeId: string | null;
  paused: boolean;
};

/**
 * Scan logs in JS for matching row positions. Used for the paused snapshot
 * and as a fallback in browser dev / tests where no Rust backend exists.
 * Must produce the same positions as `InMemoryLogStore::search` for identical
 * input (same target string, same regex rules).
 */
export function searchLogsLocally(logs: FlatRow[], matcher: Matcher): number[] {
  if (matcher === null || "error" in matcher) return [];
  const indices: number[] = [];
  for (let i = 0; i < logs.length; i++) {
    const row = logs[i];
    if (lineMatches(matcher, `${row.stream} ${stripAnsi(row.text)}`)) {
      indices.push(i);
    }
  }
  return indices;
}

/**
 * Resolve matching row positions. Delegates to the Rust `search_process_logs`
 * command when live + Tauri (off-main-thread, scans the backend buffer);
 * otherwise scans locally. Returns `[]` for an empty or invalid matcher.
 */
export async function computeMatchIndices(args: ComputeMatchArgs): Promise<number[]> {
  const { logs, matcher, query, options, runtimeId, paused } = args;
  if (matcher === null || "error" in matcher) return [];
  if (paused || !isTauriRuntime() || runtimeId === null) {
    return searchLogsLocally(logs, matcher);
  }
  try {
    const reply = await searchProcessLogs({
      runtimeId,
      query,
      regex: options.regex,
      caseSensitive: options.caseSensitive,
      upTo: logs.length,
    });
    return reply.matchIndices;
  } catch {
    return searchLogsLocally(logs, matcher);
  }
}

// Re-export so callers can build a matcher from the same source of truth.
export { buildMatcher };
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/lib/utils/logSearch.test.ts`
Expected: PASS (all local-path tests).

- [ ] **Step 5: Commit**

```bash
git add src/lib/utils/logSearch.ts src/lib/utils/logSearch.test.ts
git commit -m "feat: add logSearch resolver (rust live / js fallback)"
```

---

## Task 7: LogViewer — stop filtering, wire matches, add `runtimeId`

**Files:**
- Modify: `src/lib/components/LogViewer.svelte`
- Modify: `src/routes/+page.svelte:188` (pass `runtimeId`)
- Modify: `src/lib/components/LogViewer.test.ts`

**Interfaces:**
- Consumes: `computeMatchIndices`, `debounceWithMaxWait`, existing `buildMatcher`/`highlightLine`.
- Produces: `LogViewer` prop `runtimeId?: string | null`.

- [ ] **Step 1: Update the LogViewer props + imports**

In `src/lib/components/LogViewer.svelte`:

Extend the imports — replace the `searchHighlight` import block (lines 6–11) and add the new utilities. The matcher/highlight imports stay; add `computeMatchIndices` and `debounceWithMaxWait`:

```ts
  import {
    buildMatcher,
    highlightLine,
    type SearchOptions,
  } from "$lib/utils/searchHighlight";
  import { computeMatchIndices } from "$lib/utils/logSearch";
  import { debounceWithMaxWait } from "$lib/utils/scheduler";
```

Add `runtimeId` to `Props` (after `processName`):

```ts
  type Props = {
    logs: FlatRow[];
    processName: string | null;
    runtimeId?: string | null;
    truncatedCount: number;
    onClear: () => void;
    onActions?: (actions: { copy: () => void; clear: () => void }) => void;
  };

  let { logs, processName, runtimeId = null, truncatedCount, onClear, onActions }: Props =
      $props();
```

- [ ] **Step 2: Replace filtering with match-index state**

Replace the `filteredLogs` block, the two `activeMatchIndex` effects, the `matcherActive`/`matchTotal`/`activeMatchNumber` deriveds, the navigation helpers, **and** the `virtualScroll`/`totalHeight`/`startIndex`/`endIndex`/`visibleItems` block (original lines 74–128 — the latter still references `filteredLogs`, so they move into the new block) with:

```ts
  // All rows are always rendered; search only highlights and navigates.
  const virtualScroll = $derived(
    computeVirtualScroll(scrollTop, viewportHeight, visibleLogs.length),
  );
  const totalHeight = $derived(virtualScroll.totalHeight);
  const startIndex = $derived(virtualScroll.startIndex);
  const endIndex = $derived(virtualScroll.endIndex);
  const visibleItems = $derived(visibleLogs.slice(startIndex, endIndex));

  let matchRowIndices = $state<number[]>([]);
  let activeMatchIndex = $state(0);

  const matcherActive = $derived(matcher !== null && "regex" in matcher);
  const matchTotal = $derived(matcher === null ? null : matchRowIndices.length);
  const activeMatchNumber = $derived(activeMatchIndex + 1);
  const activeMatchRow = $derived(
    matchRowIndices.length > 0 ? matchRowIndices[activeMatchIndex] : -1,
  );

  function scrollToActiveMatch() {
    if (!viewport || activeMatchRow < 0) return;
    const top = activeMatchRow * ROW_HEIGHT - (viewportHeight - ROW_HEIGHT) / 2;
    const maxScroll = totalHeight - viewportHeight;
    viewport.scrollTo({ top: Math.max(0, Math.min(top, maxScroll)) });
  }

  function goToMatch(next: number) {
    const len = matchRowIndices.length;
    if (len === 0) return;
    activeMatchIndex = ((next % len) + len) % len;
    autoScroll = false;
    scrollToActiveMatch();
  }

  function nextMatch() {
    goToMatch(activeMatchIndex + 1);
  }

  function prevMatch() {
    goToMatch(activeMatchIndex - 1);
  }
```

- [ ] **Step 3: Add the debounced refresh scheduler**

Add after the scheduler-less block (near the other `$effect`s, e.g. after the `searchOptions` persistence effect):

```ts
  // Refresh match positions whenever the query, options, pause state, runtime,
  // or row count changes. Debounced for typing (80ms) with a maxWait (250ms)
  // so live streaming can't starve the count indefinitely.
  let searchGeneration = 0;
  const searchScheduler = debounceWithMaxWait(refreshMatches, 80, 250);

  async function refreshMatches() {
    const m = matcher;
    if (m === null || "error" in m) {
      matchRowIndices = [];
      return;
    }
    const generation = ++searchGeneration;
    const indices = await computeMatchIndices({
      logs: visibleLogs,
      matcher: m,
      query,
      options: searchOptions,
      runtimeId: runtimeId ?? null,
      paused,
    });
    if (generation !== searchGeneration) return; // a newer search superseded this one
    matchRowIndices = indices;
    if (activeMatchIndex > indices.length - 1) {
      activeMatchIndex = Math.max(0, indices.length - 1);
    }
  }

  let lastQuerySig = "";
  $effect(() => {
    void query;
    void searchOptions.regex;
    void searchOptions.caseSensitive;
    void runtimeId;
    void paused;
    void visibleLogs.length;
    const sig = `${query}|${searchOptions.regex}|${searchOptions.caseSensitive}`;
    if (sig !== lastQuerySig) {
      lastQuerySig = sig;
      activeMatchIndex = 0;
    }
    searchScheduler.schedule();
  });
```

And ensure the scheduler is cancelled on destroy (extend the existing cleanup `$effect` at lines 258–267 by adding `searchScheduler.cancel();` inside the returned teardown):

```ts
  $effect(() => {
    return () => {
      searchScheduler.cancel();
      if (copyTimer !== null) {
        clearTimeout(copyTimer);
      }
      if (entryCopyTimer !== null) {
        clearTimeout(entryCopyTimer);
      }
    };
  });
```

- [ ] **Step 4: Replace `filteredLogs` references that remain**

There are several remaining uses of `filteredLogs` (copy, entry copy, visual grouping, border corners, empty state) outside the block replaced in Step 2. Replace each with `visibleLogs`:

- `copyLogs` (line 147): `const text = visibleLogs.map(...)`.
- `copyEntry` (line 172): `const entryLines = visibleLogs.filter(...)`.
- `visualGroupIdForIndex` (line 304) and `borderCornerClass` (line 340): replace every `filteredLogs[...]` with `visibleLogs[...]` and `filteredLogs.length` with `visibleLogs.length`.
- Empty state (line 390): `{#if visibleLogs.length === 0}` and message `{query ? "No logs" : "No log line"}` (we no longer say "No matching lines" since rows are never filtered out).

- [ ] **Step 5: Update the active-match row highlight in the template**

In the row `<div>` class (line 405), replace the active-match condition `startIndex + index === activeMatchIndex && matcherActive` with:

```
{startIndex + index === activeMatchRow && matcherActive ? 'bg-surface-hover/60' : ''}
```

- [ ] **Step 6: Update process-switch + clear resets**

In the process-switch `$effect` (lines 231–241), add resets for the new state:

```ts
  $effect(() => {
    if (processName === activeProcessName) {
      return;
    }
    activeProcessName = processName;
    paused = false;
    pausedLogs = null;
    autoScroll = true;
    scrollTop = 0;
    activeMatchIndex = 0;
    matchRowIndices = [];
    searchScheduler.cancel();
  });
```

- [ ] **Step 7: Pass `runtimeId` from the page**

In `src/routes/+page.svelte`, the `<LogViewer>` block (lines 188–194) — add the prop:

```svelte
            <LogViewer
              logs={runtimeStore.logsForSelectedProcess()}
              processName={selectedProcess?.name ?? null}
              runtimeId={runtimeStore.selectedProcessRuntimeId}
              truncatedCount={runtimeStore.truncatedLogCountForSelectedProcess()}
              onClear={() => runtimeStore.clearSelectedProcessLogs()}
              onActions={(actions) => (runtimeStore.logActions = actions)}
            />
```

- [ ] **Step 8: Update LogViewer tests for highlight-only behavior**

In `src/lib/components/LogViewer.test.ts`:

- Replace the first test ("filters lines by a substring query") — rows must now ALL stay rendered:

```ts
  it("keeps all rows rendered and highlights matches (no filtering)", async () => {
    const { container } = render(LogViewer, {
      props: makeProps({
        logs: logs(["listening on 3000", "worker ready", "listening on 3001"]),
      }),
    });
    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    input.value = "LISTENING";
    await fireEvent.input(input);
    await new Promise((r) => setTimeout(r, 0));
    expect(container.textContent).toContain("worker ready");
    const marks = container.querySelectorAll("mark");
    expect(marks.length).toBeGreaterThanOrEqual(2);
  });
```

- Replace the regex test ("treats the query as regex"):

```ts
  it("highlights regex matches without hiding non-matches", async () => {
    const { container } = render(LogViewer, {
      props: makeProps({ logs: logs(["error 42", "warn 7", "error 99"]) }),
    });
    await fireEvent.click(container.querySelector('[aria-label="Toggle regex"]')!);
    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    input.value = "error \\d+";
    await fireEvent.input(input);
    await new Promise((r) => setTimeout(r, 0));
    expect(container.textContent).toContain("warn 7");
    expect(container.querySelectorAll("mark").length).toBe(2);
  });
```

- Keep the invalid-regex test mostly as-is (it asserts the popover + disabled nav). Since the JS matcher detects the error locally and skips the Rust path, this still works. Ensure it waits a tick:

```ts
  it("shows the error popover and disables nav on an invalid regex", async () => {
    const { container, getByText } = render(LogViewer, {
      props: makeProps({ logs: logs(["api listening"]) }),
    });
    await fireEvent.click(container.querySelector('[aria-label="Toggle regex"]')!);
    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    input.value = "(unclosed";
    await fireEvent.input(input);
    await fireEvent.focus(input);
    await new Promise((r) => setTimeout(r, 0));
    expect(getByText(/Unterminated|Invalid|regular expression/i)).toBeInTheDocument();
    expect(container.querySelector('[aria-label="Next match"]')).toBeDisabled();
  });
```

- The Enter/Shift+Enter navigation test still works via the local scan (jsdom is non-Tauri). Add a `await new Promise((r) => setTimeout(r, 0))` after each `fireEvent.input` to let the async local scan resolve before reading the counter:

```ts
  it("navigates matches with Enter / Shift+Enter and wraps around", async () => {
    const { container } = render(LogViewer, {
      props: makeProps({ logs: logs(["a one", "b one", "c one"]) }),
    });
    const counter = () =>
      container.querySelector('[aria-label="Match count"]')?.textContent?.replace(/\s+/g, "");

    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    input.value = "one";
    await fireEvent.input(input);
    await new Promise((r) => setTimeout(r, 0));
    expect(counter()).toBe("1/3");

    await fireEvent.keyDown(input, { key: "Enter" });
    expect(counter()).toBe("2/3");

    await fireEvent.keyDown(input, { key: "Enter" });
    expect(counter()).toBe("3/3");

    await fireEvent.keyDown(input, { key: "Enter" });
    expect(counter()).toBe("1/3");

    await fireEvent.keyDown(input, { key: "Enter", shiftKey: true });
    expect(counter()).toBe("3/3");
  });
```

- [ ] **Step 9: Typecheck + run JS tests**

Run: `deno task check` then `deno task test`
Expected: typecheck clean; all LogViewer + LogToolbar + new util tests pass.

- [ ] **Step 10: Commit**

```bash
git add src/lib/components/LogViewer.svelte src/lib/components/LogViewer.test.ts src/routes/+page.svelte
git commit -m "feat(log-viewer): highlight-only search with rust-backed matches"
```

---

## Task 8: Toolbar — always-mounted nav (no layout shift)

**Files:**
- Modify: `src/lib/components/LogToolbar.svelte`
- Modify: `src/lib/components/LogToolbar.test.ts`

**Interfaces:**
- No prop changes.

- [ ] **Step 1: Update the failing test first**

In `src/lib/components/LogToolbar.test.ts`, replace the "hides the nav group" test (lines 25–29). The nav now stays mounted but is visually hidden when there's no query:

```ts
  it("keeps the nav group mounted but hidden when matchTotal is null", () => {
    const { getByRole } = render(LogToolbar, { props: makeProps() });
    const next = getByRole("button", { name: "Next match" });
    const prev = getByRole("button", { name: "Previous match" });
    // Mounted (no layout shift) but not visible and not interactive.
    expect(next).not.toBeVisible();
    expect(prev).not.toBeVisible();
    expect(next).toBeDisabled();
    expect(prev).toBeDisabled();
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/lib/components/LogToolbar.test.ts`
Expected: FAIL (buttons still visible / or not found).

- [ ] **Step 3: Change the nav block to always render**

In `src/lib/components/LogToolbar.svelte`, replace the `{#if showNav} ... {/if}` block (lines 74–115) with an always-rendered block whose visibility is toggled by class. Replace the whole block with:

```svelte
  <div
    class="flex shrink-0 items-center gap-0.5 {showNav ? '' : 'invisible pointer-events-none'}"
    role="group"
    aria-label="Match navigation"
    aria-hidden={showNav ? undefined : "true"}
  >
    <button
      type="button"
      tabindex="-1"
      disabled={navDisabled}
      onclick={onPrev}
      class="grid h-6 w-6 place-items-center rounded-md text-text-subtle transition-colors duration-75 hover:bg-surface-hover hover:text-text disabled:opacity-55"
      aria-label="Previous match"
      title="Previous match (Shift+Enter)"
    >
      <Icon name="back" size="xs" />
    </button>
    <span
      class="min-w-[34px] text-center text-[10px] tabular-nums text-text-subtle"
      aria-label="Match count"
    >
      {#if regexError}
        <span class="inline-flex text-danger" title={regexError}>
          <Icon name="error" size="xs" />
        </span>
      {:else}
        {activeMatchNumber}/{matchTotal ?? 0}
      {/if}
    </span>
    <button
      type="button"
      tabindex="-1"
      disabled={navDisabled}
      onclick={onNext}
      class="grid h-6 w-6 place-items-center rounded-md text-text-subtle transition-colors duration-75 hover:bg-surface-hover hover:text-text disabled:opacity-55"
      aria-label="Next match"
      title="Next match (Enter)"
    >
      <Icon name="chevron-right" size="xs" />
    </button>
  </div>
```

`showNav` and `navDisabled` (already derived at lines 37–38) keep working: when `matchTotal === null`, `showNav` is false → `invisible pointer-events-none` + `aria-hidden`. When active, normal.

- [ ] **Step 4: Run tests to verify they pass**

Run: `npx vitest run src/lib/components/LogToolbar.test.ts`
Expected: PASS (all toolbar tests, including the rewritten one).

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/LogToolbar.svelte src/lib/components/LogToolbar.test.ts
git commit -m "feat(log-toolbar): always reserve nav space to avoid layout shift"
```

---

## Task 9: Full verification

- [ ] **Step 1: Rust**

Run (workdir `src-tauri`): `cargo test`
Expected: all pass.

- [ ] **Step 2: JS typecheck + tests**

Run: `deno task check` and `deno task test`
Expected: clean + all pass.

- [ ] **Step 3: Manual smoke (optional, if a Tauri dev build is feasible)**

Run: `deno task app -- <path-to-diavola.yml>`
- Type a search → all rows remain, matches highlight, count shows `n/m`.
- Prev/Next arrows + Enter/Shift+Enter move the active match and scroll.
- Clearing the query → nav hides (space reserved), no toolbar reflow.
- Toggle regex / case-sensitive → count updates.
- While logs stream, the count refreshes within ~250ms.
Expected: matches the spec behavior.
