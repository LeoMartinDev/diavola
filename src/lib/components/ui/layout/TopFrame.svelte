<script lang="ts">
  import type { Snippet } from "svelte";

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();
</script>

<div class="flex h-[44px] shrink-0 items-center overflow-hidden bg-canvas">
  {@render children()}
</div>
