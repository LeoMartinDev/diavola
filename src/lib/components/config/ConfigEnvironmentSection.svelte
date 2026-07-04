<script lang="ts">
  import type { EnvRow } from "$lib/config/editorModel";
  import EnvEditor from "$lib/components/EnvEditor.svelte";
  import AddButton from "$lib/components/ui/AddButton.svelte";
  import SectionCard from "$lib/components/ui/SectionCard.svelte";

  type Props = {
    rows: EnvRow[];
    issueFor: (key: string) => string | null;
    onAdd: () => void;
    onRemove: (rowId: string) => void;
    onFieldBlur: (key: string) => void;
  };

  let { rows = $bindable(), issueFor, onAdd, onRemove, onFieldBlur }: Props = $props();
</script>

<section class="grid gap-4">
  <div class="flex items-start justify-between gap-3">
    <div>
      <h2 class="text-base font-semibold text-text">Environment</h2>
      <p class="mt-1 text-sm leading-6 text-text-subtle">
        Shared variables are injected into every configured process.
      </p>
    </div>
    <AddButton onclick={onAdd} />
  </div>

  <SectionCard>
    <EnvEditor
      bind:rows
      {issueFor}
      {onAdd}
      {onRemove}
      {onFieldBlur}
      showHeading={false}
      showAddButton={false}
    />
  </SectionCard>
</section>
