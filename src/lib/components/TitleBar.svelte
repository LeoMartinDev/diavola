<script lang="ts">
  import ProjectChip from "$lib/components/ProjectChip.svelte";
  import WindowControls from "$lib/components/WindowControls.svelte";
  import { runtimeStore } from "$lib/stores/runtime.svelte";
  import { isDev } from "$lib/tauri/environment";
  import { startWindowDrag } from "$lib/tauri/window";

  const isMac = navigator.platform.toLowerCase().includes("mac");
  const isDevMode = isDev();

  const project = $derived(runtimeStore.project);
  const session = $derived(runtimeStore.session);
  const sessionActive = $derived(!!session && !session.stoppedAt);
  const gitInfo = $derived(runtimeStore.gitInfo);
  const appUpdate = $derived(runtimeStore.appUpdate);

  async function installUpdate() {
    try { await runtimeStore.installDownloadedUpdate(); } catch { /* handled by store */ }
  }

  function onDragStart(e: MouseEvent) {
    if ((e.target as HTMLElement).closest("[data-tauri-no-drag]")) return;
    void startWindowDrag();
  }

  function openConfigDialog() {
    document.dispatchEvent(new CustomEvent("trame:open-config-dialog"));
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header
  class="titlebar"
  data-tauri-drag-region
  style:padding-left={isMac ? "74px" : "16px"}
  style:padding-right={isMac ? "8px" : "0px"}
  onmousedown={onDragStart}
>
  <div class="titlebar-left" data-tauri-no-drag></div>

  <div class="titlebar-center" data-tauri-no-drag>
    <ProjectChip
      name={project?.name ?? "Trame"}
      baseDir={project?.baseDir ?? null}
      {gitInfo}
      {sessionActive}
      busy={runtimeStore.busy}
      projectId={runtimeStore.projectId}
      onSettings={openConfigDialog}
      onRun={() => void runtimeStore.startCurrentProject()}
      onStop={() => void runtimeStore.stopCurrentProject()}
    />
  </div>

  <div class="titlebar-right" data-tauri-no-drag>
    {#if !isDevMode && appUpdate?.status === "ready"}
      <button type="button" class="update-button" data-tauri-no-drag onclick={installUpdate}>
        Update app
      </button>
    {/if}
    {#if !isMac}
      <WindowControls />
    {/if}
  </div>
</header>

<style>
  .titlebar {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    width: 100%;
    height: 44px;
    min-height: 44px;
    background: transparent;
    user-select: none;
    -webkit-user-select: none;
    gap: 8px;
  }
  .titlebar-left { min-width: 0; }
  .titlebar-center {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-width: 0;
    overflow: hidden;
    padding-top: 6px;
  }
  .titlebar-right {
    display: flex;
    align-items: center;
    justify-self: end;
    gap: 0;
    flex-shrink: 0;
    height: 100%;
  }
  .update-button {
    display: inline-flex;
    align-items: center;
    height: 28px;
    padding: 0 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-accent);
    color: var(--color-canvas);
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
  }
  .update-button:hover { background: var(--color-accent-hover); }
  .update-button:disabled { opacity: 0.7; cursor: default; }
</style>
