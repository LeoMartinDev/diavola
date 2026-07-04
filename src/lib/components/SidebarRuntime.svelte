<script lang="ts">
  import ProcessList from "$lib/components/ProcessList.svelte";
  import Button from "$lib/components/ui/Button.svelte";
  import { runtimeStore } from "$lib/stores/runtime.svelte";

  const session = $derived(runtimeStore.session);

  async function openTerminal() {
    const s = session;
    const terminal = await runtimeStore.openTitledTerminal(
      s && !s.stoppedAt ? s.projectId : undefined,
    );
    if (terminal) runtimeStore.selectTerminal(terminal.terminalId);
  }
</script>

<section class="min-h-0">
  {#if runtimeStore.uiError}
    <div class="mb-3 rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger">
      <div class="break-words">{runtimeStore.uiError}</div>
      <Button variant="ghost" size="sm" class="mt-2 h-auto px-0 text-danger/70 hover:bg-transparent hover:text-danger" onclick={() => runtimeStore.clearError()}>
        Dismiss
      </Button>
    </div>
  {/if}

  <div class="mb-2 flex items-center justify-between px-1">
    <h2 class="text-[11px] font-semibold uppercase tracking-wider text-text-subtle">Processes</h2>
    {#if session}
      <span class="text-[11px] text-text-subtle">
        {session.processes.filter(p => p.status !== 'pending' && p.status !== 'stopped').length}/{session.processes.length}
      </span>
    {/if}
  </div>
  <ProcessList
    processes={session?.processes ?? []}
    terminals={[]}
    selectedProcessRuntimeId={runtimeStore.selectedProcessRuntimeId}
    selectedTerminalId={null}
    busy={runtimeStore.busy}
    onSelectProcess={(runtimeId) => runtimeStore.selectProcess(runtimeId)}
    onSelectTerminal={() => {}}
    onStart={(processName) => runtimeStore.startSessionProcess(processName)}
    onStop={(processName) => runtimeStore.stopSessionProcess(processName)}
    onRestart={(processName) => runtimeStore.restartSessionProcess(processName)}
    onCloseTerminal={() => {}}
  />

  <div class="mb-2 mt-4 flex items-center justify-between px-1">
    <h2 class="text-[11px] font-semibold uppercase tracking-wider text-text-subtle">Terminals</h2>
    {#if runtimeStore.projectId}
      <button
        type="button"
        class="text-[12px] text-text-subtle transition-colors hover:text-text"
        aria-label="Open terminal"
        title="Open terminal (Ctrl+T)"
        onclick={() => { void openTerminal(); }}
      >+</button>
    {/if}
  </div>
  {#if runtimeStore.terminals.some(t => t.isOpen)}
    <ProcessList
      processes={[]}
      terminals={runtimeStore.terminals}
      selectedProcessRuntimeId={null}
      selectedTerminalId={runtimeStore.selectedTerminalId}
      busy={runtimeStore.busy}
      onSelectProcess={() => {}}
      onSelectTerminal={(terminalId) => runtimeStore.selectTerminal(terminalId)}
      onStart={() => {}}
      onStop={() => {}}
      onRestart={() => {}}
      onCloseTerminal={(terminalId) => {
        runtimeStore.selectTerminal(terminalId);
        runtimeStore.closeSelectedTerminal();
      }}
    />
  {:else}
    <div class="px-3 py-2.5 text-[11px] text-text-subtle leading-relaxed">
      <kbd class="rounded border border-border px-1 py-px text-[10px] text-text-muted">Ctrl+T</kbd> to open a terminal
    </div>
  {/if}
</section>
