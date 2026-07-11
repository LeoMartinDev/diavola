import { describe, it, expect } from "vitest";
import { searchLogsLocally, computeMatchIndices } from "./logSearch";
import { buildMatcher, type SearchOptions } from "./searchHighlight";
import type { FlatRow } from "$lib/types";

const opts: SearchOptions = { regex: false, caseSensitive: false };

function row(text: string, i: number, stream: FlatRow["stream"] = "stdout"): FlatRow {
  return {
    entryId: i,
    lineIndex: 0,
    isFirstLine: true,
    isContinuation: false,
    text,
    stream,
    timestamp: `2026-01-01T00:00:${i.toString().padStart(2, "0")}Z`,
  };
}

describe("searchLogsLocally", () => {
  it("returns row positions matching the query (case-insensitive)", () => {
    const logs = [row("Listening on 3000", 0), row("worker ready", 1), row("Listening on 3001", 2)];
    const m = buildMatcher("listening", opts);
    expect(searchLogsLocally(logs, m)).toEqual([0, 2]);
  });

  it("matches the stream-prefixed target", () => {
    const logs = [row("boom", 0, "stderr")];
    const m = buildMatcher("stderr", opts);
    expect(searchLogsLocally(logs, m)).toEqual([0]);
  });

  it("strips ANSI before matching", () => {
    const logs = [row("\x1b[32mready\x1b[39m", 0)];
    const m = buildMatcher("ready", opts);
    expect(searchLogsLocally(logs, m)).toEqual([0]);
  });

  it("returns [] for an empty matcher", () => {
    expect(searchLogsLocally([row("x", 0)], buildMatcher("", opts))).toEqual([]);
  });
});

describe("computeMatchIndices", () => {
  it("returns row positions matching the query", () => {
    const logs = [row("hit", 0), row("miss", 1), row("hit", 2)];
    const m = buildMatcher("hit", opts);
    const indices = computeMatchIndices({ logs, matcher: m });
    expect(indices).toEqual([0, 2]);
  });

  it("returns [] for an invalid matcher (regex error)", () => {
    const m = buildMatcher("(unclosed", { regex: true, caseSensitive: false });
    const indices = computeMatchIndices({ logs: [row("x", 0)], matcher: m });
    expect(indices).toEqual([]);
  });
});
