# Highlight-only log search (Rust-backed)

**Date:** 2026-07-08
**Status:** Approved
**Related:** `2026-07-05-log-search-modes-design.md`, `2026-07-06-multiline-log-design.md`

## Goal

The log search bar must **stop filtering** logs and instead **highlight** matching
substrings while keeping every row visible. Prev/next navigation and the match
count remain. The UI must not shift when the search UI appears/disappears.
Match scanning must stay fast under up to 10,000 live-streaming rows, so the
heavy scan runs in **Rust**.

## Non-goals

- No dimming of non-matching rows (matches only are highlighted).
- No change to how logs are streamed or rendered (event payload shape unchanged).
- No change to copy/pause/clear behavior.

## Current state

- `LogViewer.svelte:74` computes `filteredLogs = base.filter(...)` on every
  keystroke **and** every log append, shrinking the virtual-scroll height and
  reflowing the list.
- Navigation (prev/next/count) already exists in `LogToolbar.svelte`, but the
  nav group is mounted conditionally (`{#if showNav}`, `LogToolbar.svelte:74`),
  causing a toolbar reflow when a query is typed or cleared.
- `InMemoryLogStore` (`log_store.rs:12`) retains `VecDeque<ProcessLogPayload>`
  capped at **10,000 payloads**. Its `list()` is never called and `clear()` is
  never invoked from production code — the buffer is effectively write-only.
- JS retains up to **10,000 `FlatRow`** (`MAX_LOG_LINES_PER_PROCESS`,
  `runtime.svelte.ts:50`), flattening payloads on receipt with a JS-side
  `entryCounter`.
- Per-line highlighting is already lazy: only virtualized/visible rows run
  `highlightLine`.

## Design

### 1. Rust — flattened, FlatRow-capped buffer

Repurpose `InMemoryLogStore` to store a **flattened buffer** per `runtime_id`,
capped at **10,000 flat rows** (matching JS `MAX_LOG_LINES_PER_PROCESS`).

- Storage: `HashMap<ProcessRuntimeId, VecDeque<FlatRowEntry>>` where
  `FlatRowEntry { text: String, stream: LogStream }`. Only fields needed for
  search are stored (no `entryId`/`timestamp` — those remain JS-side concerns).
- `append(payload)` flattens the payload with the **same logic as JS**
  (`runtime.svelte.ts:547` `#flattenPayload`): `lines[0]` is the first line,
  the rest are continuations. Entry-pattern grouping already happened upstream
  in `log.rs`, so flattening is a straight line expansion.
- Truncation drops from the front beyond 10,000 flat rows — identical policy to
  JS, so Rust's buffer and JS's `logs` array are **positionally aligned** (same
  order, same length, same cap).
- `entryId` stays JS-assigned; Rust's search buffer does not need it because
  results are **positional row indices**, not entry ids.

Because `list()`/`clear()` have no production callers, replacing the payload
storage breaks nothing.

### 2. Rust — ANSI stripping

Search matches against `"{stream} {ansi_stripped_text}"` (same target as
`LogViewer.svelte:77`). Port the `ESC_SEQ` regex from `ansi.ts:54`
(CSI / OSC / charset / misc escape forms) into a small Rust helper so stripping
parity matches JS. The `regex = "1"` crate is already a dependency.

### 3. Rust — search command

