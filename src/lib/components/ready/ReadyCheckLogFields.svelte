<script lang="ts">
  import type { ProcessForm } from "$lib/config/editorModel";
  import CheckboxField from "$lib/components/ui/CheckboxField.svelte";
  import TextField from "$lib/components/ui/TextField.svelte";
  import { readyFieldState, readyErrorId } from "$lib/components/readyCheckHelpers.svelte";

  type Props = {
    process: ProcessForm;
    readyIssue: (process: ProcessForm, field: string) => string | null;
    onFieldBlur?: (key: string) => void;
  };

  let { process, readyIssue, onFieldBlur }: Props = $props();

  const logPatternField = $derived(readyFieldState(process, "logPattern", readyIssue));
  const timeoutField = $derived(readyFieldState(process, "timeoutMs", readyIssue));
</script>

<div class="grid grid-cols-1 gap-3 md:grid-cols-[180px_minmax(0,1fr)]">
  <div class="grid grid-cols-1 gap-3 md:grid-cols-[minmax(0,1fr)_120px]">
    <div>
      <TextField
        label="Pattern"
        density="compact"
        class={logPatternField.className}
        aria-invalid={logPatternField.invalid}
        aria-describedby={logPatternField.describedBy}
        bind:value={process.logPattern}
        onblur={() => onFieldBlur?.(`process.${process.id}.ready.logPattern`)}
      />
      {#if logPatternField.message}
        <span id={readyErrorId(process.id, "logPattern")} class="text-xs text-danger">{logPatternField.message}</span>
      {/if}
    </div>
    <CheckboxField
      label="Regex"
      class="mt-7 text-[12px] text-text-muted"
      bind:checked={process.logRegex}
      onblur={() => onFieldBlur?.(`process.${process.id}.ready.logRegex`)}
    />
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
