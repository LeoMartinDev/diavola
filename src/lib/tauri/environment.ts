import { isTauri } from "@tauri-apps/api/core";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && isTauri();
}

export function isDev(): boolean {
  return import.meta.env.DEV;
}
