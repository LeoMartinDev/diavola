<script lang="ts">
  import type { Snippet } from "svelte";

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();
</script>

<section class="min-h-0 overflow-hidden rounded-lg border border-border bg-surface">
  {@render children()}
</section>
