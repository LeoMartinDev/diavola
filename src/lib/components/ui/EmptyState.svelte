<script lang="ts">
  type Props = {
    message: string;
    children?: import("svelte").Snippet;
  };

  let { message, children }: Props = $props();
</script>

<div class="px-1 text-sm text-text-subtle">
  {message}
  {#if children}
    <div class="mt-2 flex justify-center">
      {@render children()}
    </div>
  {/if}
</div>
