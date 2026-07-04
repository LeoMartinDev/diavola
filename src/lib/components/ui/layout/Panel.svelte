<script lang="ts">
  import type { Snippet } from "svelte";

  type Props = {
    children: Snippet;
    class?: string;
    border?: boolean;
  };

  let { children, class: className = "", border = false }: Props = $props();
</script>

<div
  class={`rounded-lg bg-surface-raised ${border ? "border border-border" : ""} ${className}`}
>
  {@render children()}
</div>
