import type { ProcessSnapshot } from "$lib/types";

export function formatProcessDuration(process: ProcessSnapshot): string {
  const { startedAt, exitedAt, kind, status } = process;

  if (!startedAt) return "";
  if (status === "pending" || status === "blocked") return "";

  const start = new Date(startedAt).getTime();
  const end = exitedAt ? new Date(exitedAt).getTime() : Date.now();
  let diffMs = end - start;

  if (diffMs < 0) return "";

  const isCompleted =
    status === "succeeded" || status === "failed" || status === "stopped";
  const prefix = kind === "task" && isCompleted ? "took " : "";

  const totalSeconds = diffMs / 1000;
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const secs = totalSeconds % 60;

  if (hours > 0) {
    const m = Math.round(minutes);
    return `${prefix}${hours}h ${m}m`;
  }

  if (minutes > 0) {
    const s = Math.floor(secs);
    if (s === 0) return `${prefix}${minutes}m`;
    return `${prefix}${minutes}m ${s}s`;
  }

  if (kind === "task" && isCompleted && secs < 10) {
    return `${prefix}${secs.toFixed(1)}s`;
  }

  return `${prefix}${Math.floor(secs)}s`;
}
