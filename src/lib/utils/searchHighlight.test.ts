import { describe, it, expect } from "vitest";
import {
  buildMatcher,
  escapeRegExp,
  highlightLine,
  lineMatches,
} from "./searchHighlight";

const opts = (regex = false, caseSensitive = false) => ({ regex, caseSensitive });

describe("escapeRegExp", () => {
  it("escapes regex metacharacters", () => {
    expect(escapeRegExp("a.b*c")).toBe("a\\.b\\*c");
  });
});

describe("buildMatcher", () => {
  it("returns null for an empty query", () => {
    expect(buildMatcher("", opts())).toBeNull();
    expect(buildMatcher("   ", opts())).toBeNull();
  });

  it("builds an escaped substring matcher (case-insensitive by default)", () => {
    const m = buildMatcher("a.b", opts());
    if (m === null || "error" in m) throw new Error("expected a valid matcher");
    // "." must NOT act as a wildcard in substring mode
    expect(lineMatches(m, "axb")).toBe(false);
    expect(lineMatches(m, "a.b here")).toBe(true);
  });

  it("treats the query as a raw regex in regex mode", () => {
    const m = buildMatcher("a.b", opts(true));
    if (m === null || "error" in m) throw new Error("expected a valid matcher");
    expect(lineMatches(m, "axb")).toBe(true);
    expect(lineMatches(m, "a.b")).toBe(true);
  });

  it("honors case-sensitive mode", () => {
    const ci = buildMatcher("Error", opts(false, false));
    const cs = buildMatcher("Error", opts(false, true));
    if (ci === null || "error" in ci) throw new Error("expected ci matcher");
    if (cs === null || "error" in cs) throw new Error("expected cs matcher");
    expect(lineMatches(ci, "error")).toBe(true);
    expect(lineMatches(cs, "error")).toBe(false);
  });

  it("returns { error } for an invalid regex", () => {
    const m = buildMatcher("(unclosed", opts(true));
    if (m === null || "regex" in m) throw new Error("expected an error matcher");
    expect(typeof m.error).toBe("string");
    expect(m.error.length).toBeGreaterThan(0);
  });
});

describe("lineMatches", () => {
  it("returns false for null and error matchers", () => {
    expect(lineMatches(null, "anything")).toBe(false);
    expect(lineMatches({ error: "bad" }, "anything")).toBe(false);
  });
});

describe("highlightLine", () => {
  it("returns a single non-match segment for null/error matchers", () => {
    expect(highlightLine("hello", null)).toEqual([{ text: "hello", match: false }]);
    expect(highlightLine("hello", { error: "bad" })).toEqual([
      { text: "hello", match: false },
    ]);
  });

  it("highlights all case-insensitive substring occurrences", () => {
    const m = buildMatcher("a", opts());
    if (m === null || "error" in m) throw new Error("expected matcher");
    expect(highlightLine("banana", m)).toEqual([
      { text: "b", match: false },
      { text: "a", match: true },
      { text: "n", match: false },
      { text: "a", match: true },
      { text: "n", match: false },
      { text: "a", match: true },
    ]);
  });

  it("highlights regex matches", () => {
    const m = buildMatcher("\\d+", opts(true));
    if (m === null || "error" in m) throw new Error("expected matcher");
    expect(highlightLine("a1b22c", m)).toEqual([
      { text: "a", match: false },
      { text: "1", match: true },
      { text: "b", match: false },
      { text: "22", match: true },
      { text: "c", match: false },
    ]);
  });

  it("does not infinite-loop on zero-width regex matches", () => {
    const m = buildMatcher("a*", opts(true));
    if (m === null || "error" in m) throw new Error("expected matcher");
    const segs = highlightLine("baaab", m);
    // Concatenated text must equal the original (no chars lost/duplicated).
    expect(segs.map((s) => s.text).join("")).toBe("baaab");
  });
});
