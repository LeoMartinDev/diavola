import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";

vi.mock("$lib/tauri/nativeMenu", () => ({
  canShowNativeMenu: vi.fn(() => true),
  showNativeMenu: vi.fn(() => Promise.resolve()),
}));

import ProjectMenu from "./ProjectMenu.svelte";
import { canShowNativeMenu, showNativeMenu } from "$lib/tauri/nativeMenu";
import type { ProjectRecord } from "$lib/types";
import type { Selection } from "$lib/stores/runtime.svelte";

const project: ProjectRecord = {
  id: "p1",
  name: "Trame",
  baseDir: "/home/leo/trame",
  configSource: "projectFile",
  configPath: "/home/leo/dev/devapp/trame.yml",
  createdAt: "2026-01-01T00:00:00.000Z",
  updatedAt: "2026-01-01T00:00:00.000Z",
};

describe("ProjectMenu native popup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("uses the native menu path when available", async () => {
    const onOpenTerminal = vi.fn();
    const { getByRole, queryByRole } = render(ProjectMenu, {
      props: {
        project,
        selection: null satisfies Selection,
        selectedProcess: null,
        selectedTerminal: null,
        busy: false,
        logActions: null,
        onEditProject: vi.fn(),
        onOpenConfig: vi.fn(),
        onRestartProcess: vi.fn(),
        onStopProcess: vi.fn(),
        launchLocked: false,
        onCloseTerminal: vi.fn(),
        onOpenTerminal,
      },
    });

    const trigger = getByRole("button", { name: "Project menu" });
    await fireEvent.click(trigger, { clientX: 18, clientY: 28 });

    expect(canShowNativeMenu).toHaveBeenCalledOnce();
    expect(showNativeMenu).toHaveBeenCalledOnce();
    expect(showNativeMenu).toHaveBeenCalledWith(
      expect.arrayContaining([
        expect.objectContaining({ label: "Runtime config" }),
        expect.objectContaining({ label: "Open terminal" }),
      ]),
      expect.objectContaining({ x: 18, y: 28 }),
    );
    expect(showNativeMenu).not.toHaveBeenCalledWith(
      expect.arrayContaining([expect.objectContaining({ label: "Edit project" })]),
      expect.anything(),
    );
    expect(queryByRole("menuitem", { name: "Open terminal" })).toBeNull();
  });
});
