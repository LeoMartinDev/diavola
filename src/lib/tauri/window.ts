import { getCurrentWindow } from "@tauri-apps/api/window";

import { isTauriRuntime } from "./environment";

export function canUseTauriWindow(): boolean {
  return isTauriRuntime();
}

// Tauri's default (unlabeled) window is created with the label "main"; every
// other window Diavola opens (e.g. `project-<id>` per-project windows) has a
// distinct label. We designate "main" the sole owner of update
// checking/downloading so multiple open windows don't each independently
// check for and download the same update. Outside Tauri (browser/dev-server)
// there is only ever one window, so it is always treated as primary.
export function isPrimaryWindow(): boolean {
  if (!canUseTauriWindow()) return true;
  return getCurrentWindow().label === "main";
}

export async function setWindowTitle(title: string): Promise<void> {
  if (!canUseTauriWindow()) return;
  await getCurrentWindow().setTitle(title);
}

export async function startWindowDrag(): Promise<void> {
  if (!canUseTauriWindow()) return;
  await getCurrentWindow().startDragging();
}

export async function minimizeWindow(): Promise<void> {
  if (!canUseTauriWindow()) return;
  await getCurrentWindow().minimize();
}

export async function toggleWindowMaximize(): Promise<void> {
  if (!canUseTauriWindow()) return;
  await getCurrentWindow().toggleMaximize();
}

export async function closeWindow(): Promise<void> {
  if (!canUseTauriWindow()) return;
  await getCurrentWindow().close();
}

export async function isWindowMaximized(): Promise<boolean> {
  if (!canUseTauriWindow()) return false;
  return getCurrentWindow().isMaximized();
}
