<script lang="ts">
  import type { ProcessForm } from "$lib/config/editorModel";
  import CheckboxField from "$lib/components/ui/CheckboxField.svelte";
  import SelectField from "$lib/components/ui/SelectField.svelte";
  import {
    isSupportedReadyType,
    readyTypeOptions,
    readyErrorId,
    UNKNOWN_READY_TYPE_MESSAGE,
  } from "$lib/components/readyCheckHelpers.svelte";
  import ReadyCheckHttpFields from "$lib/components/ready/ReadyCheckHttpFields.svelte";
  import ReadyCheckLogFields from "$lib/components/ready/ReadyCheckLogFields.svelte";
  import ReadyCheckDelayFields from "$lib/components/ready/ReadyCheckDelayFields.svelte";
  import ReadyCheckCommandFields from "$lib/components/ready/ReadyCheckCommandFields.svelte";

  type Props = {
    process: ProcessForm;
    readyIssue: (process: ProcessForm, field: string) => string | null;
    onFieldBlur?: (key: string) => void;
    showHeading?: boolean;
  };

  let { process, readyIssue, onFieldBlur, showHeading = true }: Props = $props();
</script>

<section data-settings-surface="flat" class="grid gap-3">
  {#if showHeading}
    <div>
      <h3 class="text-sm font-semibold text-text">Readiness</h3>
      <p class="mt-1 text-xs leading-5 text-text-subtle">Optional startup probe for services that need an explicit ready signal.</p>
    </div>
  {/if}

  <CheckboxField
    label="Enable readiness check"
    class="text-[12px] text-text-muted"
    bind:checked={process.readyEnabled}
    onblur={() => onFieldBlur?.(`process.${process.id}.ready.enabled`)}
  />

  {#if process.readyEnabled}
    {@const unsupportedType = !isSupportedReadyType(process.readyType)}
    <div class="grid grid-cols-1 gap-3 md:grid-cols-[180px_minmax(0,1fr)]">
      <SelectField
        label="Type"
        density="compact"
        options={readyTypeOptions(process.readyType)}
        class={unsupportedType ? "border-danger focus:border-danger" : undefined}
        aria-invalid={unsupportedType ? "true" : undefined}
        aria-describedby={unsupportedType ? readyErrorId(process.id, "type") : undefined}
        bind:value={process.readyType}
        onblur={() => onFieldBlur?.(`process.${process.id}.ready.type`)}
      />

      {#if process.readyType === "http"}
        <ReadyCheckHttpFields {process} {readyIssue} {onFieldBlur} />
      {:else if process.readyType === "log"}
        <ReadyCheckLogFields {process} {readyIssue} {onFieldBlur} />
      {:else if process.readyType === "delay"}
        <ReadyCheckDelayFields {process} {readyIssue} {onFieldBlur} />
      {:else if process.readyType === "command"}
        <ReadyCheckCommandFields {process} {readyIssue} {onFieldBlur} />
      {:else}
        <p id={readyErrorId(process.id, "type")} class="text-xs text-danger">{UNKNOWN_READY_TYPE_MESSAGE}</p>
      {/if}
    </div>
  {/if}
</section>
