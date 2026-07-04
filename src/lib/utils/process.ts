import type { ProcessSnapshot, ProcessStatus } from "$lib/types";

export type RowAction = "stop" | "start" | null;

export const STATUS_COLOR: Record<ProcessStatus, string> = {
  pending: "text-text-subtle",
  blocked: "text-warning",
  starting: "text-accent",
  running: "text-accent",
  ready: "text-success",
  succeeded: "text-success",
  failed: "text-danger",
  stopping: "text-warning",
  stopped: "text-text-subtle",
};

export function processRowAction(process: ProcessSnapshot): RowAction {
  if (process.kind === "task") return null;
  switch (process.status) {
    case "running":
    case "ready":
    case "starting":
      return "stop";
    case "stopped":
    case "failed":
    case "succeeded":
      return "start";
    default:
      return null;
  }
}

export function processActionEnabled(process: ProcessSnapshot, busy: boolean): boolean {
  return !busy && processRowAction(process) !== null;
}
