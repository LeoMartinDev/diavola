# Log Search Modes & Match Navigation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add regex search, case-sensitive search, and prev/next match navigation (with active-match highlighting) to the log viewer.

**Architecture:** A shared `Matcher` (built once from query + options) drives filtering, highlighting, and navigation from a single source of truth in `searchHighlight.ts`. Because filtering keeps only matching lines, the match total is simply `filteredLogs.length` and navigation indexes directly into it — no separate "matching indices" array. The toolbar gains a navigation group and two mode toggles; a floating popover surfaces regex errors without causing layout shift.

**Tech Stack:** Svelte 5 (runes: `$state`/`$derived`/`$props`/`$bindable`/`$effect`), TypeScript, Tailwind v4, Vitest + @testing-library/svelte, jsdom.

**Spec:** `docs/superpowers/specs/2026-07-05-log-search-modes-design.md`

## Global Constraints

- Test command: `deno task test` (runs `vitest run`). Run a single file: `deno task test <path>`.
- Typecheck command: `deno task check` (runs `svelte-kit sync && svelte-check`).
- Tests use globals (`describe`/`it`/`expect` are available without import) but, following repo convention, import them explicitly from `vitest`. DOM assertions use `@testing-library/jest-dom/vitest` matchers (already in `src/tests/setup.ts`).
- Svelte 5 runes only — no stores, no `let`-reactive. Use `$state`, `$derived`, `$derived.by`, `$props`, `$bindable`, `$effect`.
- Theme tokens are Tailwind v4 utilities derived from `src/app.css` (e.g. `bg-surface-raised`, `text-text-subtle`, `border-danger`, `bg-accent/15`). Reuse existing classes — do not hardcode hex values.
- Buttons are queried in tests by accessible name (`aria-label`). Every interactive control MUST have a stable `aria-label`.
- The `Icon` component (`src/lib/components/ui/Icon.svelte`) maps names to `@lucide/svelte` icons. Available names used here: `search`, `error` (CircleX), `back` (ChevronLeft), `chevron-right`, `play`, `pause`, `clear`, `scroll-down`. **No new icons are needed.**
- Keep the `.log-search` class on the search input and the exported `focusSearch()` — `LogViewer` focuses it via `document.querySelector(".log-search")` on the `/` shortcut.
- Frequent small commits, one per task step group.

---

## File Structure

- **Modify** `src/lib/utils/searchHighlight.ts` — single source of truth for matching: `buildMatcher`, `lineMatches`, `highlightLine`, `escapeRegExp`. Remove `countMatches`.
- **Create** `src/lib/utils/searchHighlight.test.ts` — unit tests for the matcher helpers.
- **Modify** `src/lib/components/LogToolbar.svelte` — add nav group, mode toggles, floating error popover; new props; remove the absolute match-count badge.
- **Create** `src/lib/components/LogToolbar.test.ts` — component tests for the toolbar.
- **Modify** `src/lib/components/LogViewer.svelte` — wire `searchOptions` state (persisted), `matcher`, navigation logic, keyboard shortcuts, active-match highlighting; update toolbar props; use `highlightLine(line, matcher)`.
- **Create** `src/lib/components/LogViewer.test.ts` — integration tests for filtering, navigation, wrap, active highlight, error state.

---

## Task 1: Matcher logic in `searchHighlight.ts`

**Files:**
- Modify: `src/lib/utils/searchHighlight.ts` (full rewrite)
- Test: `src/lib/utils/searchHighlight.test.ts` (new)

**Interfaces:**
- Consumes: nothing (leaf module).
- Produces: `SearchOptions`, `Matcher`, `buildMatcher(query, options)`, `lineMatches(matcher, text)`, `highlightLine(text, matcher)` (signature change: was `(text, query)`), `escapeRegExp`, `TextSegment`. Tasks 2 and 3 import these names verbatim.

- [ ] **Step 1: Write the failing tests**

