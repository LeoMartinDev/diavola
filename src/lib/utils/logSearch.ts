import type { FlatRow } from "$lib/types";
import { stripAnsi } from "$lib/utils/ansi";
import {
  buildMatcher,
  lineMatches,
  type Matcher,
  type SearchOptions,
} from "$lib/utils/searchHighlight";
import { isTauriRuntime } from "$lib/tauri/environment";
import { searchProcessLogs } from "$lib/tauri/client";

export type ComputeMatchArgs = {
  logs: FlatRow[];
  matcher: Matcher;
  query: string;
  options: SearchOptions;
  runtimeId: string | null;
  paused: boolean;
};

export function searchLogsLocally(logs: FlatRow[], matcher: Matcher): number[] {
  if (matcher === null || "error" in matcher) return [];
  const indices: number[] = [];
  for (let i = 0; i < logs.length; i++) {
    const row = logs[i];
    if (lineMatches(matcher, `${row.stream} ${stripAnsi(row.text)}`)) {
      indices.push(i);
    }
  }
  return indices;
}

export async function computeMatchIndices(args: ComputeMatchArgs): Promise<number[]> {
  const { logs, matcher, query, options, runtimeId, paused } = args;
  if (matcher === null || "error" in matcher) return [];
  if (paused || !isTauriRuntime() || runtimeId === null) {
    return searchLogsLocally(logs, matcher);
  }
  try {
    const reply = await searchProcessLogs({
      runtimeId,
      query,
      regex: options.regex,
      caseSensitive: options.caseSensitive,
      upTo: logs.length,
    });
    if (reply.matchIndices.length === 0) {
      return searchLogsLocally(logs, matcher);
    }
    return reply.matchIndices;
  } catch {
    return searchLogsLocally(logs, matcher);
  }
}

export { buildMatcher };
