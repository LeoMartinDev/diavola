# Reliable Live Logs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make process logs appear live and reliably under high throughput, without letting multi-line grouping hide logs or virtual scrolling render blank space.

**Architecture:** The backend will emit each physical log line immediately and stop using `logTimestampPattern` as a transport buffer. The frontend will build best-effort visual grouping from already-delivered rows and clamp virtual scroll calculations so stale or oversized `scrollTop` values cannot produce an empty rendered window.

**Tech Stack:** Rust/Tauri 2, Tokio, Svelte 5 runes, TypeScript, Vitest, Testing Library Svelte, Cargo tests.

## Global Constraints

- Preserve live log delivery: no code path may wait for a future log line before emitting the current line to the frontend.
- Do not add dependencies.
- Keep `ProcessLogPayload.lines: string[]` for compatibility with existing frontend/runtime types, but emit one physical line per payload after this change.
- Keep `logTimestampPattern` config fields parseable for now, but do not use them to buffer transport.
- Keep the virtual scroll fixed-row model with `ROW_HEIGHT = 22`.
- Use TDD: each behavior change starts with a failing test and a red-green verification.
- Do not modify unrelated dirty worktree files. Current known unrelated dirty file: `src-tauri/Cargo.lock`.

---

## File Structure

- `src-tauri/src/application/orchestrator/log.rs`
  - Owns process stdout/stderr line reading and Tauri log event emission.
  - Will change from timestamp-regex buffering to immediate one-line payload emission.
  - Will keep the existing `timestamp_pattern` parameter temporarily unused at the call boundary to minimize API churn, then explicitly name it `_timestamp_pattern`.

- `src/lib/utils/virtualScroll.ts`
  - Owns pure fixed-row virtual scroll math.
  - Will clamp `scrollTop` to the maximum valid range and guarantee `startIndex <= endIndex`.

- `src/lib/utils/virtualScroll.test.ts`
  - New focused tests for virtual scroll edge cases.

- `src/lib/components/LogViewer.svelte`
  - Owns log rendering, autoscroll, search, and visual grouping.
  - Will keep accepting `FlatRow[]`, but visual grouping should be computed from rows already present instead of requiring backend multi-line payloads.
  - Will update `scrollTop` after programmatic autoscroll so virtual scroll state follows the DOM.

- `src/lib/components/LogViewer.test.ts`
  - Will gain a test proving adjacent continuation-like lines render as a visual group while still being individual rows.

---

### Task 1: Backend Immediate Line Emission

**Files:**
- Modify: `src-tauri/src/application/orchestrator/log.rs:18-131`

**Interfaces:**
- Consumes: `spawn_log_task<R, F>(..., append_log_fn: F, timestamp_pattern: Option<regex::Regex>)`
- Produces: Same public function signature, but runtime behavior emits `ProcessLogPayload { lines: vec![line], timestamp: Utc::now() }` for every physical line as soon as `next_line()` returns it.

- [ ] **Step 1: Write the failing tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `src-tauri/src/application/orchestrator/log.rs`. Replace the currently unused imports with the imports below and append the async test.

```rust
    use regex::Regex;
    use std::{future::Future, pin::Pin, sync::Arc};
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn non_timestamp_lines_are_emitted_without_waiting_for_next_timestamp() {
        let appended = Arc::new(Mutex::new(Vec::<ProcessLogPayload>::new()));
        let appended_for_fn = appended.clone();

        let append_fn = move |payload: ProcessLogPayload| -> Pin<Box<dyn Future<Output = ()> + Send>> {
            let appended = appended_for_fn.clone();
            Box::pin(async move {
                appended.lock().await.push(payload);
            })
        };

        append_lines(
            vec![
                r#"method: "GET""#.to_string(),
                r#"clientRelease: "DEV""#.to_string(),
                r#"currentFiscalYearConfiguration: {"#.to_string(),
                r#""year": 2025"#.to_string(),
                r#"}"#.to_string(),
                r#"statusCode: 500"#.to_string(),
            ],
            LogStream::Stdout,
            &RunSessionId::new(),
            &ProcessRuntimeId::new(),
            "api",
            Some(Regex::new(r"^\[\d{2}:\d{2}:\d{2}\]").unwrap()),
            &append_fn,
        )
        .await;

        let payloads = appended.lock().await;
        assert_eq!(payloads.len(), 6);
        assert_eq!(payloads[0].lines, vec![r#"method: "GET""#]);
        assert_eq!(payloads[5].lines, vec![r#"statusCode: 500"#]);
    }
```

