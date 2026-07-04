<script lang="ts">
  import type { Snippet } from "svelte";

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();
</script>

<aside class="min-h-0 overflow-hidden bg-canvas">
  <div class="h-full overflow-y-auto px-3 pb-4 pt-4">
    {@render children()}
  </div>
</aside>
