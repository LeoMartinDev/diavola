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

  const httpUrlField = $derived(readyFieldState(process, "httpUrl", readyIssue));
  const intervalField = $derived(readyFieldState(process, "intervalMs", readyIssue));
  const timeoutField = $derived(readyFieldState(process, "timeoutMs", readyIssue));
</script>

<div class="grid grid-cols-1 gap-3 md:grid-cols-[180px_minmax(0,1fr)]">
  <div>
    <TextField
      label="URL"
      density="compact"
      class={httpUrlField.className}
      aria-invalid={httpUrlField.invalid}
      aria-describedby={httpUrlField.describedBy}
      bind:value={process.httpUrl}
      onblur={() => onFieldBlur?.(`process.${process.id}.ready.httpUrl`)}
    />
    {#if httpUrlField.message}
      <span id={readyErrorId(process.id, "httpUrl")} class="text-xs text-danger">{httpUrlField.message}</span>
    {/if}
  </div>

  <div class="grid grid-cols-1 gap-3 md:grid-cols-2">
    <div>
      <TextField
        label="Interval ms"
        density="compact"
        type="number"
        min="0"
        class={intervalField.className}
        aria-invalid={intervalField.invalid}
        aria-describedby={intervalField.describedBy}
        bind:value={process.intervalMs}
        onblur={() => onFieldBlur?.(`process.${process.id}.ready.intervalMs`)}
      />
      {#if intervalField.message}
        <span id={readyErrorId(process.id, "intervalMs")} class="text-xs text-danger">{intervalField.message}</span>
      {/if}
    </div>
    <div>
      <TextField
        label="Timeout ms"
        density="compact"
        type="number"
        min="0"
        class={timeoutField.className}
        aria-invalid={timeoutField.invalid}
        aria-describedby={timeoutField.describedBy}
        bind:value={process.timeoutMs}
        onblur={() => onFieldBlur?.(`process.${process.id}.ready.timeoutMs`)}
      />
      {#if timeoutField.message}
        <span id={readyErrorId(process.id, "timeoutMs")} class="text-xs text-danger">{timeoutField.message}</span>
      {/if}
    </div>
  </div>
</div>
