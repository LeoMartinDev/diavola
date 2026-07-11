<script lang="ts">
  import { MAX_LOG_LINES_PER_PROCESS } from "$lib/stores/runtime.svelte";
  import type { FlatRow } from "$lib/types";
  import { isTypingTarget } from "$lib/utils/dom";
  import { computeVirtualScroll, isAtBottom } from "$lib/utils/virtualScroll";
  import {
    buildMatcher,
    type SearchOptions,
  } from "$lib/utils/searchHighlight";
  import { computeMatchIndices } from "$lib/utils/logSearch";
  import { debounceWithMaxWait } from "$lib/utils/scheduler";
  import { parseAnsi, stripAnsi, styleToCss } from "$lib/utils/ansi";
  import Icon from "$lib/components/ui/Icon.svelte";
  import LogToolbar from "$lib/components/LogToolbar.svelte";

  type Props = {
    logs: FlatRow[];
    processName: string | null;
    runtimeId?: string | null;
    truncatedCount: number;
    onClear: () => void;
    onActions?: (actions: { copy: () => void; clear: () => void }) => void;
  };

  let { logs, processName, runtimeId = null, truncatedCount, onClear, onActions }: Props = $props();

  const ROW_HEIGHT = 22;

  let query = $state("");
  let autoScroll = $state(true);
  let paused = $state(false);
  let pausedLogs = $state<FlatRow[] | null>(null);
  let viewport = $state<HTMLDivElement | null>(null);
  let activeProcessName = $state<string | null>(null);
  let copied = $state(false);
  let copyTimer = $state<number | null>(null);
  let scrollTop = $state(0);
  let viewportHeight = $state(0);

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

  const visibleLogs = $derived(paused ? (pausedLogs ?? logs) : logs);
  const totalVisibleCount = $derived(visibleLogs.length);

  const matcher = $derived(buildMatcher(query, searchOptions));
  const regexError = $derived(matcher && "error" in matcher ? matcher.error : null);

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
    if (generation !== searchGeneration) return;
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

  function togglePaused() {
    paused = !paused;
    pausedLogs = paused ? [...logs] : null;
  }

  function clearLogs() {
    onClear();
    if (paused) {
      pausedLogs = [];
    }
  }

  $effect(() => {
    onActions?.({ copy: copyLogs, clear: clearLogs });
  });

  async function copyLogs() {
    const text = visibleLogs
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

  let copiedEntryId = $state<number | null>(null);
  let entryCopyTimer = $state<number | null>(null);

  async function copyEntry(entryId: number) {
    const entryLines = visibleLogs
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

  function handleKeydown(event: KeyboardEvent) {
    const typing = isTypingTarget(event.target as HTMLElement | null);

    if (!typing && event.key === "/") {
      event.preventDefault();
      (document.querySelector(".log-search") as HTMLInputElement)?.focus();
      return;
    }
    if (typing && event.key === "Escape") {
      if (query.length > 0) {
        query = "";
      } else {
        (event.target as HTMLElement)?.blur();
      }
    }
    if (typing && event.key === "Enter" && matcherActive && matchRowIndices.length > 0) {
      event.preventDefault();
      if (event.shiftKey) prevMatch();
      else nextMatch();
    }
  }

  function handleScroll() {
    if (!viewport) return;
    scrollTop = viewport.scrollTop;
    viewportHeight = viewport.clientHeight;
    autoScroll = isAtBottom(scrollTop, viewportHeight, viewport.scrollHeight);
  }

  $effect(() => {
    const el = viewport;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      if (viewport) viewportHeight = viewport.clientHeight;
    });
    ro.observe(el);
    viewportHeight = el.clientHeight;
    return () => ro.disconnect();
  });

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

  $effect(() => {
    visibleLogs.length;
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

  const toneByStream: Record<string, string> = {
    stdout: "text-text",
    stderr: "text-danger",
    system: "text-accent",
  };

  const borderByStream: Record<string, string> = {
    stdout: "border-l-border-strong",
    stderr: "border-l-danger",
    system: "border-l-accent",
  };

  function isObjectContinuation(row: FlatRow): boolean {
    const raw = stripAnsi(row.text);
    const text = raw.trimStart();
    return (
      raw !== text ||
      text.startsWith('"') ||
      text === "{" ||
      text === "[" ||
      text === "}" ||
      text === "}," ||
      text === "]" ||
      text === "],"
    );
  }

  function isStructuralOpener(text: string): boolean {
    return text.endsWith("{") || text.endsWith("[");
  }

  function isStructuralCloser(text: string): boolean {
    return text === "}" || text === "}," || text === "]" || text === "],";
  }

  function visualGroupIdForIndex(indexInLogs: number): number | string {
    const row = visibleLogs[indexInLogs];
    if (!row) return `missing-${indexInLogs}`;
    if (row.isContinuation) return row.entryId;
    if (!isObjectContinuation(row)) return row.entryId;

    const rowText = stripAnsi(row.text).trim();
    const rowIsOpener = isStructuralOpener(rowText);
    let unmatchedClosers = isStructuralCloser(rowText) ? 1 : 0;

    for (let i = indexInLogs - 1; i >= 0; i -= 1) {
      const previous = visibleLogs[i];
      if (!previous) break;
      if (previous.stream !== row.stream) break;

      const previousText = stripAnsi(previous.text).trim();
      if (isStructuralCloser(previousText)) {
        unmatchedClosers += 1;
        continue;
      }
      if (isStructuralOpener(previousText)) {
        if (unmatchedClosers === 0) {
          return visualGroupIdForIndex(i);
        }
        unmatchedClosers -= 1;
        if (!rowIsOpener && unmatchedClosers === 0) {
          return visualGroupIdForIndex(i);
        }
        continue;
      }
      if (!isObjectContinuation(previous)) return previous.entryId;
    }

    return row.entryId;
  }

  const borderCornerClass = $derived((indexInLogs: number): string => {
    const row = visibleLogs[indexInLogs];
    if (!row) return "";
    const currentGroup = visualGroupIdForIndex(indexInLogs);
    const prev = indexInLogs > 0 ? visibleLogs[indexInLogs - 1] : null;
    const next =
      indexInLogs < visibleLogs.length - 1
        ? visibleLogs[indexInLogs + 1]
        : null;
    const sameAsPrev = prev !== null && visualGroupIdForIndex(indexInLogs - 1) === currentGroup;
    const sameAsNext = next !== null && visualGroupIdForIndex(indexInLogs + 1) === currentGroup;

    if (!sameAsPrev && !sameAsNext) return "rounded-tl rounded-bl";
    if (!sameAsPrev) return "rounded-tl";
    if (!sameAsNext) return "rounded-bl";
    return "";
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<section class="flex h-full min-h-0 flex-col bg-surface">
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

  {#if truncatedCount > 0}
    <div class="border-b border-border px-3 py-1 text-[11px] text-text-subtle">
      {truncatedCount} older line{truncatedCount === 1 ? "" : "s"} hidden · latest
      {Math.min(totalVisibleCount, MAX_LOG_LINES_PER_PROCESS)} shown
    </div>
  {/if}

  <div
    bind:this={viewport}
    onscroll={handleScroll}
    data-native-selectable="logs"
    class="min-h-0 flex-1 overflow-auto font-mono text-[12px] leading-[1.45]"
  >
    {#if visibleLogs.length === 0}
        <div class="px-3 py-2 text-text-subtle">No log line</div>
    {:else}
      <div style="height: {totalHeight}px; position: relative;">
        {#each visibleItems as row, index (`${row.entryId}-${row.lineIndex}-${startIndex + index}`)}
          <div
            style="position: absolute; top: {(startIndex + index) *
              ROW_HEIGHT}px; left: 0; right: 0; height: {ROW_HEIGHT}px;"
            data-log-row="true"
            class="group flex items-center gap-3 px-3 border-l-[3px] {borderByStream[
              row.stream
            ] ?? 'border-l-transparent'} {borderCornerClass(
              startIndex + index,
            )} {row.isContinuation ? 'bg-surface-muted/40' : ''} {matchRowIndices.includes(startIndex + index) && matcherActive ? 'bg-warning/15' : ''} {startIndex + index === activeMatchRow && matcherActive ? 'bg-surface-hover/60' : ''}"
          >
            <span class="flex shrink-0 items-center gap-1 whitespace-nowrap text-[10px] text-text-subtle w-[70px] select-none">
              {#if row.isFirstLine}
                <button
                  type="button"
                  onclick={(e: MouseEvent) => { e.stopPropagation(); copyEntry(row.entryId); }}
                  class="shrink-0 grid h-3 w-3 place-items-center rounded text-text-subtle hover:text-text opacity-0 group-hover:opacity-100 transition-opacity"
                  title="Copy entry"
                >
                  <Icon name={copiedEntryId === row.entryId ? "check" : "copy"} size="xs" />
                </button>
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
                <span style={styleToCss(ansiSeg.style) ?? undefined}>{ansiSeg.text}</span>
              {/each}
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</section>
