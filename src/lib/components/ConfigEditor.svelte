<script lang="ts">
  import { useConfigEditor } from "$lib/components/config/configEditorLogic.svelte";
  import ConfigGeneralSection from "$lib/components/config/ConfigGeneralSection.svelte";
  import ConfigEnvironmentSection from "$lib/components/config/ConfigEnvironmentSection.svelte";
  import ConfigProcessDetail from "$lib/components/config/ConfigProcessDetail.svelte";
  import ConfigNav from "$lib/components/config/ConfigNav.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import AddButton from "$lib/components/ui/AddButton.svelte";
  import type { ProjectRecord } from "$lib/types";
  import SegmentedControl from "$lib/components/ui/SegmentedControl.svelte";
  import { themeStore } from "$lib/stores/theme.svelte";
  import { untrack } from "svelte";

  const CONFIG_SAVE_DEBOUNCE_MS = 400;

  type Props = {
    open: boolean;
    project: ProjectRecord | null;
    onClose: () => void;
  };

  let { open, project, onClose }: Props = $props();

  const projectFn = () => project;
  const openFn = () => open;

  const ctx = useConfigEditor(projectFn, openFn);

  $effect(() => {
    if (!open) { ctx.processesViewMode = "list"; return; }
    ctx.activeSection = "settings-general";
    ctx.processesViewMode = "list";
  });

  $effect(() => {
    if (ctx.processesViewMode === "detail") ctx.activeSection = "settings-processes";
  });

  $effect(() => {
    const projectId = project?.id ?? null;
    if (!open) { ctx.dialogWasOpen = false; return; }
    if (!projectId) return;
    const openedNow = !ctx.dialogWasOpen;
    ctx.dialogWasOpen = true;
    if (openedNow || projectId !== ctx.loadedProjectId) void ctx.load(projectId);
  });

  $effect(() => {
    if (!open || ctx.loadedProjectId === null) return;
    if (untrack(() => ctx.suppressDirty)) return;
    const yaml = ctx.currentYaml();
    if (yaml === ctx.lastSavedYaml) return;
    const timer = untrack(() => ctx.debounceTimer);
    if (timer !== null) clearTimeout(timer);
    const projectId = ctx.loadedProjectId;
    untrack(() => { ctx.debounceTimer = setTimeout(() => { void ctx.autoSave(projectId); }, CONFIG_SAVE_DEBOUNCE_MS); });
    return () => {
      const t = untrack(() => ctx.debounceTimer);
      if (t !== null) clearTimeout(t);
    };
  });
</script>

