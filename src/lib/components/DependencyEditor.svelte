<script lang="ts">
  import type { DependencyCondition } from "$lib/types";
  import type { ProcessForm } from "$lib/config/editorModel";
  import Icon from "$lib/components/ui/Icon.svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import SelectField from "$lib/components/ui/SelectField.svelte";
  import AddButton from "$lib/components/ui/AddButton.svelte";
  import EmptyState from "$lib/components/ui/EmptyState.svelte";
  import { ERROR_FIELD_CLASS as errorFieldClass } from "$lib/utils/classes";

  type Props = {
    process: ProcessForm;
    processes: ProcessForm[];
    dependencyIssue: (
      process: ProcessForm,
      dependencyId: string,
    ) => string | null;
    onAdd: (process: ProcessForm) => void;
    onRemove: (process: ProcessForm, dependencyId: string) => void;
    onFieldBlur?: (key: string) => void;
    showHeading?: boolean;
    showAddButton?: boolean;
  };

  let {
    process,
    processes,
    dependencyIssue,
    onAdd,
    onRemove,
    onFieldBlur,
    showHeading = true,
    showAddButton = true,
  }: Props = $props();
</script>

<section class="grid gap-3">
  {#if showHeading}
    <div class="flex items-start justify-between gap-3">
      <div>
        <h3 class="text-sm font-semibold text-text">Dependencies</h3>
        <p class="mt-1 text-xs leading-5 text-text-subtle">
          Process launch order for this node.
        </p>
      </div>
    </div>
  {/if}

  {#if process.dependencies.length === 0}
      <EmptyState message="Starts without dependencies.">
        {#snippet children()}
          {#if showAddButton}
            <AddButton onclick={() => onAdd(process)} />
          {/if}
        {/snippet}
      </EmptyState>
  {:else}
    {#if showAddButton}
      <div class="flex justify-end mb-1">
        <AddButton onclick={() => onAdd(process)} />
      </div>
    {/if}
    <div class="grid gap-2.5">
      {#each process.dependencies as dependency, index (dependency.id)}
        {@const depError = dependencyIssue(process, dependency.id)}
        {@const depErrorId = `dependency-error-${process.id}-${dependency.id}`}
        {@const dependencyIndex = index + 1}
        <div class="grid gap-1">
          <div
            class="grid grid-cols-1 gap-2 md:grid-cols-[minmax(0,1fr)_150px_auto] md:items-start"
          >
            <SelectField
              aria-label={`Dependency process ${dependencyIndex}`}
              density="compact"
              aria-invalid={depError ? "true" : undefined}
              aria-describedby={depError ? depErrorId : undefined}
              class={depError ? errorFieldClass : ""}
              options={[
                { value: "", label: "Select process" },
                ...processes
                  .filter((candidate) => candidate.id !== process.id)
                  .map((candidate) => ({
                    value: candidate.name,
                    label: candidate.name,
                  })),
              ]}
              bind:value={dependency.processName}
              onblur={() =>
                onFieldBlur?.(
                  `process.${process.id}.dependency.${dependency.id}`,
                )}
            />
            <SelectField
              aria-label={`Dependency condition ${dependencyIndex}`}
              density="compact"
              options={[
                {
                  value: "ready" satisfies DependencyCondition,
                  label: "Ready",
                },
                {
                  value: "success" satisfies DependencyCondition,
                  label: "Success",
                },
              ]}
              bind:value={dependency.condition}
              onblur={() =>
                onFieldBlur?.(
                  `process.${process.id}.dependency.${dependency.id}`,
                )}
            />
            <IconButton
              label={`Remove dependency ${dependencyIndex}`}
              variant="ghost"
              size="sm"
              onclick={() => onRemove(process, dependency.id)}
              class="mt-1"
            >
              <Icon name="trash" size="sm" />
            </IconButton>
          </div>
          {#if depError}
            <span id={depErrorId} class="text-xs text-danger">{depError}</span>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</section>
