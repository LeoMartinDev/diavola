import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";

import ProjectMenu from "./ProjectMenu.svelte";
import type { ProcessSnapshot, ProjectRecord, TerminalSnapshot } from "$lib/types";
import type { Selection } from "$lib/stores/runtime.svelte";

const project: ProjectRecord = {
  id: "p1",
  name: "Diavola",
  baseDir: "/home/leo/diavola",
  configSource: "projectFile",
  configPath: "/home/leo/diavola/diavola.yml",
  createdAt: "2026-01-01T00:00:00.000Z",
  updatedAt: "2026-01-01T00:00:00.000Z",
};

const process: ProcessSnapshot = {
  runtimeId: "r1",
  name: "api",
  kind: "service",
  status: "running",
};

const terminal: TerminalSnapshot = {
  terminalId: "t1",
  title: "bash",
  cwd: "/home/leo/diavola",
  createdAt: "2026-01-01T00:00:00.000Z",
  isOpen: true,
};

function makeProps(overrides: Partial<Parameters<typeof render<ProjectMenu>>[1]> = {}) {
  // Plain object of props; callers can override individual fields.
  const defaultProps = {
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
    onCloseTerminal: vi.fn(),
    onOpenTerminal: vi.fn(),
  };
  // @ts-expect-error: merge overrides loosely; tests are responsible for shape.
  return { props: { ...defaultProps, ...overrides.props } };
}

async function openMenu(getByRole: (role: string, options?: { name?: string }) => HTMLElement) {
  await fireEvent.click(getByRole("button", { name: "Project menu" }));
}

