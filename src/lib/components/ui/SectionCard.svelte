<script lang="ts">
  type Props = {
    class?: string;
    children?: import("svelte").Snippet;
    testid?: string;
  };

  let { class: cls = "", children, testid }: Props = $props();
</script>

<div data-testid={testid} class={`rounded-xl border border-border/60 bg-surface-raised/70 p-4 shadow-sm ${cls}`}>
  {#if children}
    {@render children()}
  {/if}
</div>
