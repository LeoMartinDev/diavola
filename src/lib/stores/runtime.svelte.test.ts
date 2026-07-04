import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { toastStore } from "$lib/stores/toast.svelte";

const updaterMocks = vi.hoisted(() => ({
  checkForUpdate: vi.fn(),
  relaunchApp: vi.fn(),
}));

const clientMocks = vi.hoisted(() => ({
  getLaunchProject: vi.fn(),
  getSessionSnapshot: vi.fn(),
  hasActiveSessions: vi.fn(),
}));

const windowMocks = vi.hoisted(() => ({
  isPrimaryWindow: vi.fn(),
}));

vi.mock("$lib/tauri/client", async () => {
  const actual = await vi.importActual<typeof import("$lib/tauri/client")>("$lib/tauri/client");
  return {
    ...actual,
    getLaunchProject: clientMocks.getLaunchProject,
    getSessionSnapshot: clientMocks.getSessionSnapshot,
    hasActiveSessions: clientMocks.hasActiveSessions,
  };
});

vi.mock("$lib/tauri/updater", () => updaterMocks);

vi.mock("$lib/tauri/window", async () => {
  const actual = await vi.importActual<typeof import("$lib/tauri/window")>("$lib/tauri/window");
  return {
    ...actual,
    isPrimaryWindow: windowMocks.isPrimaryWindow,
  };
});

import { runtimeStore } from "./runtime.svelte";