describe("ProjectMenu", () => {
  describe("with a registered project", () => {
    it("exposes Runtime config without any project-edit action", async () => {
      const { getByRole, queryByRole } = render(ProjectMenu, makeProps());

      await openMenu(getByRole);

      expect(getByRole("menuitem", { name: "Runtime config" })).toBeInTheDocument();
      expect(queryByRole("menuitem", { name: "Edit project" })).toBeNull();
    });

    it("always exposes Open terminal", async () => {
      const { getByRole } = render(ProjectMenu, makeProps());

      await openMenu(getByRole);

      expect(getByRole("menuitem", { name: "Open terminal" })).toBeInTheDocument();
    });

    it("does not show process-specific actions when nothing is selected", async () => {
      const { getByRole, queryByRole } = render(ProjectMenu, makeProps());

      await openMenu(getByRole);

      expect(queryByRole("menuitem", { name: /Restart/ })).toBeNull();
      expect(queryByRole("menuitem", { name: /Stop/ })).toBeNull();
      expect(queryByRole("menuitem", { name: "Copy logs" })).toBeNull();
    });

    it("calls onOpenConfig when Runtime config is clicked", async () => {
      const onOpenConfig = vi.fn();
      const { getByRole } = render(ProjectMenu, makeProps({ props: { onOpenConfig } }));

      await openMenu(getByRole);
      await fireEvent.click(getByRole("menuitem", { name: "Runtime config" }));

      expect(onOpenConfig).toHaveBeenCalledOnce();
    });
  });

  describe("with a process selected", () => {
    const processSelection: Selection = { kind: "process", runtimeId: "r1" };

    it("exposes Restart and Stop named after the process", async () => {
      const { getByRole } = render(
        ProjectMenu,
        makeProps({ props: { selection: processSelection, selectedProcess: process } }),
      );

      await openMenu(getByRole);

      expect(getByRole("menuitem", { name: "Restart api" })).toBeInTheDocument();
      expect(getByRole("menuitem", { name: "Stop api" })).toBeInTheDocument();
    });

    it("exposes Copy logs and Clear logs only once log actions are registered", async () => {
      const { getByRole, rerender } = render(
        ProjectMenu,
        makeProps({ props: { selection: processSelection, selectedProcess: process } }),
      );
      await openMenu(getByRole);

      // Disabled until logActions are provided (LogViewer not mounted yet).
      expect(getByRole("menuitem", { name: "Copy logs" })).toBeDisabled();
      expect(getByRole("menuitem", { name: "Clear logs" })).toBeDisabled();

      rerender(
        // @ts-expect-error: partial props merge for rerender
        makeProps({
          props: {
            selection: processSelection,
            selectedProcess: process,
            logActions: { copy: vi.fn(), clear: vi.fn() },
          },
        }),
      );

      expect(getByRole("menuitem", { name: "Copy logs" })).toBeEnabled();
      expect(getByRole("menuitem", { name: "Clear logs" })).toBeEnabled();
    });

    it("calls onRestartProcess with the process name", async () => {
      const onRestartProcess = vi.fn();
      const { getByRole } = render(
        ProjectMenu,
        makeProps({
          props: { selection: processSelection, selectedProcess: process, onRestartProcess },
        }),
      );

      await openMenu(getByRole);
      await fireEvent.click(getByRole("menuitem", { name: "Restart api" }));

      expect(onRestartProcess).toHaveBeenCalledWith("api");
    });

    it("calls onStopProcess with the process name", async () => {
      const onStopProcess = vi.fn();
      const { getByRole } = render(
        ProjectMenu,
        makeProps({
          props: { selection: processSelection, selectedProcess: process, onStopProcess },
        }),
      );

      await openMenu(getByRole);
      await fireEvent.click(getByRole("menuitem", { name: "Stop api" }));

      expect(onStopProcess).toHaveBeenCalledWith("api");
    });

    it("invokes the registered copy action via Copy logs", async () => {
      const copy = vi.fn();
      const { getByRole } = render(
        ProjectMenu,
        makeProps({
          props: {
            selection: processSelection,
            selectedProcess: process,
            logActions: { copy, clear: vi.fn() },
          },
        }),
      );

      await openMenu(getByRole);
      await fireEvent.click(getByRole("menuitem", { name: "Copy logs" }));

      expect(copy).toHaveBeenCalledOnce();
    });
  });

  describe("with a terminal selected", () => {
    const terminalSelection: Selection = { kind: "terminal", terminalId: "t1" };

    it("exposes Close named after the terminal", async () => {
      const { getByRole } = render(
        ProjectMenu,
        makeProps({ props: { selection: terminalSelection, selectedTerminal: terminal } }),
      );

      await openMenu(getByRole);

      expect(getByRole("menuitem", { name: "Close bash" })).toBeInTheDocument();
    });

    it("does not show process-specific actions", async () => {
      const { queryByRole, getByRole } = render(
        ProjectMenu,
        makeProps({ props: { selection: terminalSelection, selectedTerminal: terminal } }),
      );

      await openMenu(getByRole);

      expect(queryByRole("menuitem", { name: /Restart/ })).toBeNull();
      expect(queryByRole("menuitem", { name: /Copy logs/ })).toBeNull();
    });

    it("calls onCloseTerminal when Close is clicked", async () => {
      const onCloseTerminal = vi.fn();
      const { getByRole } = render(
        ProjectMenu,
        makeProps({
          props: { selection: terminalSelection, selectedTerminal: terminal, onCloseTerminal },
        }),
      );

      await openMenu(getByRole);
      await fireEvent.click(getByRole("menuitem", { name: "Close bash" }));

      expect(onCloseTerminal).toHaveBeenCalledOnce();
    });
  });

  describe("Open terminal", () => {
    it("is disabled when busy", async () => {
      const { getByRole } = render(ProjectMenu, makeProps({ props: { busy: true } }));

      await openMenu(getByRole);

      expect(getByRole("menuitem", { name: "Open terminal" })).toBeDisabled();
    });

    it("calls onOpenTerminal when enabled and clicked", async () => {
      const onOpenTerminal = vi.fn();
      const { getByRole } = render(ProjectMenu, makeProps({ props: { onOpenTerminal } }));

      await openMenu(getByRole);
      await fireEvent.click(getByRole("menuitem", { name: "Open terminal" }));

      expect(onOpenTerminal).toHaveBeenCalledOnce();
    });
  });
});
