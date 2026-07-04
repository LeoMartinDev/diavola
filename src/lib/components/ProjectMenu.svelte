<script lang="ts">
  import Menu from "$lib/components/ui/Menu.svelte";
  import type { MenuItem } from "$lib/components/ui/Menu.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import {
    canShowNativeMenu,
    showNativeMenu,
    type NativeMenuItem,
  } from "$lib/tauri/nativeMenu";
  import type { Selection } from "$lib/stores/runtime.svelte";
  import type { ProcessSnapshot, TerminalSnapshot } from "$lib/types";

  type LogActions = { copy: () => void; clear: () => void };

  type Props = {
    selection: Selection;
    selectedProcess: ProcessSnapshot | null;
    selectedTerminal: TerminalSnapshot | null;
    busy: boolean;
    logActions: LogActions | null;
    onRestartProcess: (name: string) => void;
    onStopProcess: (name: string) => void;
    launchLocked: boolean;
    onCloseTerminal: () => void;
    onOpenTerminal: () => void;
  };

  let {
    selection,
    selectedProcess,
    selectedTerminal,
    busy,
    logActions,
    onRestartProcess,
    onStopProcess,
    launchLocked,
    onCloseTerminal,
    onOpenTerminal,
  }: Props = $props();

  const nativeMenuAvailable = canShowNativeMenu();

  // The ⚙ menu is contextual: project actions always; process actions when a
  // process is selected; terminal actions when a terminal is selected.
  const items = $derived<MenuItem[]>(buildItems());
  const nativeItems = $derived<NativeMenuItem[]>(buildNativeItems());

  function buildItems(): MenuItem[] {
    const built: MenuItem[] = [];

    if (selection?.kind === "process" && selectedProcess) {
      const name = selectedProcess.name;
      built.push({
        label: `Restart ${name}`,
        icon: "restart",
        onSelect: () => onRestartProcess(name),
      });
      built.push({
        label: `Stop ${name}`,
        icon: "stop",
        onSelect: () => onStopProcess(name),
        danger: true,
        dividerAfter: true,
      });
      built.push({
        label: "Copy logs",
        icon: "copy",
        onSelect: () => logActions?.copy(),
        disabled: !logActions,
      });
      built.push({
        label: "Clear logs",
        icon: "clear",
        onSelect: () => logActions?.clear(),
        disabled: !logActions,
        danger: true,
      });
    }

    if (selection?.kind === "terminal" && selectedTerminal) {
      built.push({
        label: `Close ${selectedTerminal.title}`,
        icon: "close",
        onSelect: onCloseTerminal,
        danger: true,
      });
    }

    built.push({
      label: "Open terminal",
      icon: "terminal",
      onSelect: onOpenTerminal,
      disabled: busy,
    });

    return built;
  }

  function buildNativeItems(): NativeMenuItem[] {
    return items.map((item) => ({
      label: item.label,
      enabled: !item.disabled,
      danger: item.danger,
      dividerAfter: item.dividerAfter,
      action: item.onSelect,
    }));
  }

  async function openNativeMenu(event: MouseEvent) {
    const trigger = event.currentTarget as HTMLElement | null;
    const rect = trigger?.getBoundingClientRect();
    const hasMeasuredRect = !!rect && (rect.width > 0 || rect.height > 0);
    const x = hasMeasuredRect
      ? Math.round(rect.left)
      : Math.round(event.clientX);
    const y = hasMeasuredRect
      ? Math.round(rect.bottom)
      : Math.round(event.clientY);
    await showNativeMenu(nativeItems, { x, y });
  }
</script>

{#if nativeMenuAvailable}
  <IconButton
    label="Project menu"
    onclick={(event) => {
      void openNativeMenu(event);
    }}
  >
    <Icon name="more" size="md" />
  </IconButton>
{:else}
  <Menu label="Project menu" {items} />
{/if}
