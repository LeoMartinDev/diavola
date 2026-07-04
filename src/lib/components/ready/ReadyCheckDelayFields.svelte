<script lang="ts">
  import type { ProcessForm } from "$lib/config/editorModel";
  import TextField from "$lib/components/ui/TextField.svelte";
  import { readyFieldState, readyErrorId } from "$lib/components/readyCheckHelpers.svelte";

  type Props = {
    process: ProcessForm;
    readyIssue: (process: ProcessForm, field: string) => string | null;
    onFieldBlur?: (key: string) => void;
  };

  let { process, readyIssue, onFieldBlur }: Props = $props();

  const delayField = $derived(readyFieldState(process, "delayDurationMs", readyIssue));
</script>

<div class="grid grid-cols-1 gap-3 md:grid-cols-[180px_minmax(0,1fr)]">
  <div>
    <TextField
      label="Duration ms"
      density="compact"
      type="number"
      min="0"
      class={delayField.className}
      aria-invalid={delayField.invalid}
      aria-describedby={delayField.describedBy}
      bind:value={process.delayDurationMs}
      onblur={() => onFieldBlur?.(`process.${process.id}.ready.delayDurationMs`)}
    />
    {#if delayField.message}
      <span id={readyErrorId(process.id, "delayDurationMs")} class="text-xs text-danger">{delayField.message}</span>
    {/if}
  </div>
</div>