Also add this helper signature above the test if it does not exist yet. The test should call real production code, so the helper must be production code introduced in Step 3. In Step 1, only write the test; leave the helper missing so the red run proves the behavior is not implemented.

```rust
// Expected production helper signature, intentionally missing during RED:
// async fn append_lines(
//     lines: Vec<String>,
//     stream: LogStream,
//     session_id: &RunSessionId,
//     runtime_id: &ProcessRuntimeId,
//     process_name: &str,
//     timestamp_pattern: Option<regex::Regex>,
//     append_log_fn: &(dyn Fn(ProcessLogPayload) -> Pin<Box<dyn Future<Output = ()> + Send>> + Sync),
// )
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib non_timestamp_lines_are_emitted_without_waiting_for_next_timestamp --manifest-path src-tauri/Cargo.toml`

Expected: FAIL to compile with an error like `cannot find function append_lines in this scope`.

- [ ] **Step 3: Write minimal production implementation**

In `src-tauri/src/application/orchestrator/log.rs`, introduce a small helper above `spawn_log_task` and simplify `spawn_log_task` to use it. The helper accepts `timestamp_pattern` only to preserve the nearby call contract; it must not use it for transport buffering.

```rust
async fn append_lines(
    lines: Vec<String>,
    stream: LogStream,
    session_id: &RunSessionId,
    runtime_id: &ProcessRuntimeId,
    process_name: &str,
    _timestamp_pattern: Option<regex::Regex>,
    append_log_fn: &(dyn Fn(ProcessLogPayload) -> Pin<Box<dyn Future<Output = ()> + Send>> + Sync),
) {
    for line in lines {
        let payload = ProcessLogPayload {
            session_id: session_id.clone(),
            runtime_id: runtime_id.clone(),
            process_name: process_name.to_string(),
            stream,
            lines: vec![line],
            timestamp: Utc::now(),
        };
        append_log_fn(payload).await;
    }
}
```

Replace the body of the `tokio::spawn(async move { ... })` block with immediate line emission that also emits to Tauri. Use this full block for `spawn_log_task`:

```rust
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
        while let Ok(Some(line)) = lines_reader.next_line().await {
            let _ = log_tx.send(line.clone());
            let payload = ProcessLogPayload {
                session_id: session_id.clone(),
                runtime_id: runtime_id.clone(),
                process_name: process_name.clone(),
                stream,
                lines: vec![line],
                timestamp: Utc::now(),
            };
            let _ = app_handle.emit_to(
                &window_key,
                PROCESS_LOG_EVENT,
                ProcessLogEvent {
                    payload: payload.clone(),
                },
            );
            append_log_fn(payload).await;
        }

        let _ = timestamp_pattern;
    });
}
```

Note: The helper and `spawn_log_task` duplicate payload construction for now. Keep this duplication in Task 1 to avoid changing Tauri emission behavior while proving the regression. Task 2 removes the duplication.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib non_timestamp_lines_are_emitted_without_waiting_for_next_timestamp --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Run related Rust tests**

Run: `cargo test --lib log --manifest-path src-tauri/Cargo.toml`

