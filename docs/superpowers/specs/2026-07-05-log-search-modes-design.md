# Log Search Modes & Match Navigation

**Date:** 2026-07-05
**Status:** Approved (pending implementation plan)
**Scope:** Enhancement to the log viewer's search capabilities

## Problem

`LogViewer.svelte` already supports plain-text, case-insensitive substring
search (with highlight, a match counter, and the `/` / `Escape` shortcuts).
Three gaps remain:

1. **No regex search** — matching is hardcoded to an escaped substring
   (`searchHighlight.ts` builds `escapeRegExp(trimmed)` with `"gi"`).
2. **No case-sensitive option** — matching is always case-insensitive.
3. **No match navigation** — the toolbar shows a match count but there is no
   way to jump between occurrences; the user must scroll manually.

Stream filtering (stdout/stderr/system) was deliberately excluded: modern
structured loggers (pino, etc.) write everything to `stdout` as JSON, so
isolating by stream is rarely useful.

## Goals

- Toggle regex mode in the search bar.
- Toggle case-sensitive mode in the search bar.
- Navigate between matches with prev/next buttons and keyboard shortcuts, with
  the current match highlighted distinctly and auto-scrolled into view.

## Non-Goals

- Per-occurrence navigation (we navigate by matching line — see "Navigation
  model" below).
- Stream filtering.
- Search-history or saved searches.
- Persisting the query text across sessions.

## Design Decisions

### Navigation model: per line, not per occurrence

The log list is virtualized with one row per log line (`ROW_HEIGHT = 22`).
Navigation jumps **between matching lines**, not individual occurrences within
a line. Rationale:

- Consistent with the virtualized rendering (1 row = 1 line).
- Simpler scroll math: target line index maps directly to a pixel offset.
- Matches how users scan logs (line by line).

Consequence: the match counter displays `{active} / {matchingLineCount}`
(number of matching lines), replacing the current occurrence-based
`countMatches`. This is consistent with the navigation unit.

### Defaults preserve existing behavior

- Default search mode: **substring, case-insensitive** (unchanged).
- The `/` shortcut focuses the search field; `Escape` clears the query — both
  unchanged.

### Persistence

Search **modes** (`regex`, `caseSensitive`) are a user preference and are
persisted to `localStorage` under key `diavola.logSearch` as a JSON object.
They survive process switches and app restarts. The **query text** is not
persisted (unchanged: cleared manually via `Escape` or the clear button).

On process switch, modes persist; `activeMatchIndex` resets to `0`.

## Architecture

### Logic layer — `src/lib/utils/searchHighlight.ts`

Introduce a shared matcher builder so `highlightLine`, `countMatches`, and the
new line-matching logic all derive from one source of truth.

```ts
export type SearchOptions = { regex: boolean; caseSensitive: boolean };

export type Matcher =
  | null                       // query empty — no filter
  | { regex: RegExp }          // valid matcher
  | { error: string };         // invalid regex — surfaced in UI

export function buildMatcher(query: string, options: SearchOptions): Matcher {
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
```

- `highlightLine(text, query, options)` — uses `buildMatcher`; returns
  `[{ text, match: false }]` on error/empty (no highlight).
- `countMatchingLines(lines, query, options)` — **new**, replaces
  occurrence counting for the counter; returns the number of lines containing
  at least one match.
- `countMatches` — **removed** (only referenced in `LogViewer.svelte`, which
  switches to `countMatchingLines`).
- `escapeRegExp` — unchanged.

### View layer — `src/lib/components/LogViewer.svelte`

New state and derived values:

- `searchOptions = $state<SearchOptions>({ regex: false, caseSensitive: false })`
  (bound to the toolbar; restored from `localStorage` on mount, saved on
  change).
- `matcher = $derived(buildMatcher(query, searchOptions))`.
- `regexError = $derived` — the error message when `matcher` is `{ error }`,
  else `null`.
- `filteredLogs` — derived; keeps a line when `matcher` is `null` or when the
  matcher's regex tests against `${entry.stream} ${entry.line}`.
- `matchingLineIndices = $derived<number[]>` — indices into `filteredLogs`
  whose line matches (same test used for filtering).
- `activeMatchIndex = $state(0)` — position within
  `matchingLineIndices`. Reset to `0` whenever `query` or `searchOptions`
  change (via `$effect`); clamped to `matchingLineIndices.length - 1` when it
  shrinks.

Counter values passed to the toolbar:
- `matchTotal = matchingLineIndices.length` when query is non-empty, else
  `null`.
- `activeMatchNumber = activeMatchIndex + 1` (1-based).

Navigation actions:

```ts
function goToMatch(next: number) {
  if (matchingLineIndices.length === 0) return;
  const len = matchingLineIndices.length;
  activeMatchIndex = (next % len + len) % len; // wrap-around
  autoScroll = false;
  scrollToActiveMatch();
}

function nextMatch() { goToMatch(activeMatchIndex + 1); }
function prevMatch() { goToMatch(activeMatchIndex - 1); }
```

Scroll-to-active reuses the existing virtualized layout:

```ts
function scrollToActiveMatch() {
  const target = matchingLineIndices[activeMatchIndex];
  if (target == null || !viewport) return;
  const top = target * ROW_HEIGHT - (viewportHeight - ROW_HEIGHT) / 2;
  const maxScroll = totalHeight - viewportHeight;
  viewport.scrollTo({ top: Math.max(0, Math.min(top, maxScroll)) });
}
```

Keyboard handling is extended in the existing `handleKeydown`: when the search
field is focused and the matcher is valid, `Enter` → `nextMatch()`,
`Shift+Enter` → `prevMatch()`.

### Active match highlight

Two visual tiers, both in `LogViewer.svelte`'s row template:

- Non-active matching lines: `<mark>` at `bg-warning/30` (existing).
- Active matching line: `<mark>` at `bg-warning/60`, plus the row gets
  `bg-surface-hover/60` for lateral visibility (in addition to the existing
  stream left-border).

The active row is identified by
`startIndex + index === matchingLineIndices[activeMatchIndex]`.

### Toolbar — `src/lib/components/LogToolbar.svelte`

Layout changes from

`[ input ] [ actions: scroll · pause · clear ]`

to

`[ input ] [ ‹  n/total  › ] [ .* · Aa ] [ actions: scroll · pause · clear ]`

The current absolute-positioned match-count badge is removed; the counter
moves into the navigation group (a flex sibling of the input), which also
fixes the existing text-overlap fragility.

New props (additions to existing `Props`):

```ts
matchTotal: number | null;       // null = query empty (hide nav)
activeMatchNumber: number;       // 1-based
regexError: string | null;
onPrev: () => void;
onNext: () => void;
regex: boolean;                  // $bindable
caseSensitive: boolean;          // $bindable
```

Navigation group:

- `‹` / `›` buttons (`onPrev` / `onNext`) + centered counter
  `${activeMatchNumber}/${matchTotal}`.
- Rendered only when `matchTotal !== null`.
- Both buttons `disabled` when `matchTotal === 0`; counter shows `0/0`.

Mode group:

- `.*` toggles `regex`; `Aa` toggles `caseSensitive`. Both use the existing
  button pattern (`grid h-6 w-6`, `aria-pressed`, `accent/15 text-accent` when
  active).

Error state (no layout shift — critical requirement):

The error must not push the log list down, ever. Three layered signals, none of
which add height to the layout:

- Search input border switches to `border-danger`.
- A persistent `⚠` icon at the right inside the input footprint.
- The counter slot (where `n/total` lives) shows a `⚠` instead of `0/0`.
- A **floating popover** anchored under the input (`position: absolute`,
  `z-index` above logs, with a small pointer arrow) shows the full message.
  It appears while the field is focused and the error is active; it hides on
  blur (the border + icons remain) and disappears entirely once the regex is
  valid again.

Navigation is disabled and treated as zero matches while `regexError` is set.

Keyboard (search field focused): `Enter` → next, `Shift+Enter` → prev.
Existing `/` (focus) and `Escape` (clear query) unchanged.

## Data Flow

```
LogToolbar (binds query, searchOptions; emits onPrev/onNext)
    │
    ▼
LogViewer
  query + searchOptions
    └─► buildMatcher ──► matcher
                            ├─► filteredLogs (filter)
                            ├─► matchingLineIndices (which lines match)
                            └─► highlightLine per row (render)
  activeMatchIndex ─► activeMatchNumber (counter) + scrollToActiveMatch
```

## Error Handling

- **Invalid regex**: `buildMatcher` returns `{ error }`. The UI disables
  navigation, marks the input red, and surfaces the message via a floating
  popover + persistent icons (see "Error state" above). Critically, the error
  feedback causes **no vertical layout shift** — the popover overlays the logs
  instead of inserting a row. No exception propagates; the app stays usable
  (substring mode or fixing the pattern restores normal behavior).
- **No matches**: navigation buttons disabled, counter `0/0`, active highlight
  absent. Existing "No matching lines" empty-state message remains.
- **Match index out of range** (lines removed by truncation/filter change):
  clamped in the `$effect` that watches `matchingLineIndices`.

## Testing

Conventions: `vitest` + `@testing-library/svelte` (see existing
`*.test.ts` in `src/lib/components/`).

### `src/lib/utils/searchHighlight.test.ts` (new)

- `buildMatcher`: empty query → `null`; substring → valid regex with escaped
  meta-chars; regex mode → raw source; case-sensitive toggles `i` flag; invalid
  regex → `{ error }`.
- `highlightLine`: substring + case variations; regex mode; error/empty → no
  highlight segments.
- `countMatchingLines`: counts lines (not occurrences); respects modes.

### `src/lib/components/LogToolbar.test.ts` (extend existing)

- `.*` toggle flips `regex` and reflects `aria-pressed`.
- `Aa` toggle flips `caseSensitive` and reflects `aria-pressed`.
- Nav `‹`/`›` invoke `onPrev`/`onNext`.
- Counter renders `${active}/${total}`; hidden when `matchTotal === null`.
- Buttons disabled when `matchTotal === 0`.
- `regexError` set → input has danger border; nav disabled.

### `LogViewer` behavior (via existing/new component tests)

- Regex query filters correctly; invalid regex shows error, no crash.
- `Enter` on focused search → `nextMatch`; `Shift+Enter` → `prevMatch`.
- Wrap-around: next after last → first; prev before first → last.
- Active highlight class applied to the expected row.
- Manual jump disables `autoScroll`.

## Open Questions

None at design time. (Defaults for wrap-around, active-highlight intensity,
persistence key, and the no-shift error presentation were chosen during
brainstorming and are recorded above.)
