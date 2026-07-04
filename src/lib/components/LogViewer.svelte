<script lang="ts">
  import { MAX_LOG_LINES_PER_PROCESS } from "$lib/stores/runtime.svelte";
  import type { ProcessLogPayload } from "$lib/types";
  import { isTypingTarget } from "$lib/utils/dom";
  import { computeVirtualScroll, isAtBottom } from "$lib/utils/virtualScroll";
  import { escapeRegExp, highlightLine, countMatches, type TextSegment } from "$lib/utils/searchHighlight";
  import Icon from "$lib/components/ui/Icon.svelte";
  import LogToolbar from "$lib/components/LogToolbar.svelte";

  type Props = {
    logs: ProcessLogPayload[];
    processName: string | null;
    truncatedCount: number;
    onClear: () => void;
    onActions?: (actions: { copy: () => void; clear: () => void }) => void;
  };

  let { logs, processName, truncatedCount, onClear, onActions }: Props =
    $props();

  const ROW_HEIGHT = 22;

  let query = $state("");
  let autoScroll = $state(true);
  let paused = $state(false);
  let pausedLogs = $state<ProcessLogPayload[] | null>(null);
  let viewport = $state<HTMLDivElement | null>(null);
  let activeProcessName = $state<string | null>(null);
  let copied = $state(false);
  let copyTimer = $state<number | null>(null);
  let scrollTop = $state(0);
  let viewportHeight = $state(0);

  const visibleLogs = $derived(paused ? (pausedLogs ?? logs) : logs);
  const totalVisibleCount = $derived(visibleLogs.length);

  let filteredLogs = $derived.by(() =>
    visibleLogs.filter((entry) =>
      query.trim().length === 0
        ? true
        : `${entry.stream} ${entry.line}`
            .toLowerCase()
            .includes(query.toLowerCase()),
    ),
  );

  const virtualScroll = $derived(
    computeVirtualScroll(scrollTop, viewportHeight, filteredLogs.length),
  );
  const totalHeight = $derived(virtualScroll.totalHeight);
  const startIndex = $derived(virtualScroll.startIndex);
  const endIndex = $derived(virtualScroll.endIndex);

  const visibleItems = $derived(filteredLogs.slice(startIndex, endIndex));

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
    const text = filteredLogs
      .map(
        (entry) =>
          `${new Date(entry.timestamp).toLocaleTimeString()} ${entry.stream} ${entry.line}`,
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
  });

  $effect(() => {
    filteredLogs.length;
    autoScroll;
    paused;
    if (autoScroll && !paused && viewport) {
      requestAnimationFrame(() => {
        viewport?.scrollTo({ top: viewport.scrollHeight });
      });
    }
  });

  $effect(() => {
    return () => {
      if (copyTimer !== null) {
        clearTimeout(copyTimer);
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

  const borderCornerClass = $derived((indexInLogs: number): string => {
    const entry = filteredLogs[indexInLogs];
    if (!entry) return "";
    const prev = indexInLogs > 0 ? filteredLogs[indexInLogs - 1] : null;
    const next =
      indexInLogs < filteredLogs.length - 1
        ? filteredLogs[indexInLogs + 1]
        : null;
    const sameAsPrev = prev !== null && prev.stream === entry.stream;
    const sameAsNext = next !== null && next.stream === entry.stream;

    if (!sameAsPrev && !sameAsNext) return "rounded-tl rounded-bl";
    if (!sameAsPrev) return "rounded-tl";
    if (!sameAsNext) return "rounded-bl";
    return "";
  });

  const matchCount = $derived(
    query.trim().length === 0
      ? null
      : countMatches(
          filteredLogs.map((log) => `${log.stream} ${log.line}`),
          query,
        ),
  );
</script>

<svelte:window onkeydown={handleKeydown} />

<section class="flex h-full min-h-0 flex-col bg-surface">
  <LogToolbar
    bind:query
    bind:autoScroll
    bind:paused
    {matchCount}
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
    {#if filteredLogs.length === 0}
      <div class="px-3 py-2 text-text-subtle">
        {query ? "No matching lines" : "No log line"}
      </div>
    {:else}
      <div style="height: {totalHeight}px; position: relative;">
        {#each visibleItems as entry, index (`${entry.timestamp}-${startIndex + index}`)}
          <div
            style="position: absolute; top: {(startIndex + index) *
              ROW_HEIGHT}px; left: 0; right: 0; height: {ROW_HEIGHT}px;"
            class="group flex items-center gap-3 px-3 hover:bg-surface-hover/40 border-l-[3px] {borderByStream[
              entry.stream
            ] ?? 'border-l-transparent'} {borderCornerClass(
              startIndex + index,
            )}"
          >
            <span class="shrink-0 select-none text-text-subtle">
              {new Date(entry.timestamp).toLocaleTimeString()}
            </span>
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
          </div>
        {/each}
      </div>
    {/if}
  </div>
</section>
