import { describe, expect, it } from "vitest";

import { computeVirtualScroll } from "./virtualScroll";

describe("computeVirtualScroll", () => {
  it("clamps stale scrollTop when the list becomes shorter", () => {
    const result = computeVirtualScroll(220_000, 500, 1_000);

    expect(result.totalHeight).toBe(22_000);
    expect(result.startIndex).toBeLessThan(result.endIndex);
    expect(result.endIndex).toBe(1_000);
  });

  it("returns an empty range for an empty list", () => {
    const result = computeVirtualScroll(10_000, 500, 0);

    expect(result.totalHeight).toBe(0);
    expect(result.startIndex).toBe(0);
    expect(result.endIndex).toBe(0);
  });
});
