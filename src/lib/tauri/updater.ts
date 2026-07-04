import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

import { isDev, isTauriRuntime } from "./environment";

export type UpdaterDownloadEvent = {
  event: "Started" | "Progress" | "Finished";
  data: {
    contentLength?: number | null;
    chunkLength?: number;
  };
};

export type PendingAppUpdate = {
  currentVersion: string;
  version: string;
  body: string;
  date: string;
  download: (onEvent?: (event: UpdaterDownloadEvent) => void) => Promise<void>;
  install: () => Promise<void>;
  close: () => Promise<void>;
};

export async function checkForUpdate(): Promise<PendingAppUpdate | null> {
  if (!isTauriRuntime() || isDev()) {
    return null;
  }

  return (await check()) as PendingAppUpdate | null;
}

export async function relaunchApp(): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }

  await relaunch();
}
