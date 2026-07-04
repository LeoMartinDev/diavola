<script lang="ts">
  import type { Snippet } from "svelte";

  type Props = {
    children: Snippet;
    class?: string;
    interactive?: boolean;
  };

  let { children, class: className = "", interactive = false }: Props = $props();
</script>

<div
  class={`rounded-md transition-colors duration-75 ${
    interactive ? "hover:bg-surface" : ""
  } ${className}`}
>
  {@render children()}
</div>
