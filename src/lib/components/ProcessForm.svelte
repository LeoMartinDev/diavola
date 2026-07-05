<script lang="ts">
  import type { ProcessForm as ProcessFormState } from "$lib/config/editorModel";
  import CommandCodeEditor from "$lib/components/CommandCodeEditor.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import SelectField from "$lib/components/ui/SelectField.svelte";
  import TextField from "$lib/components/ui/TextField.svelte";

  type Props = {
    process: ProcessFormState;
    processCount: number;
    processIssue: (process: ProcessFormState, field: string) => string | null;
    onRemove: (id: string) => void;
    onFieldBlur?: (key: string) => void;
    showHeader?: boolean;
  };

  let {
    process,
    processCount,
    processIssue,
    onRemove,
    onFieldBlur,
    showHeader = true,
  }: Props = $props();
</script>

<section class="grid gap-4">
  {#if showHeader}
    <div class="flex items-start justify-between gap-3">
      <div>
        <h3 class="text-[13px] font-semibold text-text">
          Process: {process.name || "Unnamed"}
        </h3>
        <p class="mt-1 text-xs leading-5 text-text-subtle">
          Name, command, and kind for this runtime node.
        </p>
      </div>
      <IconButton
        label="Remove process"
        variant="ghost"
        size="sm"
        onclick={() => onRemove(process.id)}
        disabled={processCount <= 1}
      >
        <Icon name="trash" size="sm" />
      </IconButton>
    </div>
  {/if}

  <div class="grid grid-cols-1 gap-3 md:grid-cols-[220px_minmax(0,1fr)]">
    <TextField
      label="Name"
      density="compact"
      error={processIssue(process, "name")}
      bind:value={process.name}
      onblur={() => onFieldBlur?.(`process.${process.id}.name`)}
    />
    <SelectField
      label="Kind"
      density="compact"
      options={[
        { value: "service", label: "Service" },
        { value: "task", label: "Task" },
      ]}
      bind:value={process.kind}
      onblur={() => onFieldBlur?.(`process.${process.id}.kind`)}
    />
  </div>

  <CommandCodeEditor
    label="Command"
    placeholder="deno task dev"
    error={processIssue(process, "cmd")}
    bind:value={process.cmd}
    onblur={() => onFieldBlur?.(`process.${process.id}.cmd`)}
  />

  <TextField
    label="Stop timeout (ms)"
    density="compact"
    placeholder="10000"
    error={processIssue(process, "stopTimeoutMs")}
    bind:value={process.stopTimeoutMs}
    onblur={() => onFieldBlur?.(`process.${process.id}.stopTimeoutMs`)}
  />
</section>
