<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";

  type Props = {
    query: string;
    autoScroll: boolean;
    paused: boolean;
    matchCount: number | null;
    onTogglePause: () => void;
    onClear: () => void;
  };

  let {
    query = $bindable(),
    autoScroll = $bindable(),
    paused = $bindable(),
    matchCount,
    onTogglePause,
    onClear,
  }: Props = $props();

  let searchInput = $state<HTMLInputElement | null>(null);

  export function focusSearch() {
    searchInput?.focus();
  }
</script>

<div class="flex items-center gap-1.5 border-b border-border px-2 pt-[4px] pb-[5px]">
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
      class="log-search h-7 w-full rounded-md border border-border bg-surface-raised pl-6 pr-2 text-[12px] text-text outline-none transition-colors duration-75 placeholder:text-[11px] placeholder:text-text-subtle focus:border-accent"
    />
    {#if matchCount !== null}
      <span class="pointer-events-none absolute right-1.5 top-1/2 -translate-y-1/2 text-[10px] text-text-subtle">
        {matchCount}
      </span>
    {/if}
  </div>

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
      const active = buttons.find((b) => b.getAttribute("aria-pressed") === "true") ?? buttons[0];
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
      class="grid h-6 w-6 place-items-center rounded-md transition-colors duration-75 {autoScroll
        ? 'bg-accent/15 text-accent'
        : 'text-text-subtle hover:bg-surface-hover hover:text-text'}"
      onclick={() => (autoScroll = !autoScroll)}
      aria-pressed={autoScroll}
      aria-label={autoScroll ? "Auto-scroll on" : "Auto-scroll off"}
      title={autoScroll ? "Auto-scroll: on" : "Auto-scroll: off"}
    >
      <Icon name="scroll-down" size="xs" />
    </button>

    <button
      type="button"
      tabindex="-1"
      class="grid h-6 w-6 place-items-center rounded-md transition-colors duration-75 {paused
        ? 'bg-accent/15 text-accent'
        : 'text-text-subtle hover:bg-surface-hover hover:text-text'}"
      onclick={onTogglePause}
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
      class="grid h-6 w-6 place-items-center rounded-md text-text-subtle transition-colors duration-75 hover:bg-surface-hover hover:text-danger"
      onclick={onClear}
      aria-label="Clear logs"
      title="Clear logs"
    >
      <Icon name="clear" size="xs" />
    </button>
  </div>
</div>
