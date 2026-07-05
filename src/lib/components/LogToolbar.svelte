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
  let searchFocused = $state(false);

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
      onfocus={() => (searchFocused = true)}
      onblur={() => (searchFocused = false)}
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

  {#if searchFocused && regexError}
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