Create `src/lib/utils/searchHighlight.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import {
  buildMatcher,
  escapeRegExp,
  highlightLine,
  lineMatches,
} from "./searchHighlight";

const opts = (regex = false, caseSensitive = false) => ({ regex, caseSensitive });

describe("escapeRegExp", () => {
  it("escapes regex metacharacters", () => {
    expect(escapeRegExp("a.b*c")).toBe("a\\.b\\*c");
  });
});

describe("buildMatcher", () => {
  it("returns null for an empty query", () => {
    expect(buildMatcher("", opts())).toBeNull();
    expect(buildMatcher("   ", opts())).toBeNull();
  });

  it("builds an escaped substring matcher (case-insensitive by default)", () => {
    const m = buildMatcher("a.b", opts());
    if (m === null || "error" in m) throw new Error("expected a valid matcher");
    // "." must NOT act as a wildcard in substring mode
    expect(lineMatches(m, "axb")).toBe(false);
    expect(lineMatches(m, "a.b here")).toBe(true);
  });

  it("treats the query as a raw regex in regex mode", () => {
    const m = buildMatcher("a.b", opts(true));
    if (m === null || "error" in m) throw new Error("expected a valid matcher");
    expect(lineMatches(m, "axb")).toBe(true);
    expect(lineMatches(m, "a.b")).toBe(true);
  });

  it("honors case-sensitive mode", () => {
    const ci = buildMatcher("Error", opts(false, false));
    const cs = buildMatcher("Error", opts(false, true));
    if (ci === null || "error" in ci) throw new Error("expected ci matcher");
    if (cs === null || "error" in cs) throw new Error("expected cs matcher");
    expect(lineMatches(ci, "error")).toBe(true);
    expect(lineMatches(cs, "error")).toBe(false);
  });

  it("returns { error } for an invalid regex", () => {
    const m = buildMatcher("(unclosed", opts(true));
    if (m === null || "regex" in m) throw new Error("expected an error matcher");
    expect(typeof m.error).toBe("string");
    expect(m.error.length).toBeGreaterThan(0);
  });
});

describe("lineMatches", () => {
  it("returns false for null and error matchers", () => {
    expect(lineMatches(null, "anything")).toBe(false);
    expect(lineMatches({ error: "bad" }, "anything")).toBe(false);
  });
});

describe("highlightLine", () => {
  it("returns a single non-match segment for null/error matchers", () => {
    expect(highlightLine("hello", null)).toEqual([{ text: "hello", match: false }]);
    expect(highlightLine("hello", { error: "bad" })).toEqual([
      { text: "hello", match: false },
    ]);
  });

  it("highlights all case-insensitive substring occurrences", () => {
    const m = buildMatcher("a", opts());
    if (m === null || "error" in m) throw new Error("expected matcher");
    expect(highlightLine("banana", m)).toEqual([
      { text: "b", match: false },
      { text: "a", match: true },
      { text: "n", match: false },
      { text: "a", match: true },
      { text: "n", match: false },
      { text: "a", match: true },
    ]);
  });

  it("highlights regex matches", () => {
    const m = buildMatcher("\\d+", opts(true));
    if (m === null || "error" in m) throw new Error("expected matcher");
    expect(highlightLine("a1b22c", m)).toEqual([
      { text: "a", match: false },
      { text: "1", match: true },
      { text: "b", match: false },
      { text: "22", match: true },
      { text: "c", match: false },
    ]);
  });

  it("does not infinite-loop on zero-width regex matches", () => {
    const m = buildMatcher("a*", opts(true));
    if (m === null || "error" in m) throw new Error("expected matcher");
    const segs = highlightLine("baaab", m);
    // Concatenated text must equal the original (no chars lost/duplicated).
    expect(segs.map((s) => s.text).join("")).toBe("baaab");
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `deno task test src/lib/utils/searchHighlight.test.ts`
Expected: FAIL — `buildMatcher`/`lineMatches` are not exported; `highlightLine` signature is `(text, query)` so it won't accept a `Matcher`.

- [ ] **Step 3: Rewrite `searchHighlight.ts`**

Replace the entire contents of `src/lib/utils/searchHighlight.ts` with:

```ts
export type TextSegment = { text: string; match: boolean };

export type SearchOptions = { regex: boolean; caseSensitive: boolean };

export type Matcher =
  | null // query empty — no filter
  | { regex: RegExp } // valid matcher
  | { error: string }; // invalid regex — surfaced in UI

export function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function buildMatcher(
  query: string,
  options: SearchOptions,
): Matcher {
  const trimmed = query.trim();
  if (trimmed.length === 0) return null;
  const flags = options.caseSensitive ? "g" : "gi";
  const source = options.regex ? trimmed : escapeRegExp(trimmed);
  try {
    return { regex: new RegExp(source, flags) };
  } catch (err) {
    return { error: err instanceof Error ? err.message : String(err) };
  }
}

function regexOf(matcher: Matcher): RegExp | null {
  return matcher && "regex" in matcher ? matcher.regex : null;
}

