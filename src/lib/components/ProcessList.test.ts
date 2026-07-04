import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";

import ProcessList from "./ProcessList.svelte";
import type { ProcessSnapshot, TerminalSnapshot } from "$lib/types";

function makeProcess(overrides: Partial<ProcessSnapshot> = {}): ProcessSnapshot {
  return {
    runtimeId: "r1",
    name: "api",
    kind: "service",
    status: "running",
    ...overrides,
  };
}

const terminal: TerminalSnapshot = {
  terminalId: "t1",
  title: "bash",
  cwd: "/home/leo/trame",
  createdAt: "2026-01-01T00:00:00.000Z",
  isOpen: true,
};

function makeProps(overrides: Record<string, unknown> = {}) {
  return {
    processes: [] as ProcessSnapshot[],
    terminals: [] as TerminalSnapshot[],
    selectedProcessRuntimeId: null,
    selectedTerminalId: null,
    busy: false,
    onSelectProcess: vi.fn(),
    onSelectTerminal: vi.fn(),
    onStop: vi.fn(),
    onStart: vi.fn(),
    onRestart: vi.fn(),
    onCloseTerminal: vi.fn(),
    ...overrides,
  };
}

describe("ProcessList action button", () => {
  describe("when the process is running", () => {
    it("renders a Stop button", () => {
      const { getByRole, queryByRole } = render(ProcessList, {
        props: makeProps({ processes: [makeProcess({ status: "running" })] }),
      });

      expect(getByRole("button", { name: "Stop api" })).toBeInTheDocument();
      expect(queryByRole("button", { name: "Start api" })).toBeNull();
    });

    it("calls onStop when clicked", async () => {
      const onStop = vi.fn();
      const { getByRole } = render(ProcessList, {
        props: makeProps({
          processes: [makeProcess({ status: "running" })],
          onStop,
        }),
      });

      await fireEvent.click(getByRole("button", { name: "Stop api" }));

      expect(onStop).toHaveBeenCalledOnce();
      expect(onStop).toHaveBeenCalledWith("api");
    });

    it("renders a Restart button next to Stop", () => {
      const { getByRole } = render(ProcessList, {
        props: makeProps({ processes: [makeProcess({ status: "running" })] }),
      });

      expect(getByRole("button", { name: "Restart api" })).toBeInTheDocument();
    });

    it("calls onRestart when the Restart button is clicked", async () => {
      const onRestart = vi.fn();
      const { getByRole } = render(ProcessList, {
        props: makeProps({
          processes: [makeProcess({ status: "running" })],
          onRestart,
        }),
      });

      await fireEvent.click(getByRole("button", { name: "Restart api" }));

      expect(onRestart).toHaveBeenCalledOnce();
      expect(onRestart).toHaveBeenCalledWith("api");
    });
  });

  describe("when the process is ready", () => {
    it("renders a Stop button", () => {
      const { getByRole } = render(ProcessList, {
        props: makeProps({ processes: [makeProcess({ status: "ready" })] }),
      });

      expect(getByRole("button", { name: "Stop api" })).toBeInTheDocument();
    });
  });

  describe("when the process is failed", () => {
    it("renders a Start button", () => {
      const { getByRole, queryByRole } = render(ProcessList, {
        props: makeProps({ processes: [makeProcess({ status: "failed" })] }),
      });

      expect(getByRole("button", { name: "Start api" })).toBeInTheDocument();
      expect(queryByRole("button", { name: "Stop api" })).toBeNull();
    });

    it("calls onStart when clicked", async () => {
      const onStart = vi.fn();
      const { getByRole } = render(ProcessList, {
        props: makeProps({
          processes: [makeProcess({ status: "failed" })],
          onStart,
        }),
      });

      await fireEvent.click(getByRole("button", { name: "Start api" }));

      expect(onStart).toHaveBeenCalledOnce();
      expect(onStart).toHaveBeenCalledWith("api");
    });
  });

  describe("when the process is stopped or succeeded", () => {
    it.each(["stopped", "succeeded"] as const)(
      "renders a Start button for status %s",
      (status) => {
        const { getByRole } = render(ProcessList, {
          props: makeProps({ processes: [makeProcess({ status })] }),
        });

        expect(getByRole("button", { name: "Start api" })).toBeInTheDocument();
      },
    );
  });

  describe("when the process is in a transitional state", () => {
    it.each(["pending", "blocked", "stopping"] as const)(
      "disables the action button for status %s",
      (status) => {
        const { getByRole } = render(ProcessList, {
          props: makeProps({ processes: [makeProcess({ status })] }),
        });

        // No Stop/Start action is offered; the row's disabled action button
        // has an accessible name equal to the process name only.
        expect(getByRole("button", { name: "api" })).toBeDisabled();
      },
    );

    it.each(["pending", "blocked", "stopping"] as const)(
      "does not render a Restart button for status %s",
      (status) => {
        const { queryByRole } = render(ProcessList, {
          props: makeProps({ processes: [makeProcess({ status })] }),
        });

        expect(queryByRole("button", { name: /Restart api/ })).toBeNull();
      },
    );
  });

  describe("when busy", () => {
    it("disables the action button even for a running process", () => {
      const { getByRole } = render(ProcessList, {
        props: makeProps({
          processes: [makeProcess({ status: "running" })],
          busy: true,
        }),
      });

      expect(getByRole("button", { name: "Stop api" })).toBeDisabled();
    });
  });
});

describe("ProcessList task processes", () => {
  it("has no Stop button when the task is running", () => {
    const { queryByRole } = render(ProcessList, {
      props: makeProps({ processes: [makeProcess({ kind: "task", status: "running" })] }),
    });

    expect(queryByRole("button", { name: "Stop api" })).toBeNull();
  });

  it("has no Start button when the task has succeeded", () => {
    const { queryByRole } = render(ProcessList, {
      props: makeProps({ processes: [makeProcess({ kind: "task", status: "succeeded" })] }),
    });

    expect(queryByRole("button", { name: "Start api" })).toBeNull();
  });

  it("has no Restart button", () => {
    const { queryByRole } = render(ProcessList, {
      props: makeProps({ processes: [makeProcess({ kind: "task", status: "running" })] }),
    });

    expect(queryByRole("button", { name: "Restart api" })).toBeNull();
  });

  it("keeps the running task row selectable without rendering a separate action button", () => {
    const { getByRole, queryByRole } = render(ProcessList, {
      props: makeProps({ processes: [makeProcess({ kind: "task", status: "running" })] }),
    });

    expect(getByRole("button", { name: "api running" })).toBeEnabled();
    expect(queryByRole("button", { name: "api" })).toBeNull();
  });
});
