import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/svelte";

vi.mock("$lib/tauri/window", async () => {
  const actual = await vi.importActual<typeof import("$lib/tauri/window")>("$lib/tauri/window");
  return {
    ...actual,
    startWindowDrag: vi.fn(async () => undefined),
  };
});

import TitleBar from "./TitleBar.svelte";
import { runtimeStore } from "$lib/stores/runtime.svelte";

const project = {
  id: "workspace-project",
  name: "demo-app",
  baseDir: "/tmp/demo-app",
  configSource: "projectFile" as const,
  configPath: "/tmp/demo-app/diavola.yml",
  createdAt: "2026-07-03T00:00:00.000Z",
  updatedAt: "2026-07-03T00:00:00.000Z",
};

describe("TitleBar", () => {
  beforeEach(() => {
    runtimeStore.projects = [project];
    runtimeStore.projectId = project.id;
    runtimeStore.launchLocked = false;
    runtimeStore.session = null;
    runtimeStore.appUpdate = null;
  });

  afterEach(() => {
    runtimeStore.projects = [];
    runtimeStore.projectId = null;
    runtimeStore.launchLocked = false;
    runtimeStore.session = null;
    runtimeStore.appUpdate = null;
    vi.restoreAllMocks();
  });

  it("does not render a register-project button", () => {
    const { queryByRole } = render(TitleBar);

    expect(queryByRole("button", { name: "Register project" })).toBeNull();
  });

  it("shows an Update app button when an update is ready", () => {
    runtimeStore.appUpdate = { status: "ready", version: "26.7.3-2" } as any;

    const { getByRole } = render(TitleBar);

    expect(getByRole("button", { name: "Update app" })).toBeInTheDocument();
  });

});
