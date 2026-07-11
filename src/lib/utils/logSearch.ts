import type { FlatRow } from "$lib/types";
import { stripAnsi } from "$lib/utils/ansi";
import {
  buildMatcher,
  lineMatches,
  type Matcher,
} from "$lib/utils/searchHighlight";

export type ComputeMatchArgs = {
  logs: FlatRow[];
  matcher: Matcher;
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

export function computeMatchIndices(args: ComputeMatchArgs): number[] {
  const { logs, matcher } = args;
  if (matcher === null || "error" in matcher) return [];
  return searchLogsLocally(logs, matcher);
}

export { buildMatcher };
