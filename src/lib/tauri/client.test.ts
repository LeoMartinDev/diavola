import { describe, expect, it, vi } from "vitest";

const windowHelpers = vi.hoisted(() => ({
  setWindowTitle: vi.fn(async () => undefined),
}));

vi.mock("./window", () => windowHelpers);

import { setWindowTitle } from "./client";

describe("client window helper", () => {
  it("routes title updates through the shared window seam", async () => {
    await expect(setWindowTitle("Diavola")).resolves.toBeUndefined();
    expect(windowHelpers.setWindowTitle).toHaveBeenCalledOnce();
    expect(windowHelpers.setWindowTitle).toHaveBeenCalledWith("Diavola");
  });
});
