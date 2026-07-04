import { describe, expect, it, vi } from "vitest";

const coreMock = vi.hoisted(() => ({
  isTauri: vi.fn(() => false),
}));

vi.mock("@tauri-apps/api/core", () => coreMock);

import { isTauriRuntime } from "./environment";

describe("isTauriRuntime", () => {
  it("returns false in the browser without tauri internals", () => {
    expect(isTauriRuntime()).toBe(false);
  });

  it("returns true when tauri internals are present", () => {
    coreMock.isTauri.mockReturnValue(true);
    expect(isTauriRuntime()).toBe(true);
    coreMock.isTauri.mockReturnValue(false);
  });
});
