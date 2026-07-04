<script lang="ts">
  import { onMount } from "svelte";

  import AppShell from "$lib/components/AppShell.svelte";
  import ConfigEditor from "$lib/components/ConfigEditor.svelte";
  import LogViewer from "$lib/components/LogViewer.svelte";
  import SidebarRuntime from "$lib/components/SidebarRuntime.svelte";
  import TerminalPane from "$lib/components/TerminalPane.svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import Button from "$lib/components/ui/Button.svelte";
  import Toast from "$lib/components/ui/Toast.svelte";
  import EmptyState from "$lib/components/ui/EmptyState.svelte";
  import TopFrame from "$lib/components/ui/layout/TopFrame.svelte";
  import { runtimeStore } from "$lib/stores/runtime.svelte";
  import { setWindowTitle } from "$lib/tauri/client";
  import { createShortcutRegistry } from "$lib/shortcuts/registry";
  import { themeStore } from "$lib/stores/theme.svelte";

  let configOpen = $state(false);

  const project = $derived(runtimeStore.project);
  const session = $derived(runtimeStore.session);
  const sessionActive = $derived(!!session && !session.stoppedAt);
  const selection = $derived(runtimeStore.selection);

  $effect(() => {
    const title = runtimeStore.windowTitle;
    document.title = title;
    setWindowTitle(title);
  });

  const selectedProcess = $derived(runtimeStore.selectedProcess);
  const selectedTerminal = $derived(runtimeStore.selectedTerminal);

  const navigableItems = $derived.by(() => {
    const items: Array<
      | { kind: "process"; runtimeId: string }
      | { kind: "terminal"; terminalId: string }
    > = [];
    if (session) {
      for (const p of session.processes) {
        items.push({ kind: "process", runtimeId: p.runtimeId });
      }
    }
    for (const t of runtimeStore.terminals.filter((t) => t.isOpen)) {
      items.push({ kind: "terminal", terminalId: t.terminalId });
    }
    return items;
  });

  function navigateList(direction: 1 | -1) {
    const items = navigableItems;
    if (items.length === 0) return;

    let currentIndex = -1;
    if (selection) {
      currentIndex = items.findIndex((item) => {
        if (item.kind === "process" && selection.kind === "process") {
          return item.runtimeId === selection.runtimeId;
        }
        if (item.kind === "terminal" && selection.kind === "terminal") {
          return item.terminalId === selection.terminalId;
        }
        return false;
      });
    }

    let nextIndex = currentIndex + direction;
    if (nextIndex < 0) nextIndex = items.length - 1;
    if (nextIndex >= items.length) nextIndex = 0;

    const item = items[nextIndex];
    if (item.kind === "process") {
      runtimeStore.selectProcess(item.runtimeId);
    } else {
      runtimeStore.selectTerminal(item.terminalId);
    }
  }

  const shortcutHandler = createShortcutRegistry([
    {
      key: "Mod+Enter",
      description: "Start the current project",
      handler: () => { void runtimeStore.startCurrentProject(); },
      guard: () => !!runtimeStore.projectId && !sessionActive && !runtimeStore.busy,
    },
    {
      key: "Mod+.",
      description: "Stop the current project",
      handler: () => { void runtimeStore.stopCurrentProject(); },
      guard: () => sessionActive && !runtimeStore.busy,
    },
    {
      key: "Mod+t",
      description: "Open a new terminal",
      handler: () => { void openTerminal(); },
      guard: () => !!runtimeStore.projectId && !runtimeStore.busy,
    },
    {
      key: "Mod+j",
      description: "Select next sidebar item",
      handler: () => navigateList(1),
    },
    {
      key: "Mod+k",
      description: "Select previous sidebar item",
      handler: () => navigateList(-1),
    },
  ]);

  onMount(() => {
    void runtimeStore.init();

    const onOpenConfig = () => openConfigDialog();

    document.addEventListener("diavola:open-config-dialog", onOpenConfig);

    return () => {
      void runtimeStore.teardown();
      document.removeEventListener("diavola:open-config-dialog", onOpenConfig);
    };
  });

  function openConfigDialog() {
    configOpen = true;
  }

  async function openTerminal() {
    const terminal = await runtimeStore.openTitledTerminal(
      session && !session.stoppedAt ? session.projectId : undefined,
    );
    if (terminal) {
      runtimeStore.selectTerminal(terminal.terminalId);
    }
  }
</script>

<svelte:head>
  <title>{runtimeStore.windowTitle}</title>
</svelte:head>

<svelte:window onkeydown={shortcutHandler} />

{#snippet titleBar()}
  <TopFrame>
    <TitleBar />
  </TopFrame>
{/snippet}

{#snippet processList()}
  <SidebarRuntime />
{/snippet}

{#if configOpen}
  <main class="flex h-full min-h-0 flex-col gap-2 overflow-hidden bg-canvas text-text">
    {@render titleBar()}
    <div class="min-h-0 flex-1 overflow-hidden rounded-lg border border-border bg-surface mx-2 mb-2">
      <ConfigEditor
        open={configOpen}
        {project}
        onClose={() => {
          configOpen = false;
        }}
      />
    </div>
    <Toast />
  </main>
{:else}
  <AppShell {titleBar} {processList}>
            {#if selection?.kind === "terminal" && selectedTerminal}
              <TerminalPane
                terminalId={selectedTerminal.terminalId}
                output={runtimeStore.terminalOutput[selectedTerminal.terminalId] ?? ""}
                onInput={(data) => runtimeStore.writeToTerminal(data)}
                onResize={(cols, rows) => runtimeStore.resizeSelectedTerminal(cols, rows)}
                onOpenTerminal={openTerminal}
              />
          {:else if selection?.kind === "process" && session}
            <LogViewer
              logs={runtimeStore.logsForSelectedProcess()}
              processName={selectedProcess?.name ?? null}
              truncatedCount={runtimeStore.truncatedLogCountForSelectedProcess()}
              onClear={() => runtimeStore.clearSelectedProcessLogs()}
              onActions={(actions) => (runtimeStore.logActions = actions)}
            />
          {:else}
            <div class="grid h-full place-items-center px-6 text-center">
              <div class="max-w-sm">
                <div class="text-sm font-semibold text-text">
                  {session ? "Select a process or terminal" : "No process is running"}
                </div>
                <p class="mt-2 text-sm leading-6 text-text-subtle">
                  {session
                    ? "Choose an item in the sidebar to view its logs or terminal."
                    : "Open Runtime config in Settings to add processes and create diavola.yml in this workspace."}
                </p>
              </div>
            </div>
          {/if}
  </AppShell>
{/if}