New Tauri command, registered in `lib.rs:134`:

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchProcessLogsRequest {
    pub runtime_id: ProcessRuntimeId,
    pub query: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub up_to: usize, // = JS logs.length at call time
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchProcessLogsReply {
    pub match_count: usize,
    pub match_indices: Vec<u32>, // 0-based positions into the flat buffer
}

#[tauri::command]
pub async fn search_process_logs(
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: SearchProcessLogsRequest,
) -> Result<SearchProcessLogsReply, AppError>
```

Behavior:

- Empty/whitespace query → `{ match_count: 0, match_indices: [] }` (no regex
  built). JS avoids calling in this case anyway.
- Build `regex::Regex`: escape the query with the same algorithm as
  `searchHighlight.ts:10` `escapeRegExp` when `regex == false`; add the `i`
  flag when `case_sensitive == false`.
- On invalid regex: return `Err(AppError)` so JS can surface it. In practice
  JS detects regex errors locally first (see §4) and skips the call, but Rust
  validates defensively.
- Iterate the flat buffer for `runtime_id`, considering only the first
  `min(up_to, len)` rows. For each, build `"{stream} {stripped_text}"` and test
  the regex; collect matching **row positions** into `match_indices`.
- `u32` is used for indices (10,000 fits comfortably; halves payload size vs
  `usize`).

`up_to` closes a small timing race: Rust's buffer can be momentarily ahead of
JS's rendered array during streaming (emit then append happen in `log.rs`'s
callback; JS receives the event asynchronously). By sending `logs.length`, JS
guarantees Rust only reports matches within rows JS has actually rendered, so
navigation indices stay valid.

Wired through `Orchestrator::search_logs(window_key, runtime_id, ...)`,
mirroring `snapshot()` at `orchestrator/mod.rs:232` (lock `inner`, look up the
session by `window_key(&window)` — same `window.label()` helper used by every
other command).

### 4. JS — LogViewer

- **Remove filtering.** Delete `filteredLogs`; `visibleItems` derives directly
  from `visibleLogs` (all rows). The virtual-scroll total height becomes stable
  during search (no more list collapse/scrollbar jump).
- **Keep the local `matcher`** (`buildMatcher`) for two cheap, local purposes:
  1. Per-visible-row substring highlighting via `highlightLine` (unchanged,
     still lazy — only visible rows).
  2. Immediate regex-error detection with no IPC round-trip.
- **New state:**
  - `matchRowIndices: number[]` — positions returned by Rust.
  - `activeMatchIndex: number` — index into `matchRowIndices`.
- **Query/options change (live):** debounce ~80ms → if local matcher is valid,
  `invoke("search_process_logs", { runtimeId, query, regex, caseSensitive,
  upTo: logs.length })` → set `matchRowIndices`, clamp `activeMatchIndex` into
  range, reset to 0 if the query/options changed.
- **Live + streaming:** when a search is active and `logs` changes (new rows
  appended), re-query Rust throttled to one call per ~250ms (leading-edge +
  trailing-edge) so the count stays live without flooding. Each round-trip is
  <5ms. `activeMatchIndex` is clamped into `[0, matchRowIndices.length - 1]`
  after each refresh (it does not try to track a specific logical match across
  shifts — acceptable for v1).
- **Paused:** Rust does not know about the pause snapshot, so when `paused` is
  true, run a **one-time JS scan** over `pausedLogs` using the existing
  `lineMatches(matcher, "{stream} {stripped}")` and collect indices locally.
  The snapshot is frozen (no streaming), so a single scan is cheap and correct.
  This gives a clean split: **Rust for live, JS for paused.**
- **Derived values:**
  - `matchTotal = matchRowIndices.length`.
  - `activeMatchNumber = activeMatchIndex + 1`.
- **Navigation:** `goToMatch(next)` sets `activeMatchIndex` (mod length),
  disables `autoScroll`, and scrolls to
  `matchRowIndices[activeMatchIndex] * ROW_HEIGHT - (viewportHeight - ROW_HEIGHT) / 2`.
- **Active row highlight:** the row whose position equals
  `matchRowIndices[activeMatchIndex]` receives the stronger background class
  (the `bg-surface-hover/60` + `bg-warning/60` mark already in the template).
- **Reset** `matchRowIndices = []` and `activeMatchIndex = 0` on process switch
  and on clear (alongside the existing resets at `LogViewer.svelte:231` and
  `:243`).
- `matcherActive` is still `matcher !== null && "regex" in matcher`; it gates
  both in-viewport highlighting and whether navigation is active.

### 5. JS — toolbar, no layout shift

`LogToolbar.svelte:74` currently mounts the nav group with `{#if showNav}`.
Change it to **always render** the nav group and instead toggle visibility:

- When there is no query (`matchTotal === null`): apply
  `invisible pointer-events-none` so the group occupies its ~70px but is not
  interactive or visible.
- When a query is active: remove those classes; behavior is unchanged.

Because the search input keeps `flex-1` and the nav width is constant, the
toolbar never reflows — whether or not a search is active. The regex-error
indicator inside the input is already absolutely positioned (no shift), and the
mode toggles (regex/case) are always rendered, so no other conditional blocks
affect layout.

### 6. Edge cases

- **Empty query** → no Rust call, `matchRowIndices = []`, nav disabled but
  space reserved (invisible).
- **Regex error** → local matcher surfaces `regexError`; Rust call skipped.
- **Zero matches** → `0/0` displayed, nav buttons disabled.
- **Paused** → JS scan fallback (§4).
- **Process switch / clear** → indices reset.
- **Truncation** → both Rust and JS drop from the front beyond 10,000 flat rows
  by the same count, so indices remain aligned after overflow.

## Performance characteristics

- Rust scan of 10,000 rows: <1ms compute + ~1–3ms IPC.
- Query debounced 80ms; streaming re-query throttled to ~4/sec max.
- Highlighting unchanged (lazy, per visible row).
- No filtering → stable virtual-scroll height → no scrollbar/list shift.

## Testing

### Rust

- Flatten-on-append produces flat rows matching JS expansion; cap enforced at
  10,000; front-dropped on overflow.
- ANSI stripper removes SGR/CSI/OSC/charset/misc sequences (parity spot-checks
  against JS `stripAnsi`).
- `search_process_logs`: literal match, regex match, case-insensitive default,
  `case_sensitive` honored, `up_to` truncates results to the first N rows,
  empty query returns empty, invalid regex returns `Err`.
- Positional indices align with a JS-flattened array for the same input
  payloads (cross-check fixture).

### JS

- All logs render when a search is active (no rows hidden).
- Match count reflects Rust results; navigation moves `activeMatchIndex` and
  scrolls to the right row.
- Toolbar nav group is always in the DOM (no mount/unmount) → no layout shift
  when typing/clearing.
- Paused mode produces correct indices via the JS fallback.

## Files touched (summary)

- `src-tauri/src/infrastructure/log_store.rs` — flattened buffer + search.
- `src-tauri/src/infrastructure/ansi.rs` (new) — ANSI strip helper + tests, or
  inlined in `log_store.rs`.
- `src-tauri/src/application/orchestrator/mod.rs` — `search_logs(...)` method.
- `src-tauri/src/tauri_api/commands.rs` — command + request/reply types.
- `src-tauri/src/lib.rs` — register `search_process_logs`.
- `src/lib/tauri/client.ts` (or equivalent transport) — `searchProcessLogs`.
- `src/lib/components/LogViewer.svelte` — remove filtering, wire Rust results,
  paused JS fallback, resets.
- `src/lib/components/LogToolbar.svelte` — always-rendered nav group.
- Tests on both sides.
