<script lang="ts">
  import type { EnvRow } from "$lib/config/editorModel";
  import Icon from "$lib/components/ui/Icon.svelte";
  import Button from "$lib/components/ui/Button.svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import TextField from "$lib/components/ui/TextField.svelte";
  import AddButton from "$lib/components/ui/AddButton.svelte";
  import EmptyState from "$lib/components/ui/EmptyState.svelte";
  import { ERROR_FIELD_CLASS as errorFieldClass } from "$lib/utils/classes";

  type Props = {
    rows: EnvRow[];
    processId?: string;
    issueFor: (key: string) => string | null;
    onAdd: () => void;
    onRemove: (id: string) => void;
    onFieldBlur?: (key: string) => void;
    showHeading?: boolean;
    showAddButton?: boolean;
  };

  let {
    rows = $bindable(),
    processId,
    issueFor,
    onAdd,
    onRemove,
    onFieldBlur,
    showHeading = true,
    showAddButton = true,
  }: Props = $props();

  const envScopeLabel = (value: string | undefined) =>
    value ? "Process" : "Global";
</script>

<section class="grid gap-3">
  {#if showHeading}
    <div class="flex items-start justify-between gap-3">
      <div>
        <h3 class="text-sm font-semibold text-text">Environment variables</h3>
        {#if processId}
          <p class="mt-1 text-xs leading-5 text-text-subtle">
            Injected into this process.
          </p>
        {/if}
      </div>
    </div>
  {/if}

  <div class="grid gap-2.5">
    {#if rows.length === 0}
      <EmptyState message={processId ? "No process variables." : "No global variables."}>
        {#snippet children()}
          {#if showAddButton}
            <AddButton onclick={onAdd} />
          {/if}
        {/snippet}
      </EmptyState>
    {:else}
      {#if showAddButton}
        <div class="mb-1 flex justify-end">
          <AddButton onclick={onAdd} />
        </div>
      {/if}
      {#each rows as row, index (row.id)}
        {@const envIssueKey = processId
          ? `process.${processId}.env.${row.id}.key`
          : `env.${row.id}.key`}
        {@const keyError = issueFor(envIssueKey)}
        {@const keyErrorId = `env-error-${row.id}-key`}
        {@const rowNumber = index + 1}
        {@const scopeLabel = envScopeLabel(processId)}
        <div
          class="grid grid-cols-1 gap-2 md:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)_auto] md:items-start"
        >
          <div class="grid gap-1">
            <TextField
              density="compact"
              aria-label={`${scopeLabel} environment variable key ${rowNumber}`}
              aria-invalid={keyError ? "true" : undefined}
              aria-describedby={keyError ? keyErrorId : undefined}
              class={keyError ? errorFieldClass : ""}
              placeholder="KEY"
              bind:value={row.key}
              onblur={() =>
                onFieldBlur?.(
                  processId
                    ? `process.${processId}.env.${row.id}.key`
                    : `env.${row.id}.key`,
                )}
            />
            {#if keyError}
              <span id={keyErrorId} class="text-xs text-danger">{keyError}</span
              >
            {/if}
          </div>
          <TextField
            density="compact"
            aria-label={`${scopeLabel} environment variable value ${rowNumber}`}
            placeholder="value"
            bind:value={row.value}
            onblur={() =>
              onFieldBlur?.(
                processId
                  ? `process.${processId}.env.${row.id}.value`
                  : `env.${row.id}.value`,
              )}
          />
          <IconButton
            label={`Remove ${scopeLabel.toLowerCase()} environment variable ${rowNumber}`}
            variant="ghost"
            size="sm"
            onclick={() => onRemove(row.id)}
            class="mt-1"
          >
            <Icon name="trash" size="sm" />
          </IconButton>
        </div>
      {/each}
    {/if}
  </div>
</section>
