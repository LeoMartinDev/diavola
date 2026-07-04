<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";
  import Button from "$lib/components/ui/Button.svelte";
  import type { ProcessForm } from "$lib/config/editorModel";

  type Props = {
    processes: ProcessForm[];
    loading?: boolean;
    disabled?: boolean;
    onSelect: (id: string) => void;
    onAdd: () => void;
  };

  let {
    processes,
    loading = false,
    disabled = false,
    onSelect,
    onAdd,
  }: Props = $props();
</script>

<aside
  class="flex min-h-0 flex-col border-b border-border bg-surface lg:border-b-0 lg:border-r"
>
  <div
    class="flex items-center justify-between gap-3 border-b border-border px-4 py-3 lg:block lg:py-4"
  >
    <h2 class="text-xs font-semibold text-text">Processes</h2>
    <Button
      class="shrink-0 lg:hidden"
      size="xs"
      variant="ghost"
      onclick={onAdd}
      disabled={loading || disabled}
    >
      <Icon name="plus" size="sm" />
      <span>New</span>
    </Button>
  </div>
  <div class="hidden border-b border-border p-3 lg:block">
    <Button
      class="w-full"
      size="xs"
      variant="ghost"
      onclick={onAdd}
      disabled={loading || disabled}
    >
      <Icon name="plus" size="sm" />
      <span>New</span>
    </Button>
  </div>
  <div class="min-h-0 overflow-y-auto py-1">
    {#each processes as process (process.id)}
      <button
        type="button"
        class="flex w-full items-center gap-2 px-4 py-2 text-left text-[13px] text-text-muted hover:bg-surface-hover hover:text-text transition-colors duration-75"
        onclick={() => onSelect(process.id)}
      >
        {process.name || "Unnamed"}
      </button>
    {/each}
    {#if processes.length === 0}
      <div class="px-4 py-2 text-[12px] text-text-subtle">
        No processes configured
      </div>
    {/if}
  </div>
</aside>