{#snippet pageBody()}
  <div
    class="grid h-full min-h-0 grid-cols-1 overflow-hidden lg:grid-cols-[248px_minmax(0,1fr)]"
  >
    <ConfigNav
      activeSection={ctx.activeSection}
      onNavigate={ctx.handleNav}
      {onClose}
    />

    <div class="min-h-0 overflow-y-auto bg-surface">
      <div
        class="mx-auto flex w-full max-w-[52rem] flex-col gap-8 px-20 py-8 lg:px-32 lg:py-10"
      >
        {#if ctx.loading}
          <div class="text-sm text-text-subtle">Loading settings...</div>
        {:else if ctx.loadError}
          <div
            class="grid gap-3 rounded-xl border border-danger/40 bg-danger/10 p-4 text-sm"
          >
            <div class="font-medium text-danger">
              Configuration could not be loaded
            </div>
            <div class="text-danger/80">{ctx.loadError}</div>
            <div class="text-text-subtle">
              The settings form is disabled so an empty generated configuration
              cannot overwrite the project YAML.
            </div>
          </div>
        {:else}
          <div>
            <h3 class="text-text-subtle text-xs font-semibold uppercase tracking-wider">
              App Settings
            </h3>
            <div class="flex items-center justify-between mt-3">
              <span class="text-sm text-text-muted">Appearance</span>
              <SegmentedControl
                value={themeStore.theme}
                options={[
                  { value: "auto", label: "Auto", icon: "monitor" },
                  { value: "light", label: "Light", icon: "sun" },
                  { value: "dark", label: "Dark", icon: "moon" },
                ]}
                label="Theme"
                onChange={(v) => themeStore.setTheme(v as "auto" | "light" | "dark")}
              />
            </div>
          </div>
          <div class="border-t border-border"></div>
          <h3 class="text-text-subtle text-xs font-semibold uppercase tracking-wider">
            Project Settings
          </h3>
          {#if ctx.status && (ctx.formIssueCount > 0 || ctx.statusIsError)}
            <div
              class={`text-xs ${ctx.formIssueCount > 0 || ctx.statusIsError ? "text-danger" : "text-text-subtle"}`}
            >
              {ctx.status}
            </div>
          {/if}

          {#if ctx.activeSection === "settings-general"}
            <ConfigGeneralSection
              {project}
              projectSourceLabel={ctx.projectSourceLabel}
              globalGracePeriodMs={ctx.globalGracePeriodMs}
              globalGracePeriodError={ctx.globalGracePeriodError()}
              onGlobalGracePeriodChange={ctx.onGlobalGracePeriodChange}
            />
          {:else if ctx.activeSection === "settings-environment"}
            <ConfigEnvironmentSection
              bind:rows={ctx.globalEnvRows}
              issueFor={(key: string) => ctx.issueFor(key)}
              onAdd={ctx.addGlobalEnvRow}
              onRemove={ctx.removeGlobalEnvRow}
              onFieldBlur={ctx.markTouched}
            />
          {:else if ctx.activeSection === "settings-processes"}
            {#if ctx.processesViewMode === "detail" && ctx.selectedProcess}
              <ConfigProcessDetail
                process={ctx.selectedProcess}
                processes={ctx.processes}
                processCount={ctx.processes.length}
                processIssue={ctx.processIssue}
                dependencyIssue={ctx.dependencyIssue}
                readyIssue={ctx.readyIssue}
                issueFor={(key: string) => ctx.issueFor(key)}
                onFieldBlur={ctx.markTouched}
                onRemove={ctx.removeProcess}
                addEnvRow={ctx.addEnvRow}
                removeEnvRow={ctx.removeEnvRow}
                addDependency={ctx.addDependency}
                removeDependency={ctx.removeDependency}
                onBack={() => { ctx.processesViewMode = "list"; }}
              />
            {:else}
              {#snippet processListContent()}
                <section class="grid gap-5">
                  <div class="flex items-start justify-between gap-3">
                    <div>
                      <h2 class="text-base font-semibold text-text">Processes</h2>
                      <p class="mt-1 text-sm leading-6 text-text-subtle">
                        Manage configured local processes below.
                      </p>
                    </div>
                    <AddButton
                      onclick={ctx.addProcess}
                      disabled={ctx.loading || ctx.loadError !== null}
                    />
                  </div>

                  <div class="rounded-xl border border-border/60 bg-surface-raised/70 shadow-sm">
                    {#if ctx.processes.length === 0}
                      <div class="px-4 py-5 text-sm text-text-subtle">
                        Add a process to start building the runtime graph.
                      </div>
                    {:else}
                      <ul aria-label="Configured processes" class="divide-y divide-border/70">
                        {#each ctx.processes as process (process.id)}
                          {@const processLabel = process.name || "Unnamed process"}
                          <li>
                            <button
                              type="button"
                              class="group flex w-full items-center justify-between gap-3 px-4 py-3 text-left transition-colors duration-75 hover:bg-surface-hover/70"
                              aria-label={`Open process ${processLabel} (${process.kind})`}
                              onclick={() => {
                                ctx.selectedProcessId = process.id;
                                ctx.processesViewMode = "detail";
                              }}
                            >
                              <span class="min-w-0">
                                <span class="block truncate text-sm font-medium text-text">{processLabel}</span>
                                <span class="mt-0.5 block truncate text-[11px] text-text-subtle">{process.kind}</span>
                              </span>
                              <div class="flex shrink-0 items-center gap-1">
                                <span
                                  role="button"
                                  tabindex="0"
                                  class="grid h-6 w-6 place-items-center rounded text-text-subtle opacity-0 transition-opacity duration-75 hover:bg-danger/10 hover:text-danger group-hover:opacity-100 disabled:cursor-not-allowed disabled:opacity-30"
                                  aria-label={`Delete process ${processLabel}`}
                                  title={`Delete ${processLabel}`}
                                  onclick={(e) => { e.stopPropagation(); ctx.removeProcess(process.id); }}
                                  onkeydown={(e) => {
                                    if (e.key === "Enter" || e.key === " ") { e.stopPropagation(); ctx.removeProcess(process.id); }
                                  }}
                                  aria-disabled={ctx.processes.length <= 1}
                                >
                                  <Icon name="trash" size="sm" />
                                </span>
                                <Icon name="chevron-right" size="sm" class="shrink-0 text-text-subtle" />
                              </div>
                            </button>
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  </div>
                </section>
              {/snippet}
              {@render processListContent()}
            {/if}
          {/if}
        {/if}
      </div>
    </div>
  </div>
{/snippet}

{#if open}
  <section class="flex h-full min-h-0 flex-col bg-surface text-text">
    {@render pageBody()}
  </section>
{/if}
