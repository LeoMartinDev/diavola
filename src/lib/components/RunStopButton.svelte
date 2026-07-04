<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";

  type Props = {
    active: boolean;
    busy?: boolean;
    disabled?: boolean;
    compact?: boolean;
    onRun: () => void;
    onStop: () => void;
  };

  let {
    active,
    busy = false,
    disabled = false,
    compact = false,
    onRun,
    onStop,
  }: Props = $props();

  const runClass =
    "inline-flex items-center justify-center border border-emerald-500/50 bg-emerald-500/10 text-emerald-500 transition-colors duration-75 hover:bg-emerald-500/20 disabled:cursor-not-allowed disabled:border-border disabled:text-text-subtle disabled:bg-transparent";
  const stopClass =
    "inline-flex items-center justify-center border border-danger/30 bg-danger/10 text-danger transition-colors duration-75 hover:bg-danger/20 disabled:cursor-not-allowed disabled:opacity-55";
  const busyClass =
    "inline-flex items-center justify-center border border-warning/30 bg-warning/10 text-warning cursor-not-allowed";
  const compactClass = "h-4 w-4 rounded-[4px] p-0";
</script>

{#if busy}
  <span
    class="{busyClass} {compact ? compactClass : 'h-8 rounded-md px-2 text-xs'}"
    aria-label="Busy"
    title="Operation in progress"
  >
    <Icon name="spinner" size={compact ? "xs" : "sm"} class="animate-spin" />
    {#if !compact}<span>Busy</span>{/if}
  </span>
{:else if active}
  <button
    type="button"
    class="{stopClass} {compact ? compactClass : 'h-8 rounded-md px-2 text-xs'}"
    onclick={onStop}
    disabled={busy}
    aria-label="Stop current run"
    title="Stop current run"
  >
    <Icon name="stop" size={compact ? "xs" : "sm"} />
    {#if !compact}<span>Stop</span>{/if}
  </button>
{:else}
  <button
    type="button"
    class="{runClass} {compact ? compactClass : 'h-8 rounded-md px-2 text-xs'}"
    onclick={onRun}
    disabled={disabled || busy}
    aria-label="Run project"
    title="Run project"
  >
    <Icon name="play" size={compact ? "xs" : "sm"} />
    {#if !compact}<span>Run</span>{/if}
  </button>
{/if}