describe("runtimeStore.init", () => {
  afterEach(async () => {
    await runtimeStore.teardown();
    vi.restoreAllMocks();
    vi.clearAllMocks();
  });

  beforeEach(() => {
    // Default to "primary window, no other active sessions" so existing
    // update-flow tests don't need to know about the new safety gates unless
    // they're specifically exercising them.
    windowMocks.isPrimaryWindow.mockReturnValue(true);
    clientMocks.hasActiveSessions.mockResolvedValue(false);
  });

  it("hydrates the current workspace project from launch info", async () => {
    clientMocks.getLaunchProject.mockResolvedValue({
      project: {
        id: "workspace-project",
        name: "demo-app",
        baseDir: "/tmp/demo-app",
        configPath: "/tmp/demo-app/diavola.yml",
        createdAt: "2026-07-03T00:00:00.000Z",
        updatedAt: "2026-07-03T00:00:00.000Z",
      },
      locked: true,
      autoRun: false,
      error: null,
    });
    clientMocks.getSessionSnapshot.mockResolvedValue({
      sessionId: "workspace-session",
      projectId: "workspace-project",
      projectName: "demo-app",
      baseDir: "/tmp/demo-app",
      startedAt: "2026-07-03T00:00:01.000Z",
      processes: [],
    });

    await runtimeStore.init();

    expect(runtimeStore.project?.baseDir).toBe("/tmp/demo-app");
    expect(runtimeStore.projectId).toBe("workspace-project");
  });

  it("does not auto-start the workspace when launch info says no config was found", async () => {
    clientMocks.getLaunchProject.mockResolvedValue({
      project: {
        id: "workspace-project",
        name: "demo-app",
        baseDir: "/tmp/demo-app",
        configPath: "/tmp/demo-app/diavola.yml",
        createdAt: "2026-07-03T00:00:00.000Z",
        updatedAt: "2026-07-03T00:00:00.000Z",
      },
      locked: true,
      autoRun: false,
      error: null,
    });
    clientMocks.getSessionSnapshot.mockResolvedValue(null);
    const startSpy = vi.spyOn(runtimeStore, "startCurrentProject");

    await runtimeStore.init();

    expect(startSpy).not.toHaveBeenCalled();
  });

  it("downloads an available update during init and exposes a ready state", async () => {
    const download = vi.fn(async (onEvent?: (event: unknown) => void) => {
      onEvent?.({ event: "Started", data: { contentLength: 5 } });
      onEvent?.({ event: "Progress", data: { chunkLength: 5 } });
      onEvent?.({ event: "Finished" });
    });

    updaterMocks.checkForUpdate.mockResolvedValue({
      currentVersion: "26.7.3-1",
      version: "26.7.3-2",
      body: "",
      date: "2026-07-03T12:00:00Z",
      download,
      install: vi.fn(async () => undefined),
      close: vi.fn(async () => undefined),
    });
    clientMocks.getLaunchProject.mockResolvedValue({
      project: {
        id: "workspace-project",
        name: "demo-app",
        baseDir: "/tmp/demo-app",
        configPath: "/tmp/demo-app/diavola.yml",
        createdAt: "2026-07-03T00:00:00.000Z",
        updatedAt: "2026-07-03T00:00:00.000Z",
      },
      locked: true,
      autoRun: false,
      error: null,
    });
    clientMocks.getSessionSnapshot.mockResolvedValue(null);

    await runtimeStore.init();

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(download).toHaveBeenCalledOnce();
    expect(runtimeStore.appUpdate).toEqual({
      status: "ready",
      version: "26.7.3-2",
    });
  });

  it("starts update checking in the background without blocking init", async () => {
    let resolveCheckForUpdate: ((value: unknown) => void) | undefined;
    let resolveDownload: (() => void) | undefined;
    const checkForUpdate = new Promise((resolve) => {
      resolveCheckForUpdate = resolve;
    });
    const download = vi.fn(async (onEvent?: (event: unknown) => void) => {
      onEvent?.({ event: "Started", data: { contentLength: 5 } });
      return await new Promise<void>((resolve) => {
        resolveDownload = resolve;
      });
    });

    updaterMocks.checkForUpdate.mockReturnValue(checkForUpdate);
    clientMocks.getLaunchProject.mockResolvedValue({
      project: {
        id: "workspace-project",
        name: "demo-app",
        baseDir: "/tmp/demo-app",
        configPath: "/tmp/demo-app/diavola.yml",
        createdAt: "2026-07-03T00:00:00.000Z",
        updatedAt: "2026-07-03T00:00:00.000Z",
      },
      locked: true,
      autoRun: false,
      error: null,
    });
    clientMocks.getSessionSnapshot.mockResolvedValue(null);

    const initPromise = runtimeStore.init();
    await expect(initPromise).resolves.toBeUndefined();

    resolveCheckForUpdate?.({
      currentVersion: "26.7.3-1",
      version: "26.7.3-2",
      body: "",
      date: "2026-07-03T12:00:00Z",
      download,
      install: vi.fn(async () => undefined),
      close: vi.fn(async () => undefined),
    });

    await Promise.resolve();
    await Promise.resolve();
    expect(download).toHaveBeenCalledOnce();
    expect(runtimeStore.appUpdate).toEqual({
      status: "downloading",
      version: "26.7.3-2",
      downloadedBytes: 0,
      contentLength: 5,
    });

    resolveDownload?.();
    await Promise.resolve();
    await Promise.resolve();

    expect(runtimeStore.appUpdate).toEqual({
      status: "ready",
      version: "26.7.3-2",
    });
  });

  it("keeps update download failures out of the UI", async () => {
    const toastSpy = vi.spyOn(toastStore, "show");
    updaterMocks.checkForUpdate.mockRejectedValueOnce(new Error("network down"));
    clientMocks.getLaunchProject.mockResolvedValue({
      project: {
        id: "workspace-project",
        name: "demo-app",
        baseDir: "/tmp/demo-app",
        configPath: "/tmp/demo-app/diavola.yml",
        createdAt: "2026-07-03T00:00:00.000Z",
        updatedAt: "2026-07-03T00:00:00.000Z",
      },
      locked: true,
      autoRun: false,
      error: null,
    });
    clientMocks.getSessionSnapshot.mockResolvedValue(null);

    await runtimeStore.init();
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(runtimeStore.appUpdate).toEqual({
      status: "error",
      message: "network down",
    });
    expect(toastSpy).not.toHaveBeenCalled();
  });

  it("installs the downloaded update and relaunches the app", async () => {
    const install = vi.fn(async () => undefined);
    updaterMocks.checkForUpdate.mockResolvedValue({
      currentVersion: "26.7.3-1",
      version: "26.7.3-2",
      body: "",
      date: "2026-07-03T12:00:00Z",
      download: vi.fn(async () => undefined),
      install,
      close: vi.fn(async () => undefined),
    });
    clientMocks.getLaunchProject.mockResolvedValue({
      project: {
        id: "workspace-project",
        name: "demo-app",
        baseDir: "/tmp/demo-app",
        configPath: "/tmp/demo-app/diavola.yml",
        createdAt: "2026-07-03T00:00:00.000Z",
        updatedAt: "2026-07-03T00:00:00.000Z",
      },
      locked: true,
      autoRun: false,
      error: null,
    });
    clientMocks.getSessionSnapshot.mockResolvedValue(null);

    await runtimeStore.init();
    await new Promise((resolve) => setTimeout(resolve, 0));
    await runtimeStore.installDownloadedUpdate();

    expect(install).toHaveBeenCalledOnce();
    expect(updaterMocks.relaunchApp).toHaveBeenCalledOnce();
    expect(runtimeStore.appUpdate).toEqual({
      status: "installing",
      version: "26.7.3-2",
    });
  });

  it("returns to ready after an install failure so the update can be retried", async () => {
    const toastSpy = vi.spyOn(toastStore, "show");
    const install = vi
      .fn<() => Promise<void>>()
      .mockRejectedValueOnce(new Error("disk full"))
      .mockResolvedValueOnce(undefined);
    updaterMocks.checkForUpdate.mockResolvedValue({
      currentVersion: "26.7.3-1",
      version: "26.7.3-2",
      body: "",
      date: "2026-07-03T12:00:00Z",
      download: vi.fn(async () => undefined),
      install,
      close: vi.fn(async () => undefined),
    });
    clientMocks.getLaunchProject.mockResolvedValue({
      project: {
        id: "workspace-project",
        name: "demo-app",
        baseDir: "/tmp/demo-app",
        configPath: "/tmp/demo-app/diavola.yml",
        createdAt: "2026-07-03T00:00:00.000Z",
        updatedAt: "2026-07-03T00:00:00.000Z",
      },
      locked: true,
      autoRun: false,
      error: null,
    });
    clientMocks.getSessionSnapshot.mockResolvedValue(null);

    await runtimeStore.init();
    await new Promise((resolve) => setTimeout(resolve, 0));

    await expect(runtimeStore.installDownloadedUpdate()).rejects.toThrow("disk full");
    expect(runtimeStore.appUpdate).toEqual({
      status: "ready",
      version: "26.7.3-2",
    });
    expect(toastSpy).not.toHaveBeenCalled();

    await runtimeStore.installDownloadedUpdate();

    expect(install).toHaveBeenCalledTimes(2);
    expect(updaterMocks.relaunchApp).toHaveBeenCalledOnce();
  });

  it("keeps update relaunch failures out of the UI", async () => {
    const toastSpy = vi.spyOn(toastStore, "show");
    const install = vi.fn(async () => undefined);
    updaterMocks.checkForUpdate.mockResolvedValue({
      currentVersion: "26.7.3-1",
      version: "26.7.3-2",
      body: "",
      date: "2026-07-03T12:00:00Z",
      download: vi.fn(async () => undefined),
      install,
      close: vi.fn(async () => undefined),
    });
    updaterMocks.relaunchApp.mockRejectedValueOnce(new Error("restart blocked"));
    clientMocks.getLaunchProject.mockResolvedValue({
      project: {
        id: "workspace-project",
        name: "demo-app",
        baseDir: "/tmp/demo-app",
        configPath: "/tmp/demo-app/diavola.yml",
        createdAt: "2026-07-03T00:00:00.000Z",
        updatedAt: "2026-07-03T00:00:00.000Z",
      },
      locked: true,
      autoRun: false,
      error: null,
    });
    clientMocks.getSessionSnapshot.mockResolvedValue(null);

    await runtimeStore.init();
    await new Promise((resolve) => setTimeout(resolve, 0));

    await expect(runtimeStore.installDownloadedUpdate()).rejects.toThrow("restart blocked");
    expect(runtimeStore.appUpdate).toEqual({
      status: "error",
      message: "restart blocked",
    });
    expect(toastSpy).not.toHaveBeenCalled();
  });

  it("ignores a re-entrant install call while one is already in flight", async () => {
    const install = vi.fn(async () => undefined);
    updaterMocks.checkForUpdate.mockResolvedValue({
      currentVersion: "26.7.3-1",
      version: "26.7.3-2",
      body: "",
      date: "2026-07-03T12:00:00Z",
      download: vi.fn(async () => undefined),
      install,
      close: vi.fn(async () => undefined),
    });
    clientMocks.getLaunchProject.mockResolvedValue({
      project: {
        id: "workspace-project",
        name: "demo-app",
        baseDir: "/tmp/demo-app",
        configPath: "/tmp/demo-app/diavola.yml",
        createdAt: "2026-07-03T00:00:00.000Z",
        updatedAt: "2026-07-03T00:00:00.000Z",
      },
      locked: true,
      autoRun: false,
      error: null,
    });
    clientMocks.getSessionSnapshot.mockResolvedValue(null);

    // Hold `hasActiveSessions` open so a second, re-entrant call to
    // `installDownloadedUpdate()` (simulating a fast double-click) arrives
    // while the first call is still awaiting it.
    let resolveActiveSessions: ((value: boolean) => void) | undefined;
    clientMocks.hasActiveSessions.mockImplementation(
      () =>
        new Promise<boolean>((resolve) => {
          resolveActiveSessions = resolve;
        }),
    );

    await runtimeStore.init();
    await new Promise((resolve) => setTimeout(resolve, 0));

    const firstCall = runtimeStore.installDownloadedUpdate();
    await Promise.resolve();
    const secondCall = runtimeStore.installDownloadedUpdate();

    expect(clientMocks.hasActiveSessions).toHaveBeenCalledOnce();

    resolveActiveSessions?.(false);
    await Promise.all([firstCall, secondCall]);

    expect(install).toHaveBeenCalledOnce();
    expect(updaterMocks.relaunchApp).toHaveBeenCalledOnce();
    expect(runtimeStore.appUpdate).toEqual({
      status: "installing",
      version: "26.7.3-2",
    });
  });

  it("refuses to install when a project session is active in any window", async () => {
    const toastSpy = vi.spyOn(toastStore, "show");
    const install = vi.fn(async () => undefined);
    updaterMocks.checkForUpdate.mockResolvedValue({
      currentVersion: "26.7.3-1",
      version: "26.7.3-2",
      body: "",
      date: "2026-07-03T12:00:00Z",
      download: vi.fn(async () => undefined),
      install,
      close: vi.fn(async () => undefined),
    });
    clientMocks.getLaunchProject.mockResolvedValue({
      project: {
        id: "workspace-project",
        name: "demo-app",
        baseDir: "/tmp/demo-app",
        configPath: "/tmp/demo-app/diavola.yml",
        createdAt: "2026-07-03T00:00:00.000Z",
        updatedAt: "2026-07-03T00:00:00.000Z",
      },
      locked: true,
      autoRun: false,
      error: null,
    });
    clientMocks.getSessionSnapshot.mockResolvedValue(null);
    clientMocks.hasActiveSessions.mockResolvedValue(true);

    await runtimeStore.init();
    await new Promise((resolve) => setTimeout(resolve, 0));
    await runtimeStore.installDownloadedUpdate();

    expect(install).not.toHaveBeenCalled();
    expect(updaterMocks.relaunchApp).not.toHaveBeenCalled();
    expect(runtimeStore.appUpdate).toEqual({
      status: "ready",
      version: "26.7.3-2",
    });
    expect(toastSpy).not.toHaveBeenCalled();
  });

  it("does not check for updates in a non-primary window", async () => {
    windowMocks.isPrimaryWindow.mockReturnValue(false);
    clientMocks.getLaunchProject.mockResolvedValue({
      project: {
        id: "workspace-project",
        name: "demo-app",
        baseDir: "/tmp/demo-app",
        configPath: "/tmp/demo-app/diavola.yml",
        createdAt: "2026-07-03T00:00:00.000Z",
        updatedAt: "2026-07-03T00:00:00.000Z",
      },
      locked: true,
      autoRun: false,
      error: null,
    });
    clientMocks.getSessionSnapshot.mockResolvedValue(null);

    await runtimeStore.init();
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(updaterMocks.checkForUpdate).not.toHaveBeenCalled();
    expect(runtimeStore.appUpdate).toEqual({ status: "idle" });
  });
});
