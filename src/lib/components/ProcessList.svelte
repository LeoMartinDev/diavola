<script lang="ts">
  import Card from "$lib/components/ui/layout/Card.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import { processRowAction, processActionEnabled, STATUS_COLOR as statusColor, type RowAction } from "$lib/utils/process";
  import type {
    ProcessRuntimeId,
    ProcessSnapshot,
    TerminalSnapshot,
  } from "$lib/types";

  type Props = {
    processes: ProcessSnapshot[];
    terminals: TerminalSnapshot[];
    selectedProcessRuntimeId: ProcessRuntimeId | null;
    selectedTerminalId: string | null;
    busy?: boolean;
    onSelectProcess: (runtimeId: ProcessRuntimeId) => void;
    onSelectTerminal: (terminalId: string) => void;
    onStop: (processName: string) => void;
    onStart: (processName: string) => void;
    onRestart: (processName: string) => void;
    onCloseTerminal: (terminalId: string) => void;
  };

  let {
    processes,
    terminals,
    selectedProcessRuntimeId,
    selectedTerminalId,
    busy = false,
    onSelectProcess,
    onSelectTerminal,
    onStop,
    onStart,
    onRestart,
    onCloseTerminal,
  }: Props = $props();
</script>

<div class="grid gap-1">
  {#if processes.length === 0 && terminals.length === 0}
    <Card class="px-3 py-6 text-center">
      <span class="text-xs leading-5 text-text-subtle">No process loaded</span>
    </Card>
  {:else}
    {#each processes as process (process.runtimeId)}
      {@const selected = process.runtimeId === selectedProcessRuntimeId}
      {@const action = processRowAction(process)}
      {@const restartable = !busy && (action === "stop" || action === "start")}
      <Card
        class={`group relative flex items-center gap-2.5 px-3 ${
          selected ? "bg-surface-raised" : ""
        }`}
        interactive={!selected}
      >
        <button
          type="button"
          class="grid min-w-0 flex-1 self-stretch py-1.5 grid-cols-[auto_minmax(0,1fr)] items-center gap-2.5 text-left"
          aria-current={selected ? "true" : undefined}
          onclick={() => onSelectProcess(process.runtimeId)}
        >
          {#if process.kind === "task"}
            <Icon
              name="task"
              size="sm"
              class={`shrink-0 ${statusColor[process.status]}`}
            />
          {:else}
            <Icon
              name="service"
              size="sm"
              class={`shrink-0 ${statusColor[process.status]}`}
            />
          {/if}
          <span class="min-w-0">
            <span
              class="block truncate select-none text-[13px] font-medium text-text"
              >{process.name}</span
            >
            <span
              class="block truncate select-none text-[11px] text-text-subtle"
              >{process.status}</span
            >
          </span>
        </button>

        <div
          class="flex shrink-0 items-center gap-0.5 transition opacity-0 group-hover:opacity-100 group-focus-within:opacity-100"
        >
          {#if action}
            {@const danger = action === "stop"}
            <button
              type="button"
              class={`grid h-6 w-6 place-items-center rounded text-text-subtle transition-colors duration-75 hover:bg-surface-hover disabled:cursor-not-allowed disabled:opacity-40 ${danger ? "hover:bg-danger/10 hover:text-danger" : "hover:text-text"}`}
              disabled={!processActionEnabled(process, busy)}
              aria-label={`${action === "stop" ? "Stop" : "Start"} ${process.name}`}
              title={`${action === "stop" ? "Stop" : "Start"} ${process.name}`}
              onclick={(event) => {
                event.stopPropagation();
                if (action === "stop") {
                  onStop(process.name);
                } else if (action === "start") {
                  onStart(process.name);
                }
              }}
            >
              {#if action === "stop"}
                <Icon name="stop" size="xs" />
              {:else}
                <Icon name="play" size="xs" />
              {/if}
            </button>
            <button
              type="button"
              class="grid h-6 w-6 place-items-center rounded text-text-subtle transition-colors duration-75 hover:bg-surface-hover hover:text-text disabled:cursor-not-allowed disabled:opacity-40"
              disabled={!restartable}
              aria-label={`Restart ${process.name}`}
              title={`Restart ${process.name}`}
              onclick={(event) => {
                event.stopPropagation();
                onRestart(process.name);
              }}
            >
              <!-- Restart: circular arrow -->
              <Icon name="restart" size="xs" />
            </button>
          {:else if process.kind !== "task"}
            <button
              type="button"
              class="grid h-6 w-6 place-items-center rounded text-text-subtle transition-colors duration-75 hover:bg-surface-hover disabled:cursor-not-allowed disabled:opacity-40"
              disabled
              aria-label={`${process.name}`}
              title={`${process.name}`}
            >
              <Icon name="play" size="xs" />
            </button>
          {/if}
        </div>
      </Card>
    {/each}

    {#each terminals.filter((t) => t.isOpen) as terminal (terminal.terminalId)}
      {@const selected = terminal.terminalId === selectedTerminalId}
      <Card
        class={`group relative flex items-center gap-2.5 px-3 ${
          selected ? "bg-surface-raised" : ""
        }`}
        interactive={!selected}
      >
        <button
          type="button"
          class="grid min-w-0 flex-1 self-stretch py-1.5 grid-cols-[auto_minmax(0,1fr)] items-center gap-2.5 text-left"
          aria-current={selected ? "true" : undefined}
          onclick={() => onSelectTerminal(terminal.terminalId)}
        >
          <Icon name="terminal" size="sm" class="shrink-0 text-text-subtle" />
          <span class="min-w-0">
            <span
              class="block truncate select-none text-[13px] font-medium text-text"
              >{terminal.title}</span
            >
            <span
              class="block truncate select-none text-[11px] text-text-subtle"
              >{terminal.cwd}</span
            >
          </span>
        </button>

        <div
          class="flex shrink-0 items-center gap-0.5 transition opacity-0 group-hover:opacity-100 group-focus-within:opacity-100"
        >
          <button
            type="button"
            class="grid h-6 w-6 place-items-center rounded text-text-subtle transition-colors duration-75 hover:bg-danger/10 hover:text-danger disabled:cursor-not-allowed disabled:opacity-40"
            aria-label={`Close ${terminal.title}`}
            title={`Close ${terminal.title}`}
            onclick={(event) => {
              event.stopPropagation();
              onCloseTerminal(terminal.terminalId);
            }}
          >
            <Icon name="close" size="xs" />
          </button>
        </div>
      </Card>
    {/each}
  {/if}
</div>
