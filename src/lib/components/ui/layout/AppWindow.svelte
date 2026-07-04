<script lang="ts">
  import type { Snippet } from "svelte";

  type Props = {
    topFrame: Snippet;
    sidebar: Snippet;
    main: Snippet;
  };

  let { topFrame, sidebar, main }: Props = $props();
</script>

<div class="flex h-full flex-col gap-2 bg-canvas">
  {@render topFrame()}

  <div class="grid min-h-0 flex-1 grid-cols-[260px_minmax(0,1fr)] gap-2 overflow-hidden px-2 pb-2">
    {@render sidebar()}
    {@render main()}
  </div>
</div>
