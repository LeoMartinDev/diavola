<script lang="ts">
  import type { ProcessForm as ProcessFormState } from "$lib/config/editorModel";
  import ProcessForm from "$lib/components/ProcessForm.svelte";
  import EnvEditor from "$lib/components/EnvEditor.svelte";
  import DependencyEditor from "$lib/components/DependencyEditor.svelte";
  import ReadyCheckEditor from "$lib/components/ReadyCheckEditor.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import AddButton from "$lib/components/ui/AddButton.svelte";
  import SectionCard from "$lib/components/ui/SectionCard.svelte";

  type Props = {
    process: ProcessFormState;
    processes: ProcessFormState[];
    processCount: number;
    processIssue: (process: ProcessFormState, field: string) => string | null;
    dependencyIssue: (process: ProcessFormState, depId: string) => string | null;
    readyIssue: (process: ProcessFormState, field: string) => string | null;
    issueFor: (key: string) => string | null;
    onFieldBlur: (key: string) => void;
    onRemove: (id: string) => void;
    addEnvRow: (process: ProcessFormState) => void;
    removeEnvRow: (process: ProcessFormState, id: string) => void;
    addDependency: (process: ProcessFormState) => void;
    removeDependency: (process: ProcessFormState, depId: string) => void;
    onBack: () => void;
  };

  let {
    process,
    processes,
    processCount,
    processIssue,
    dependencyIssue,
    readyIssue,
    issueFor,
    onFieldBlur,
    onRemove,
    addEnvRow,
    removeEnvRow,
    addDependency,
    removeDependency,
    onBack,
  }: Props = $props();
</script>

<div data-testid="process-detail-scroll" class="flex h-full min-h-0 flex-col gap-6 overflow-y-auto pr-1">
  <div class="flex items-center justify-between gap-3">
    <button
      type="button"
      class="inline-flex w-fit items-center gap-1 rounded px-1.5 py-px text-[11px] text-text-subtle transition-colors duration-75 hover:bg-surface-hover/70 hover:text-text"
      onclick={onBack}
      aria-label="Back to processes"
    >
      <Icon name="back" size="xs" />
      Back
    </button>
    <IconButton
      label="Delete process"
      variant="ghost"
      size="sm"
      onclick={() => onRemove(process.id)}
      disabled={processCount <= 1}
    >
      <Icon name="trash" size="sm" />
    </IconButton>
  </div>

  <section class="grid gap-1">
    <h2 class="text-xl font-semibold text-text">
      {process.name || "Unnamed process"}
    </h2>
    <p class="text-sm text-text-subtle">
      {process.kind === "task" ? "Task process configuration." : "Service process configuration."}
    </p>
  </section>

  <section class="grid gap-2">
    <h3 class="text-[13px] font-semibold text-text">Details</h3>
    <SectionCard testid="process-detail-card">
      <ProcessForm
        {process}
        {processCount}
        {processIssue}
        {onRemove}
        {onFieldBlur}
        showHeader={false}
      />
    </SectionCard>
  </section>

  <section class="grid gap-2">
    <div class="flex items-center justify-between gap-3">
      <h3 class="text-[13px] font-semibold text-text">Environment</h3>
      <AddButton onclick={() => addEnvRow(process)} />
    </div>
    <SectionCard testid="process-detail-card">
      <EnvEditor
        bind:rows={process.envRows}
        processId={process.id}
        {issueFor}
        onAdd={() => addEnvRow(process)}
        onRemove={(id) => removeEnvRow(process, id)}
        {onFieldBlur}
        showHeading={false}
        showAddButton={false}
      />
    </SectionCard>
  </section>

  <section class="grid gap-2">
    <div class="flex items-center justify-between gap-3">
      <h3 class="text-[13px] font-semibold text-text">Dependencies</h3>
      <AddButton onclick={() => addDependency(process)} />
    </div>
    <SectionCard testid="process-detail-card">
      <DependencyEditor
        {process}
        {processes}
        {dependencyIssue}
        onAdd={addDependency}
        onRemove={removeDependency}
        {onFieldBlur}
        showHeading={false}
        showAddButton={false}
      />
    </SectionCard>
  </section>

  <section class="grid gap-2">
    <h3 class="text-[13px] font-semibold text-text">Readiness</h3>
    <SectionCard testid="process-detail-card">
      <ReadyCheckEditor
        {process}
        {readyIssue}
        {onFieldBlur}
        showHeading={false}
      />
    </SectionCard>
  </section>
</div>
