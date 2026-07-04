import { afterEach, describe, expect, it, vi } from "vitest";

const coreMock = vi.hoisted(() => ({
  isTauri: vi.fn(() => false),
}));

vi.mock("@tauri-apps/api/core", () => coreMock);

const windowApi = vi.hoisted(() => ({
  getCurrentWindow: vi.fn(() => ({
    label: "main",
    setTitle: vi.fn(),
    startDragging: vi.fn(),
    minimize: vi.fn(),
    toggleMaximize: vi.fn(),
    close: vi.fn(),
    isMaximized: vi.fn(async () => false),
  })),
}));

vi.mock("@tauri-apps/api/window", () => windowApi);

import {
  canUseTauriWindow,
  closeWindow,
  isPrimaryWindow,
  isWindowMaximized,
  minimizeWindow,
  setWindowTitle,
  startWindowDrag,
  toggleWindowMaximize,
} from "./window";

describe("window seam", () => {
  it("no-ops safely in browser mode", async () => {
    expect(canUseTauriWindow()).toBe(false);

    await expect(setWindowTitle("Trame")).resolves.toBeUndefined();
    await expect(startWindowDrag()).resolves.toBeUndefined();
    await expect(minimizeWindow()).resolves.toBeUndefined();
    await expect(toggleWindowMaximize()).resolves.toBeUndefined();
    await expect(closeWindow()).resolves.toBeUndefined();
    await expect(isWindowMaximized()).resolves.toBe(false);

    expect(windowApi.getCurrentWindow).not.toHaveBeenCalled();
  });
});

describe("isPrimaryWindow", () => {
  afterEach(() => {
    coreMock.isTauri.mockReturnValue(false);
    windowApi.getCurrentWindow.mockReturnValue({
      label: "main",
      setTitle: vi.fn(),
      startDragging: vi.fn(),
      minimize: vi.fn(),
      toggleMaximize: vi.fn(),
      close: vi.fn(),
      isMaximized: vi.fn(async () => false),
    });
  });

  it("returns true for the main window under the Tauri runtime", () => {
    coreMock.isTauri.mockReturnValue(true);
    windowApi.getCurrentWindow.mockReturnValue({
      label: "main",
    });

    expect(isPrimaryWindow()).toBe(true);
  });

  it("returns false for a non-main window under the Tauri runtime", () => {
    coreMock.isTauri.mockReturnValue(true);
    windowApi.getCurrentWindow.mockReturnValue({
      label: "project-123",
    });

    expect(isPrimaryWindow()).toBe(false);
  });

  it("returns true outside the Tauri runtime regardless of window label", () => {
    coreMock.isTauri.mockReturnValue(false);
    windowApi.getCurrentWindow.mockReturnValue({
      label: "project-123",
    });
    windowApi.getCurrentWindow.mockClear();

    expect(isPrimaryWindow()).toBe(true);
    expect(windowApi.getCurrentWindow).not.toHaveBeenCalled();
  });
});
