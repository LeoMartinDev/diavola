<script lang="ts">
  import type { Snippet } from "svelte";
  import { onMount } from "svelte";
  import { FitAddon } from "@xterm/addon-fit";
  import { Terminal } from "xterm";
  import "xterm/css/xterm.css";
  import Icon from "$lib/components/ui/Icon.svelte";
  import { themeStore } from "$lib/stores/theme.svelte";

  function readXtermTheme(): Record<string, string> {
    const style = getComputedStyle(document.documentElement);
    return {
      background: style.getPropertyValue("--color-surface").trim(),
      foreground: style.getPropertyValue("--color-text").trim(),
      cursor: style.getPropertyValue("--color-accent").trim(),
      cursorAccent: style.getPropertyValue("--color-canvas").trim(),
      selectionBackground: style.getPropertyValue("--color-accent-soft").trim(),
    };
  }

  type Props = {
    terminalId: string | null;
    output: string;
    onInput: (data: string) => void;
    onResize: (cols: number, rows: number) => void;
    onOpenTerminal: () => void;
    menuActions?: Snippet;
  };

  let { terminalId, output, onInput, onResize, onOpenTerminal, menuActions }: Props =
    $props();

  let host = $state<HTMLDivElement | null>(null);
  let xterm = $state<Terminal | null>(null);
  let fitAddon = $state<FitAddon | null>(null);
  let resizeObserver = $state<ResizeObserver | null>(null);
  let lastRenderedOutput = $state("");
  let activeTerminalId = $state<string | null>(null);
  let terminalOpened = $state(false);
  let pendingFitFrame = $state<number | null>(null);

  function fit() {
    fitAddon?.fit();
    if (xterm && terminalId) {
      onResize(xterm.cols, xterm.rows);
    }
  }

  function fitOnNextFrame() {
    if (pendingFitFrame !== null) {
      cancelAnimationFrame(pendingFitFrame);
    }
    pendingFitFrame = requestAnimationFrame(() => {
      pendingFitFrame = null;
      fit();
    });
  }

  onMount(() => {
    xterm = new Terminal({
      cursorBlink: true,
      fontFamily:
        'JetBrains Mono, ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace',
      fontSize: 12,
      theme: readXtermTheme(),
    });
    fitAddon = new FitAddon();
    xterm.loadAddon(fitAddon);
    xterm.onData((data) => onInput(data));

    return () => {
      if (pendingFitFrame !== null) {
        cancelAnimationFrame(pendingFitFrame);
      }
      resizeObserver?.disconnect();
      xterm?.dispose();
    };
  });

  $effect(() => {
    if (!xterm || !host || terminalOpened) {
      return;
    }

    xterm.open(host);
    terminalOpened = true;
    fitOnNextFrame();
    resizeObserver = new ResizeObserver(() => fitOnNextFrame());
    resizeObserver.observe(host);
  });

  $effect(() => {
    void themeStore.resolved;
    if (xterm && terminalOpened) {
      xterm.options.theme = readXtermTheme();
      xterm.refresh(0, xterm.rows - 1);
    }
  });

  $effect(() => {
    if (!xterm) {
      return;
    }

    if (terminalId !== activeTerminalId) {
      xterm.reset();
      lastRenderedOutput = "";
      activeTerminalId = terminalId;
      if (terminalId && output.length > 0) {
        xterm.write(output);
        lastRenderedOutput = output;
      }
      fitOnNextFrame();
    }

    if (!terminalId) {
      return;
    }

    if (!output.startsWith(lastRenderedOutput)) {
      xterm.reset();
      xterm.write(output);
      lastRenderedOutput = output;
      return;
    }

    const delta = output.slice(lastRenderedOutput.length);
    if (delta.length > 0) {
      xterm.write(delta);
      lastRenderedOutput = output;
    }
  });
</script>

<section class="flex h-full min-h-0 flex-col bg-surface">
  {#if menuActions}
    <div class="flex items-center justify-end px-2 pt-[5px] pb-[4px]">
      {@render menuActions()}
    </div>
  {/if}
  <div
    bind:this={host}
    data-native-selectable="terminal"
    class={`min-h-0 flex-1 overflow-hidden px-2 py-2 ${terminalId ? "" : "hidden"}`}
  ></div>
  {#if !terminalId}
    <div
      class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-4 text-center"
    >
      <Icon name="terminal" size="lg" class="text-text-subtle" />
      <div class="text-sm text-text-subtle">No terminal open</div>
      <button
        type="button"
        class="rounded-md border border-border bg-surface px-3 py-1.5 text-sm text-text transition-colors duration-75 hover:bg-surface-hover hover:border-text-muted"
        onclick={onOpenTerminal}
      >
        Open terminal
      </button>
      <span class="text-xs text-text-subtle">or press Ctrl+T</span>
    </div>
  {/if}
</section>