Expected: PASS for log/readiness tests. Existing warnings may remain in this task, but no test failures are acceptable.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/application/orchestrator/log.rs
git commit -m "fix(log): emit process lines immediately"
```

---

### Task 2: Deduplicate Backend Payload Creation

**Files:**
- Modify: `src-tauri/src/application/orchestrator/log.rs:18-120`

**Interfaces:**
- Consumes: `append_lines(...)` from Task 1.
- Produces: `make_log_payload(session_id, runtime_id, process_name, stream, line) -> ProcessLogPayload` used by both tests and `spawn_log_task`.

- [ ] **Step 1: Write the failing test**

Add this test inside `src-tauri/src/application/orchestrator/log.rs` tests:

```rust
    #[test]
    fn make_log_payload_wraps_one_physical_line() {
        let session_id = RunSessionId::new();
        let runtime_id = ProcessRuntimeId::new();

        let payload = make_log_payload(
            &session_id,
            &runtime_id,
            "api",
            LogStream::Stdout,
            r#"statusCode: 500"#.to_string(),
        );

        assert_eq!(payload.session_id, session_id);
        assert_eq!(payload.runtime_id, runtime_id);
        assert_eq!(payload.process_name, "api");
        assert_eq!(payload.stream, LogStream::Stdout);
        assert_eq!(payload.lines, vec![r#"statusCode: 500"#]);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib make_log_payload_wraps_one_physical_line --manifest-path src-tauri/Cargo.toml`

Expected: FAIL to compile with `cannot find function make_log_payload in this scope`.

- [ ] **Step 3: Write minimal implementation**

Add this helper above `append_lines`:

```rust
fn make_log_payload(
    session_id: &RunSessionId,
    runtime_id: &ProcessRuntimeId,
    process_name: &str,
    stream: LogStream,
    line: String,
) -> ProcessLogPayload {
    ProcessLogPayload {
        session_id: session_id.clone(),
        runtime_id: runtime_id.clone(),
        process_name: process_name.to_string(),
        stream,
        lines: vec![line],
        timestamp: Utc::now(),
    }
}
```

Update `append_lines` to call `make_log_payload`:

```rust
async fn append_lines(
    lines: Vec<String>,
    stream: LogStream,
    session_id: &RunSessionId,
    runtime_id: &ProcessRuntimeId,
    process_name: &str,
    _timestamp_pattern: Option<regex::Regex>,
    append_log_fn: &(dyn Fn(ProcessLogPayload) -> Pin<Box<dyn Future<Output = ()> + Send>> + Sync),
) {
    for line in lines {
        append_log_fn(make_log_payload(
            session_id,
            runtime_id,
            process_name,
            stream,
            line,
        ))
        .await;
    }
}
```

Update `spawn_log_task` loop payload construction to call `make_log_payload`:

```rust
            let payload = make_log_payload(
                &session_id,
                &runtime_id,
                &process_name,
                stream,
                line,
            );
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib make_log_payload_wraps_one_physical_line --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Run related Rust tests**

Run: `cargo test --lib log --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/application/orchestrator/log.rs
git commit -m "refactor(log): share one-line payload creation"
```

---

### Task 3: Clamp Virtual Scroll Ranges

**Files:**
- Create: `src/lib/utils/virtualScroll.test.ts`
- Modify: `src/lib/utils/virtualScroll.ts:1-14`

**Interfaces:**
- Consumes: `computeVirtualScroll(scrollTop: number, viewportHeight: number, totalItems: number)`.
- Produces: Same function signature with clamped output where `0 <= startIndex <= endIndex <= totalItems` and `scrollTop` beyond the current content cannot produce an empty rendered range when `totalItems > 0`.

- [ ] **Step 1: Write the failing tests**

Create `src/lib/utils/virtualScroll.test.ts`:

```ts
import { describe, expect, it } from "vitest";

import { computeVirtualScroll } from "./virtualScroll";

describe("computeVirtualScroll", () => {
  it("clamps stale scrollTop when the list becomes shorter", () => {
    const result = computeVirtualScroll(220_000, 500, 1_000);

    expect(result.totalHeight).toBe(22_000);
    expect(result.startIndex).toBeLessThan(result.endIndex);
    expect(result.endIndex).toBe(1_000);
  });

  it("returns an empty range for an empty list", () => {
    const result = computeVirtualScroll(10_000, 500, 0);

    expect(result.totalHeight).toBe(0);
    expect(result.startIndex).toBe(0);
    expect(result.endIndex).toBe(0);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/lib/utils/virtualScroll.test.ts`

Expected: FAIL on `startIndex` being greater than `endIndex` for the stale `scrollTop` case.

- [ ] **Step 3: Write minimal implementation**

Replace `src/lib/utils/virtualScroll.ts` with:

```ts
const ROW_HEIGHT = 22;
const OVERSCAN = 10;

export function computeVirtualScroll(scrollTop: number, viewportHeight: number, totalItems: number) {
  const totalHeight = totalItems * ROW_HEIGHT;
  if (totalItems <= 0) {
    return { totalHeight, startIndex: 0, endIndex: 0, rowHeight: ROW_HEIGHT, overscan: OVERSCAN };
  }

  const maxScrollTop = Math.max(0, totalHeight - viewportHeight);
  const clampedScrollTop = Math.max(0, Math.min(scrollTop, maxScrollTop));
  const startIndex = Math.max(0, Math.floor(clampedScrollTop / ROW_HEIGHT) - OVERSCAN);
  const endIndex = Math.min(
    totalItems,
    Math.ceil((clampedScrollTop + viewportHeight) / ROW_HEIGHT) + OVERSCAN,
  );

  return { totalHeight, startIndex, endIndex, rowHeight: ROW_HEIGHT, overscan: OVERSCAN };
}

export function isAtBottom(scrollTop: number, viewportHeight: number, scrollHeight: number, threshold = 4): boolean {
  return scrollTop + viewportHeight >= scrollHeight - threshold;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/lib/utils/virtualScroll.test.ts`

Expected: PASS.

- [ ] **Step 5: Run LogViewer tests too**

Run: `npx vitest run src/lib/utils/virtualScroll.test.ts src/lib/components/LogViewer.test.ts`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lib/utils/virtualScroll.ts src/lib/utils/virtualScroll.test.ts
git commit -m "fix(log-viewer): clamp virtual scroll range"
```

---

### Task 4: Synchronize Programmatic Autoscroll State

**Files:**
- Modify: `src/lib/components/LogViewer.svelte:243-251`
- Modify: `src/lib/components/LogViewer.test.ts`

**Interfaces:**
- Consumes: `LogViewer` props `{ logs, processName, truncatedCount, onClear }`.
- Produces: Programmatic autoscroll updates both DOM scroll position and component `scrollTop`/`viewportHeight` state in the same animation frame.

- [ ] **Step 1: Write the failing test**

Append this test to `src/lib/components/LogViewer.test.ts`. It uses a real rendered component and a controlled `scrollTo` stub that mutates `scrollTop` like the browser would.

```ts
describe("LogViewer autoscroll", () => {
  it("keeps rendering newest rows after many logs arrive while pinned to bottom", async () => {
    Element.prototype.scrollTo = vi.fn(function (this: Element, options?: ScrollToOptions | number) {
      if (typeof options === "object" && options !== null && "top" in options) {
        Object.defineProperty(this, "scrollTop", {
          configurable: true,
          value: Number(options.top ?? 0),
        });
      }
    }) as unknown as typeof Element.prototype.scrollTo;

    const initialLogs = logs(Array.from({ length: 20 }, (_, i) => `line ${i}`));
    const { container, rerender } = render(LogViewer, {
      props: makeProps({ logs: initialLogs }),
    });

    const viewport = container.querySelector('[data-native-selectable="logs"]') as HTMLDivElement;
    Object.defineProperty(viewport, "clientHeight", { configurable: true, value: 220 });
    Object.defineProperty(viewport, "scrollHeight", { configurable: true, value: 440 });
    Object.defineProperty(viewport, "scrollTop", { configurable: true, value: 220 });
    await fireEvent.scroll(viewport);

    const nextLogs = logs(Array.from({ length: 1_000 }, (_, i) => `line ${i}`));
    Object.defineProperty(viewport, "scrollHeight", { configurable: true, value: 22_000 });
    await rerender(makeProps({ logs: nextLogs }));

    await new Promise((resolve) => requestAnimationFrame(resolve));

    expect(container.textContent).toContain("line 999");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/lib/components/LogViewer.test.ts -t "keeps rendering newest rows"`

Expected: FAIL because the newest row is not rendered after programmatic autoscroll state falls behind.

- [ ] **Step 3: Write minimal implementation**

In `src/lib/components/LogViewer.svelte`, replace the autoscroll effect with this version:

```svelte
  $effect(() => {
    filteredLogs.length;
    autoScroll;
    paused;
    if (autoScroll && !paused && viewport) {
      requestAnimationFrame(() => {
        if (!viewport) return;
        const top = viewport.scrollHeight;
        viewport.scrollTo({ top });
        scrollTop = viewport.scrollTop;
        viewportHeight = viewport.clientHeight;
      });
    }
  });
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/lib/components/LogViewer.test.ts -t "keeps rendering newest rows"`

Expected: PASS.

- [ ] **Step 5: Run related frontend tests**

Run: `npx vitest run src/lib/components/LogViewer.test.ts src/lib/utils/virtualScroll.test.ts`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/LogViewer.svelte src/lib/components/LogViewer.test.ts
git commit -m "fix(log-viewer): sync autoscroll state"
```

---

### Task 5: Frontend Best-Effort Visual Grouping Without Backend Buffering

**Files:**
- Modify: `src/lib/components/LogViewer.svelte:277-292, 331-340`
- Modify: `src/lib/components/LogViewer.test.ts`

**Interfaces:**
- Consumes: `FlatRow[]` where each backend-emitted physical line may have a unique `entryId`.
- Produces: `visualGroupIdForIndex(index: number): string | number` and `visualBorderCornerClass(index: number): string` local logic in `LogViewer.svelte` so adjacent continuation-like rows are visually grouped even when backend emits one line per payload.

- [ ] **Step 1: Write the failing test**

Append this test to `src/lib/components/LogViewer.test.ts`:

```ts
  it("visually groups object-like continuation rows even when each line is a separate payload", async () => {
    const objectRows = [
      makeRow('currentFiscalYearConfiguration: {', 1),
      makeRow('"fiscalRegime": "is",', 2),
      makeRow('"year": 2025', 3),
      makeRow('}', 4),
      makeRow('responseTimeMs: 21', 5),
    ];

    const { container } = render(LogViewer, {
      props: makeProps({ logs: objectRows }),
    });

    const renderedRows = Array.from(container.querySelectorAll('[data-log-row="true"]'));
    expect(renderedRows[0].className).toContain("rounded-tl");
    expect(renderedRows[1].className).not.toContain("rounded-tl");
    expect(renderedRows[2].className).not.toContain("rounded-bl");
    expect(renderedRows[3].className).toContain("rounded-bl");
    expect(renderedRows[4].className).toContain("rounded-tl");
    expect(renderedRows[4].className).toContain("rounded-bl");
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/lib/components/LogViewer.test.ts -t "visually groups object-like continuation rows"`

Expected: FAIL because each row currently groups only by equal `entryId`.

- [ ] **Step 3: Write minimal implementation**

In `src/lib/components/LogViewer.svelte`, add these helpers near the existing `borderCornerClass` derived function:

```svelte
  function isObjectContinuation(row: FlatRow): boolean {
    const text = stripAnsi(row.text).trimStart();
    return text.startsWith('"') || text === "}" || text === "}," || text === "]" || text === "],";
  }

  function visualGroupIdForIndex(indexInLogs: number): number | string {
    const row = filteredLogs[indexInLogs];
    if (!row) return `missing-${indexInLogs}`;
    if (row.isContinuation) return row.entryId;
    if (!isObjectContinuation(row)) return row.entryId;

    for (let i = indexInLogs - 1; i >= 0; i -= 1) {
      const previous = filteredLogs[i];
      if (!previous) break;
      const previousText = stripAnsi(previous.text).trimEnd();
      if (previousText.endsWith("{") || previousText.endsWith("[")) {
        return `object-${previous.entryId}`;
      }
      if (!isObjectContinuation(previous)) break;
    }

    return row.entryId;
  }
```

Replace the existing `borderCornerClass` derived function with:

```svelte
  const borderCornerClass = $derived((indexInLogs: number): string => {
    const row = filteredLogs[indexInLogs];
    if (!row) return "";
    const currentGroup = visualGroupIdForIndex(indexInLogs);
    const prev = indexInLogs > 0 ? filteredLogs[indexInLogs - 1] : null;
    const next =
      indexInLogs < filteredLogs.length - 1
        ? filteredLogs[indexInLogs + 1]
        : null;
    const sameAsPrev = prev !== null && visualGroupIdForIndex(indexInLogs - 1) === currentGroup;
    const sameAsNext = next !== null && visualGroupIdForIndex(indexInLogs + 1) === currentGroup;

    if (!sameAsPrev && !sameAsNext) return "rounded-tl rounded-bl";
    if (!sameAsPrev) return "rounded-tl";
    if (!sameAsNext) return "rounded-bl";
    return "";
  });
```

Add `data-log-row="true"` to the rendered absolute row `<div>` in the `{#each visibleItems ...}` block:

```svelte
            data-log-row="true"
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/lib/components/LogViewer.test.ts -t "visually groups object-like continuation rows"`

Expected: PASS.

- [ ] **Step 5: Run all LogViewer tests**

Run: `npx vitest run src/lib/components/LogViewer.test.ts`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/LogViewer.svelte src/lib/components/LogViewer.test.ts
git commit -m "feat(log-viewer): group object-like log rows visually"
```

---

### Task 6: Remove Misleading Demo Global Timestamp Pattern

**Files:**
- Modify: `diavola.yml:1-123`

**Interfaces:**
- Consumes: Existing sample project config.
- Produces: Demo config no longer applies a global `logTimestampPattern` to every process. The multi-line demo may keep a per-process pattern only if the process emits lines in that pattern and the backend no longer buffers transport.

- [ ] **Step 1: Edit config**

Remove the root-level line:

```yaml
logTimestampPattern: "^\\[\\d{2}:\\d{2}:\\d{2}\\]"
```

If preserving the multi-line demo hint is useful, add the same value under only `processes.multi-line-demo`:

```yaml
    logTimestampPattern: "^\\[\\d{2}:\\d{2}:\\d{2}\\]"
```

Do not add a global pattern back.

- [ ] **Step 2: Run config-related frontend tests**

Run: `npx vitest run src/lib/config/editorModel.test.ts src/lib/config/validation.test.ts`

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add diavola.yml
git commit -m "chore(config): avoid global log timestamp grouping"
```

---

### Task 7: Final Verification

**Files:**
- No source changes expected.

**Interfaces:**
- Consumes: All previous task outputs.
- Produces: Fresh verification evidence for frontend type checking, focused tests, and Rust lib log tests.

- [ ] **Step 1: Run Svelte diagnostics**

Run: `npx svelte-check`

Expected: `svelte-check found 0 errors and 0 warnings`.

- [ ] **Step 2: Run focused frontend tests**

Run: `npx vitest run src/lib/components/LogViewer.test.ts src/lib/utils/virtualScroll.test.ts src/lib/config/editorModel.test.ts src/lib/config/validation.test.ts`

Expected: PASS.

- [ ] **Step 3: Run focused Rust tests**

Run: `cargo test --lib log --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 4: Check worktree**

Run: `git status --short`

Expected: Only intentional files from this plan are modified or committed. Do not revert unrelated pre-existing changes such as `src-tauri/Cargo.lock` unless the user explicitly asks.

- [ ] **Step 5: Document full Rust test limitation if still present**

Run: `cargo test log --manifest-path src-tauri/Cargo.toml`

Expected: If it still fails with `_EMBED_INFO_PLIST is already defined` in `tests/session_lifecycle.rs`, report it as an existing full-test-suite blocker separate from the log fix. Do not attempt to fix it in this plan.

---

## Self-Review Notes

- Spec coverage: backend live delivery is covered by Tasks 1-2; virtual scroll blank-space prevention is covered by Tasks 3-4; visual grouping without backend buffering is covered by Task 5; config safety is covered by Task 6; verification is covered by Task 7.
- Placeholder scan: no TBD/TODO placeholders remain; all code steps include concrete snippets and commands.
- Type consistency: `ProcessLogPayload`, `FlatRow`, `LogStream`, `RunSessionId`, `ProcessRuntimeId`, and `computeVirtualScroll` signatures match the current codebase. Task 1 preserves the `spawn_log_task` call signature to avoid touching orchestrator call sites.
