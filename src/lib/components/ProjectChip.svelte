<script lang="ts">
  import RunStopButton from "$lib/components/RunStopButton.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import type { GitInfo } from "$lib/types";

  type Props = {
    name: string;
    baseDir: string | null;
    gitInfo: GitInfo | null;
    sessionActive: boolean;
    busy: boolean;
    projectId: string | null;
    onSettings: () => void;
    onRun: () => void;
    onStop: () => void;
  };

  let {
    name,
    baseDir,
    gitInfo,
    sessionActive,
    busy,
    projectId,
    onSettings,
    onRun,
    onStop,
  }: Props = $props();
</script>

<div class="project-chip">
  <div class="project-chip-settings-sep">
    <button
      type="button"
      class="project-chip-settings"
      aria-label="Project settings"
      title="Project settings"
      onclick={onSettings}
    >
      <Icon name="settings" size="sm" />
    </button>
  </div>
  <span class="project-chip-name">{name}</span>
  {#if baseDir}
    <span class="project-chip-dir-wrap">
      <span class="project-chip-dir" title={baseDir}>{baseDir}</span>
    </span>
  {/if}
  {#if gitInfo?.branch || gitInfo?.worktree}
    <span class="project-chip-git">
      <Icon name="git-branch" size="xs" />
      <span class="project-chip-git-label">{gitInfo.worktree ?? gitInfo.branch}</span>
    </span>
  {/if}
  <span class="project-chip-action">
    <RunStopButton
      active={sessionActive}
      {busy}
      disabled={!projectId}
      compact
      {onRun}
      {onStop}
    />
  </span>
</div>

<style>
  .project-chip {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    align-self: center;
    column-gap: 5px;
    position: relative;
    width: min(360px, calc(100vw - 240px));
    max-width: 360px;
    height: 36px;
    padding: 0 5px 0 36px;
    border-radius: var(--radius-sm);
    background: var(--color-surface, #0e0f12);
    border: 1px solid var(--color-border, #ffffff14);
    min-width: 0;
    line-height: 1;
    transform: translateY(-1px);
  }
  .project-chip-name {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text, #e7e9ee);
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding-left: 6px;
  }
  .project-chip-settings-sep {
    position: absolute;
    left: 5px;
    top: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 27px;
    padding-right: 4px;
    border-right: 1px solid var(--color-border, #ffffff14);
    z-index: 2;
    box-sizing: border-box;
  }
  .project-chip-settings {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    margin: 0;
    border: none;
    background: none;
    border-radius: var(--radius-sm);
    color: var(--color-text-muted, #9aa0aa);
    cursor: pointer;
    transition: background 75ms, color 75ms;
  }
  .project-chip-settings:hover {
    background: var(--color-surface-hover, #1b1d22);
    color: var(--color-text, #e7e9ee);
  }
  .project-chip-dir-wrap {
    display: flex;
    justify-content: center;
    position: absolute;
    left: 50%;
    top: 50%;
    z-index: 0;
    width: min(10.5rem, calc(100% - 12.5rem));
    min-width: 6rem;
    overflow: hidden;
    transform: translate(-50%, -50%);
    pointer-events: none;
  }
  .project-chip-dir {
    display: block;
    min-width: 0;
    width: 100%;
    max-width: 100%;
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 500;
    color: var(--color-text-muted, #9aa0aa);
    letter-spacing: 0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: center;
    unicode-bidi: plaintext;
  }
  .project-chip-git {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    min-width: 0;
    max-width: 6.5rem;
    padding: 1px 4px;
    border-radius: 3px;
    background: var(--color-surface-hover, #1b1d22);
    font-size: 10px;
    color: var(--color-text-muted, #9aa0aa);
    flex-shrink: 0;
    z-index: 1;
  }
  .project-chip-git-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .project-chip-action {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding-left: 4px;
    border-left: 1px solid var(--color-border, #ffffff14);
    z-index: 1;
  }
  @media (max-width: 500px) {
    .project-chip-dir { display: none; }
  }
  @media (max-width: 400px) {
    .project-chip-git { display: none; }
  }
</style>
