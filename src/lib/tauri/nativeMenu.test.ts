import { describe, expect, it } from "vitest";

import { canShowNativeMenu } from "./nativeMenu";

describe("native menu capability", () => {
  it("matches browser capability detection", () => {
    expect(canShowNativeMenu()).toBe(false);
  });
});
