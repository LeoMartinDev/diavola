<script lang="ts">
  import type { ProcessStatus } from "$lib/types";

  type Props = {
    status: ProcessStatus;
    class?: string;
  };

  let { status, class: className = "" }: Props = $props();

  // color + glow for each state. Active states (running/ready/starting) get a
  // soft ring so live processes read as live at a glance.
  const styleByStatus: Record<ProcessStatus, { dot: string; glow: boolean }> = {
    pending: { dot: "bg-text-subtle", glow: false },
    blocked: { dot: "bg-warning", glow: false },
    starting: { dot: "bg-accent", glow: true },
    running: { dot: "bg-accent", glow: true },
    ready: { dot: "bg-success", glow: true },
    succeeded: { dot: "bg-success", glow: false },
    failed: { dot: "bg-danger", glow: false },
    stopping: { dot: "bg-warning", glow: true },
    stopped: { dot: "bg-text-subtle", glow: false },
  };
</script>

{#key status}
  <span
    class={`relative inline-flex h-2 w-2 shrink-0 items-center justify-center ${className}`}
  >
    {#if styleByStatus[status].glow}
      <span
        class={`absolute inline-flex h-2 w-2 animate-ping rounded-full opacity-40 ${styleByStatus[status].dot}`}
      ></span>
    {/if}
    <span class={`relative inline-flex h-2 w-2 rounded-full transition-colors duration-300 ${styleByStatus[status].dot}`}></span>
  </span>
{/key}