export function lineMatches(matcher: Matcher, text: string): boolean {
  const re = regexOf(matcher);
  if (!re) return false;
  re.lastIndex = 0;
  return re.test(text);
}

export function highlightLine(
  text: string,
  matcher: Matcher,
): TextSegment[] {
  const re = regexOf(matcher);
  if (!re) return [{ text, match: false }];

  const segments: TextSegment[] = [];
  re.lastIndex = 0;
  let lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    if (m[0] === "") {
      // Zero-width match (e.g. `a*`): advance to avoid an infinite loop.
      re.lastIndex++;
      continue;
    }
    if (m.index > lastIndex) {
      segments.push({ text: text.slice(lastIndex, m.index), match: false });
    }
    segments.push({ text: m[0], match: true });
    lastIndex = re.lastIndex;
  }
  if (lastIndex < text.length) {
    segments.push({ text: text.slice(lastIndex), match: false });
  }
  return segments;
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `deno task test src/lib/utils/searchHighlight.test.ts`
Expected: PASS — all assertions green.

- [ ] **Step 5: Typecheck**

Run: `deno task check`
Expected: no errors. (Note: `LogViewer.svelte` still imports the old `countMatches`/`highlightLine(text, query)` signature — this will temporarily error. Fix it in Task 3. To keep this task independently green, comment-check is OK to have `LogViewer.svelte` errors at this checkpoint ONLY if you commit just `searchHighlight.ts` + its test. If your tooling fails the whole `check` on any error, proceed to Task 3's import fix immediately and run `check` again at the end of Task 3.)

- [ ] **Step 6: Commit**

```bash
git add src/lib/utils/searchHighlight.ts src/lib/utils/searchHighlight.test.ts
git commit -m "feat(logs): add buildMatcher/lineMatches with regex & case modes"
```

---

## Task 2: Toolbar UI in `LogToolbar.svelte`

**Files:**
- Modify: `src/lib/components/LogToolbar.svelte` (full rewrite of template + props)
- Test: `src/lib/components/LogToolbar.test.ts` (new — no existing toolbar tests)

**Interfaces:**
- Consumes: `Icon` component (`$lib/components/ui/Icon.svelte`).
- Produces: a `LogToolbar` component with these props (Task 3 wires them):
  ```ts
  type Props = {
    query: string;                 // $bindable
    autoScroll: boolean;           // $bindable
    paused: boolean;               // $bindable
    matchTotal: number | null;     // null hides nav; 0 shows nav disabled
    activeMatchNumber: number;     // 1-based
    regexError: string | null;     // when set: nav disabled, ⚠ shown, popover shown
    onPrev: () => void;
    onNext: () => void;
    regex: boolean;                // $bindable
    caseSensitive: boolean;        // $bindable
    onTogglePause: () => void;
    onClear: () => void;
  };
  ```
  plus the exported `focusSearch()` function. Keeps the `.log-search` class on the input.

- [ ] **Step 1: Write the failing tests**

Create `src/lib/components/LogToolbar.test.ts`:

```ts
import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";

import LogToolbar from "./LogToolbar.svelte";

function makeProps(overrides: Record<string, unknown> = {}) {
  return {
    query: "",
    autoScroll: true,
    paused: false,
    matchTotal: null,
    activeMatchNumber: 0,
    regexError: null,
    onPrev: vi.fn(),
    onNext: vi.fn(),
    regex: false,
    caseSensitive: false,
    onTogglePause: vi.fn(),
    onClear: vi.fn(),
    ...overrides,
  };
}

describe("LogToolbar", () => {
  it("hides the nav group when matchTotal is null (empty query)", () => {
    const { queryByRole } = render(LogToolbar, { props: makeProps() });
    expect(queryByRole("button", { name: "Next match" })).toBeNull();
    expect(queryByRole("button", { name: "Previous match" })).toBeNull();
  });

  it("renders the counter as active/total", () => {
    const { getByLabelText } = render(LogToolbar, {
      props: makeProps({ matchTotal: 7, activeMatchNumber: 3 }),
    });
    expect(getByLabelText("Match count").textContent?.replace(/\s+/g, "")).toBe("3/7");
  });

  it("invokes onPrev/onNext when the nav buttons are clicked", async () => {
    const onPrev = vi.fn();
    const onNext = vi.fn();
    const { getByRole } = render(LogToolbar, {
      props: makeProps({ matchTotal: 5, activeMatchNumber: 1, onPrev, onNext }),
    });
    await fireEvent.click(getByRole("button", { name: "Next match" }));
    expect(onNext).toHaveBeenCalledOnce();
    await fireEvent.click(getByRole("button", { name: "Previous match" }));
    expect(onPrev).toHaveBeenCalledOnce();
  });

  it("disables the nav buttons when matchTotal is 0", () => {
    const { getByRole } = render(LogToolbar, {
      props: makeProps({ matchTotal: 0, activeMatchNumber: 0 }),
    });
    expect(getByRole("button", { name: "Next match" })).toBeDisabled();
    expect(getByRole("button", { name: "Previous match" })).toBeDisabled();
  });

  it("toggles regex via the .* button and reflects aria-pressed", async () => {
    const { getByRole } = render(LogToolbar, {
      props: makeProps({ matchTotal: 1 }),
    });
    const btn = getByRole("button", { name: "Toggle regex" });
    expect(btn).toHaveAttribute("aria-pressed", "false");
    await fireEvent.click(btn);
    expect(btn).toHaveAttribute("aria-pressed", "true");
  });

  it("toggles case-sensitive via the Aa button and reflects aria-pressed", async () => {
    const { getByRole } = render(LogToolbar, {
      props: makeProps({ matchTotal: 1 }),
    });
    const btn = getByRole("button", { name: "Toggle case sensitive" });
    expect(btn).toHaveAttribute("aria-pressed", "false");
    await fireEvent.click(btn);
    expect(btn).toHaveAttribute("aria-pressed", "true");
  });

  it("disables nav and shows the popover when regexError is set", () => {
    const { getByRole, getByText } = render(LogToolbar, {
      props: makeProps({
        matchTotal: 3,
        activeMatchNumber: 1,
        regexError: "Unterminated group",
      }),
    });
    expect(getByRole("button", { name: "Next match" })).toBeDisabled();
    expect(getByText("Unterminated group")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `deno task test src/lib/components/LogToolbar.test.ts`
Expected: FAIL — toolbar does not yet render nav buttons, mode toggles, or the popover; the current props do not include `matchTotal`/`regex`/etc.

- [ ] **Step 3: Rewrite `LogToolbar.svelte`**

Replace the entire contents of `src/lib/components/LogToolbar.svelte` with:

```svelte
<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";

  type Props = {
    query: string;
    autoScroll: boolean;
    paused: boolean;
    matchTotal: number | null;
    activeMatchNumber: number;
    regexError: string | null;
    onPrev: () => void;
    onNext: () => void;
    regex: boolean;
    caseSensitive: boolean;
    onTogglePause: () => void;
    onClear: () => void;
  };

  let {
    query = $bindable(),
    autoScroll = $bindable(),
    paused = $bindable(),
    matchTotal,
    activeMatchNumber,
    regexError,
    onPrev,
    onNext,
    regex = $bindable(),
    caseSensitive = $bindable(),
    onTogglePause,
    onClear,
  }: Props = $props();

  let searchInput = $state<HTMLInputElement | null>(null);

  const showNav = $derived(matchTotal !== null);
  const navDisabled = $derived(regexError !== null || matchTotal === 0);

  export function focusSearch() {
    searchInput?.focus();
  }
</script>

<div class="relative flex items-center gap-1.5 border-b border-border px-2 pt-[4px] pb-[5px]">
  <div class="relative min-w-0 flex-1">
    <Icon
      name="search"
      size="xs"
      class="pointer-events-none absolute left-1.5 top-1/2 -translate-y-1/2 text-text-subtle"
    />
    <input
      bind:this={searchInput}
      bind:value={query}
      type="text"
      placeholder="Search logs"
      spellcheck="false"
      class="log-search h-7 w-full rounded-md border bg-surface-raised pl-6 pr-7 text-[12px] text-text outline-none transition-colors duration-75 placeholder:text-[11px] placeholder:text-text-subtle focus:border-accent {regexError
        ? 'border-danger'
        : 'border-border'}"
    />
    {#if regexError}
      <span
        class="pointer-events-none absolute right-1.5 top-1/2 -translate-y-1/2 text-danger"
        title="Regex error"
      >
        <Icon name="error" size="xs" />
      </span>
    {/if}
  </div>

  {#if showNav}
    <div
      class="flex shrink-0 items-center gap-0.5"
      role="group"
      aria-label="Match navigation"
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
          {activeMatchNumber}/{matchTotal}
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
  {/if}

  <div
    class="flex shrink-0 items-center gap-0.5"
    role="group"
    aria-label="Search modes"
  >
    <button
      type="button"
      tabindex="-1"
      onclick={() => (regex = !regex)}
      class="grid h-6 min-w-6 px-1 place-items-center rounded-md font-mono text-[11px] transition-colors duration-75 {regex
        ? 'bg-accent/15 text-accent'
        : 'text-text-subtle hover:bg-surface-hover hover:text-text'}"
      aria-pressed={regex}
      aria-label="Toggle regex"
      title="Regex"
    >
      .*
    </button>
    <button
      type="button"
      tabindex="-1"
      onclick={() => (caseSensitive = !caseSensitive)}
      class="grid h-6 min-w-6 px-1 place-items-center rounded-md font-mono text-[11px] transition-colors duration-75 {caseSensitive
        ? 'bg-accent/15 text-accent'
        : 'text-text-subtle hover:bg-surface-hover hover:text-text'}"
      aria-pressed={caseSensitive}
      aria-label="Toggle case sensitive"
      title="Case sensitive"
    >
      Aa
    </button>
  </div>

  <span class="mx-0.5 h-4 w-px shrink-0 bg-border"></span>

  <div
    class="flex shrink-0 items-center gap-0.5"
    role="toolbar"
    tabindex="0"
    aria-label="Log actions"
    onfocus={(e: FocusEvent) => {
      const buttons = Array.from(
        (e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>("button"),
      );
      if (buttons.length === 0) return;
      const active =
        buttons.find((b) => b.getAttribute("aria-pressed") === "true") ?? buttons[0];
      active.focus();
    }}
    onkeydown={(e: KeyboardEvent) => {
      const target = e.currentTarget as HTMLElement | null;
      if (!target) return;
      if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
        e.preventDefault();
        const buttons = Array.from(
          target.querySelectorAll<HTMLElement>("button:not([disabled])"),
        );
        const idx = buttons.indexOf(document.activeElement as HTMLElement);
        const next = e.key === "ArrowRight"
          ? (idx + 1) % buttons.length
          : (idx - 1 + buttons.length) % buttons.length;
        buttons[next]?.focus();
      }
    }}
  >
    <button
      type="button"
      tabindex="-1"
      onclick={() => (autoScroll = !autoScroll)}
      class="grid h-6 w-6 place-items-center rounded-md transition-colors duration-75 {autoScroll
        ? 'bg-accent/15 text-accent'
        : 'text-text-subtle hover:bg-surface-hover hover:text-text'}"
      aria-pressed={autoScroll}
      aria-label={autoScroll ? "Auto-scroll on" : "Auto-scroll off"}
      title={autoScroll ? "Auto-scroll: on" : "Auto-scroll: off"}
    >
      <Icon name="scroll-down" size="xs" />
    </button>

    <button
      type="button"
      tabindex="-1"
      onclick={onTogglePause}
      class="grid h-6 w-6 place-items-center rounded-md transition-colors duration-75 {paused
        ? 'bg-accent/15 text-accent'
        : 'text-text-subtle hover:bg-surface-hover hover:text-text'}"
      aria-pressed={paused}
      aria-label={paused ? "Resume live log view" : "Pause live log view"}
      title={paused ? "Resume" : "Pause"}
    >
      {#if paused}
        <Icon name="play" size="xs" />
      {:else}
        <Icon name="pause" size="xs" />
      {/if}
    </button>

    <button
      type="button"
      tabindex="-1"
      onclick={onClear}
      class="grid h-6 w-6 place-items-center rounded-md text-text-subtle transition-colors duration-75 hover:bg-surface-hover hover:text-danger"
      aria-label="Clear logs"
      title="Clear logs"
    >
      <Icon name="clear" size="xs" />
    </button>
  </div>

  {#if regexError}
    <div
      class="absolute left-2 top-full z-10 mt-1 flex max-w-[280px] items-center gap-1.5 rounded-md border border-danger bg-surface-raised px-2.5 py-1.5 text-[11px] text-text font-mono shadow-md"
      role="status"
      aria-live="polite"
    >
      <span class="text-danger"><Icon name="error" size="xs" /></span>
      <span>{regexError}</span>
    </div>
  {/if}
</div>
```

- [ ] **Step 4: Run the toolbar tests to verify they pass**

Run: `deno task test src/lib/components/LogToolbar.test.ts`
Expected: PASS.

- [ ] **Step 5: Typecheck**

Run: `deno task check`
Expected: errors ONLY in `LogViewer.svelte` (it still passes the old `matchCount` prop and the old `highlightLine` signature). These are fixed in Task 3. Do not fix them here.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/LogToolbar.svelte src/lib/components/LogToolbar.test.ts
git commit -m "feat(logs): add search mode toggles & match nav to toolbar"
```

---

## Task 3: Wire up `LogViewer.svelte` (state, navigation, keyboard, highlight)

**Files:**
- Modify: `src/lib/components/LogViewer.svelte` (script + row template + toolbar usage)
- Test: `src/lib/components/LogViewer.test.ts` (new — no existing viewer tests)

**Interfaces:**
- Consumes: from Task 1 — `buildMatcher`, `lineMatches`, `highlightLine`, types `SearchOptions`, `Matcher`. From Task 2 — `LogToolbar` props contract.
- Produces: a `LogViewer` that exposes the same external `Props` as before (`logs`, `processName`, `truncatedCount`, `onClear`, `onActions`) — no change for callers.

**Key implementation details to honor:**
- `ROW_HEIGHT` stays `22`.
- Searchable text per line is `${entry.stream} ${entry.line}` (preserves current behavior where searching "stderr" finds error lines).
- `matchTotal`: `matcher === null` → `null`; `matcher` is `{ regex }` → `filteredLogs.length`; `matcher` is `{ error }` → `0`.
- Navigation indexes directly into `filteredLogs` (every line in it is a match when a valid matcher is active).
- Reset `activeMatchIndex` to 0 on query/mode change via an `$effect` whose only reads are `query`, `searchOptions.regex`, `searchOptions.caseSensitive` (so writing `activeMatchIndex` inside it does not retrigger it).
- Persist `{ regex, caseSensitive }` to `localStorage` key `diavola.logSearch`; load on init.
- Active row highlight: the row at `startIndex + index === activeMatchIndex` gets `bg-surface-hover/60` and its `<mark>` uses `bg-warning/60`; other matches keep `bg-warning/30`.

- [ ] **Step 1: Write the failing tests**

Create `src/lib/components/LogViewer.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";

import LogViewer from "./LogViewer.svelte";
import type { ProcessLogPayload } from "$lib/types";

// jsdom has no layout; the auto-scroll effect calls Element.scrollTo which can
// throw "Not implemented". Stub it to a no-op for these tests.
beforeEach(() => {
  if (typeof Element.prototype.scrollTo === "function") {
    vi.spyOn(Element.prototype, "scrollTo").mockImplementation(() => {});
  }
});

function makeLine(text: string, i: number): ProcessLogPayload {
  return {
    sessionId: "s1",
    runtimeId: "r1",
    processName: "api",
    stream: "stdout",
    line: text,
    timestamp: `2026-01-01T00:00:${i.toString().padStart(2, "0")}Z`,
  };
}

const logs = (lines: string[]): ProcessLogPayload[] => lines.map(makeLine);

function makeProps(overrides: Record<string, unknown> = {}) {
  return {
    logs: [] as ProcessLogPayload[],
    processName: "api",
    truncatedCount: 0,
    onClear: vi.fn(),
    ...overrides,
  };
}

describe("LogViewer search", () => {
  it("filters lines by a substring query (case-insensitive)", async () => {
    const { container, queryByText } = render(LogViewer, {
      props: makeProps({
        logs: logs(["listening on 3000", "worker ready", "listening on 3001"]),
      }),
    });
    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    await fireEvent.input(input, { target: { value: "LISTENING" } });
    // non-matching line is gone
    expect(queryByText(/worker ready/)).toBeNull();
  });

  it("treats the query as regex when regex mode is on", async () => {
    const { container } = render(LogViewer, {
      props: makeProps({ logs: logs(["error 42", "warn 7", "error 99"]) }),
    });
    await fireEvent.click(container.querySelector('[aria-label="Toggle regex"]')!);
    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    await fireEvent.input(input, { target: { value: "error \\d+" } });
    expect(container.textContent).toMatch(/error 42/);
    expect(container.textContent).not.toMatch(/warn 7/);
  });

  it("shows the error popover and disables nav on an invalid regex", async () => {
    const { container, getByText } = render(LogViewer, {
      props: makeProps({ logs: logs(["api listening"]) }),
    });
    await fireEvent.click(container.querySelector('[aria-label="Toggle regex"]')!);
    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    await fireEvent.input(input, { target: { value: "(unclosed" } });
    expect(getByText(/Unterminated|Invalid|regular expression/i)).toBeInTheDocument();
    expect(container.querySelector('[aria-label="Next match"]')).toBeDisabled();
  });

  it("navigates matches with Enter / Shift+Enter and wraps around", async () => {
    const { container } = render(LogViewer, {
      props: makeProps({ logs: logs(["a one", "b one", "c one"]) }),
    });
    const counter = () =>
      container
        .querySelector('[aria-label="Match count"]')
        ?.textContent?.replace(/\s+/g, "");

    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    await fireEvent.input(input, { target: { value: "one" } });
    expect(counter()).toBe("1/3");

    await fireEvent.keyDown(input, { key: "Enter" });
    expect(counter()).toBe("2/3");

    await fireEvent.keyDown(input, { key: "Enter" });
    expect(counter()).toBe("3/3");

    await fireEvent.keyDown(input, { key: "Enter" }); // wrap to first
    expect(counter()).toBe("1/3");

    await fireEvent.keyDown(input, { key: "Enter", shiftKey: true }); // prev wraps to last
    expect(counter()).toBe("3/3");
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `deno task test src/lib/components/LogViewer.test.ts`
Expected: FAIL — current `LogViewer` does not accept regex mode, has no nav counter, and `Enter` does nothing.

- [ ] **Step 3: Update `LogViewer.svelte` imports**

In `src/lib/components/LogViewer.svelte`, replace the import line:

```ts
  import { escapeRegExp, highlightLine, countMatches, type TextSegment } from "$lib/utils/searchHighlight";
```

with:

```ts
  import {
    buildMatcher,
    highlightLine,
    lineMatches,
    type Matcher,
    type SearchOptions,
  } from "$lib/utils/searchHighlight";
```

- [ ] **Step 4: Add search-options state + persistence**

Directly below the existing state declarations (after `let scrollTop = $state(0);` and `let viewportHeight = $state(0);`), add:

```ts
  const SEARCH_PREFS_KEY = "diavola.logSearch";

  function loadSearchOptions(): SearchOptions {
    try {
      const raw = localStorage.getItem(SEARCH_PREFS_KEY);
      if (raw) {
        const parsed = JSON.parse(raw) as Partial<SearchOptions>;
        return { regex: !!parsed.regex, caseSensitive: !!parsed.caseSensitive };
      }
    } catch {
      // ignore corrupt/unavailable storage
    }
    return { regex: false, caseSensitive: false };
  }

  let searchOptions = $state<SearchOptions>(loadSearchOptions());

  $effect(() => {
    try {
      localStorage.setItem(
        SEARCH_PREFS_KEY,
        JSON.stringify({ regex: searchOptions.regex, caseSensitive: searchOptions.caseSensitive }),
      );
    } catch {
      // ignore storage errors
    }
  });
```

- [ ] **Step 5: Replace `filteredLogs` with a matcher-driven derivation**

Replace the existing `filteredLogs` derivation (the `$derived.by(...)` that filters by lowercase `.includes`) with:

```ts
  const matcher = $derived(buildMatcher(query, searchOptions));
  const regexError = $derived(matcher && "error" in matcher ? matcher.error : null);

  let filteredLogs = $derived.by(() => {
    const base = paused ? (pausedLogs ?? logs) : logs;
    if (matcher === null || "error" in matcher) return base;
    return base.filter((entry) => lineMatches(matcher, `${entry.stream} ${entry.line}`));
  });
```

- [ ] **Step 6: Add navigation state and logic**

Below the `filteredLogs` derivation, add:

```ts
  let activeMatchIndex = $state(0);

  // Reset to the first match whenever the query or search modes change.
  // (This effect only reads query + modes, so writing activeMatchIndex here
  // does not retrigger it.)
  $effect(() => {
    void `${query}|${searchOptions.regex}|${searchOptions.caseSensitive}`;
    activeMatchIndex = 0;
  });

  // Clamp when the filtered list shrinks below the active index.
  $effect(() => {
    const max = Math.max(0, filteredLogs.length - 1);
    if (activeMatchIndex > max) activeMatchIndex = max;
  });

  const matcherActive = $derived(matcher !== null && "regex" in matcher);
  const matchTotal = $derived(
    matcher === null ? null : matcherActive ? filteredLogs.length : 0,
  );
  const activeMatchNumber = $derived(activeMatchIndex + 1);

  function scrollToActiveMatch() {
    if (!viewport) return;
    const top = activeMatchIndex * ROW_HEIGHT - (viewportHeight - ROW_HEIGHT) / 2;
    const maxScroll = totalHeight - viewportHeight;
    viewport.scrollTo({ top: Math.max(0, Math.min(top, maxScroll)) });
  }

  function goToMatch(next: number) {
    const len = filteredLogs.length;
    if (len === 0) return;
    activeMatchIndex = (next % len + len) % len; // wrap-around
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

- [ ] **Step 7: Extend `handleKeydown` for Enter / Shift+Enter**

In the existing `handleKeydown` function, after the `Escape` branch and before the closing brace, add:

```ts
    if (typing && event.key === "Enter" && matcherActive && filteredLogs.length > 0) {
      event.preventDefault();
      if (event.shiftKey) prevMatch();
      else nextMatch();
    }
```

- [ ] **Step 8: Remove the old `matchCount` derivation**

Delete the now-unused `matchCount` const:

```ts
  const matchCount = $derived(
    query.trim().length === 0
      ? null
      : countMatches(
          filteredLogs.map((log) => `${log.stream} ${log.line}`),
          query,
        ),
  );
```

- [ ] **Step 9: Update the `<LogToolbar>` usage**

Replace the existing `<LogToolbar ... />` block (which binds `query`, `autoScroll`, `paused`, `matchCount`, `onTogglePause`, `onClear`) with:

```svelte
  <LogToolbar
    bind:query
    bind:autoScroll
    bind:paused
    bind:regex={searchOptions.regex}
    bind:caseSensitive={searchOptions.caseSensitive}
    {matchTotal}
    {activeMatchNumber}
    {regexError}
    onPrev={prevMatch}
    onNext={nextMatch}
    onTogglePause={togglePaused}
    onClear={clearLogs}
  />
```

- [ ] **Step 10: Update the row template — active highlight + matcher-based highlight**

In the `{#each visibleItems ...}` row, compute whether this row is the active match and pass the `Matcher` to `highlightLine`. Replace:

```svelte
            <span
              class={`whitespace-nowrap ${toneByStream[entry.stream] ?? "text-text"}`}
            >
              {#if entry.stream === "system" && /ready|listening/i.test(entry.line)}
                <span class="mr-1">&#9679;</span>
              {/if}
              {#each highlightLine(entry.line, query) as seg}
                {#if seg.match}
                  <mark class="bg-warning/30 text-text rounded-[2px]"
                    >{seg.text}</mark
                  >
                {:else}
                  {seg.text}
                {/if}
              {/each}
            </span>
```

with:

```svelte
            <span
              class={`whitespace-nowrap ${toneByStream[entry.stream] ?? "text-text"}`}
            >
              {#if entry.stream === "system" && /ready|listening/i.test(entry.line)}
                <span class="mr-1">&#9679;</span>
              {/if}
              {#each highlightLine(entry.line, matcher) as seg}
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
```

Then, on the row's outer `<div>` (the one with `border-l-[3px] ...`), add the active-row background. Change its class list to include:

```svelte
            class="group flex items-center gap-3 px-3 hover:bg-surface-hover/40 border-l-[3px] {borderByStream[
              entry.stream
            ] ?? 'border-l-transparent'} {borderCornerClass(
              startIndex + index,
            )} {startIndex + index === activeMatchIndex && matcherActive ? 'bg-surface-hover/60' : ''}"
```

(Only the trailing ` {startIndex + index === activeMatchIndex && matcherActive ? 'bg-surface-hover/60' : ''}` is new; keep the existing classes intact.)

- [ ] **Step 11: Run the viewer tests to verify they pass**

Run: `deno task test src/lib/components/LogViewer.test.ts`
Expected: PASS.

- [ ] **Step 12: Run the full suite + typecheck**

Run: `deno task test && deno task check`
Expected: all tests PASS, no typecheck errors.

- [ ] **Step 13: Commit**

```bash
git add src/lib/components/LogViewer.svelte src/lib/components/LogViewer.test.ts
git commit -m "feat(logs): regex/case search + prev/next match navigation"
```

---

## Verification (end-of-plan)

After all three tasks:

- `deno task test` — all green.
- `deno task check` — clean.
- Manual: run `deno task app` with `diavola.yml`, start the project, open the log viewer, press `/`, type a query, toggle `.*` and `Aa`, use `Enter`/`Shift+Enter` and the `‹`/`›` buttons, confirm wrap-around and the active-match highlight. Enter an invalid regex and confirm the floating popover + disabled nav with no log-list shift.
