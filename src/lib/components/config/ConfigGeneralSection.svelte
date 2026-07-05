<script lang="ts">
  import type { ProjectRecord } from "$lib/types";
  import TextField from "$lib/components/ui/TextField.svelte";

  type Props = {
    project: ProjectRecord | null;
    projectSourceLabel: string;
    globalGracePeriodMs: number | string | null;
    globalGracePeriodError: string | null;
    onGlobalGracePeriodChange: (value: number | string | null) => void;
  };

  let {
    project,
    projectSourceLabel,
    globalGracePeriodMs,
    globalGracePeriodError,
    onGlobalGracePeriodChange,
  }: Props = $props();
</script>

<section class="grid gap-4">
  <div>
    <h2 class="text-base font-semibold text-text">General</h2>
    <p class="mt-1 text-sm leading-6 text-text-subtle">
      Project-scoped runtime settings for the current workspace.
    </p>
  </div>

  <div class="grid gap-3 sm:grid-cols-2">
    <div class="rounded-lg border border-border/70 bg-surface-raised/65 px-4 py-3">
      <div class="text-[11px] font-semibold uppercase tracking-[0.16em] text-text-subtle">
        Project
      </div>
      <div class="mt-2 text-sm font-medium text-text">
        {project?.name ?? "No project selected"}
      </div>
      <div class="mt-1 wrap-break-word text-xs leading-5 text-text-subtle">
        {project?.baseDir ?? "Select a project to load its runtime config."}
      </div>
    </div>

    <div class="rounded-lg border border-border/70 bg-surface-raised/65 px-4 py-3">
      <div class="text-[11px] font-semibold uppercase tracking-[0.16em] text-text-subtle">
        Config source
      </div>
      <div class="mt-2 text-sm font-medium text-text">
        {projectSourceLabel}
      </div>
      <div class="mt-1 wrap-break-word text-xs leading-5 text-text-subtle">
        {project?.configPath ?? "The backend will resolve diavola.yml when a project is selected."}
      </div>
    </div>
  </div>

  <TextField
    label="Default grace period (ms)"
    density="compact"
    placeholder="10000"
    error={globalGracePeriodError}
    value={globalGracePeriodMs ?? ""}
    oninput={(e) => onGlobalGracePeriodChange((e.currentTarget as HTMLInputElement).value)}
  />
</section>
