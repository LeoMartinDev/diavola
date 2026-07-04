import { describe, expect, it } from "vitest";
import { render } from "@testing-library/svelte";

import LogViewer from "./LogViewer.svelte";
import ProcessList from "./ProcessList.svelte";
import type { ProcessLogPayload, ProcessSnapshot, TerminalSnapshot } from "$lib/types";

class ResizeObserverMock {
  observe() {}
  disconnect() {}
}

Object.defineProperty(globalThis, "ResizeObserver", {
  value: ResizeObserverMock,
  configurable: true,
});

const process: ProcessSnapshot = {
  runtimeId: "r1",
  name: "api",
  kind: "service",
  status: "running",
};

const logEntry: ProcessLogPayload = {
  timestamp: "2026-06-27T12:00:00.000Z",
  stream: "stdout",
  line: "server listening on :3000",
};

describe("Native feel affordances", () => {
  it("marks sidebar process labels as non-selectable chrome", () => {
    const { getByText } = render(ProcessList, {
      props: {
        processes: [process],
        terminals: [] as TerminalSnapshot[],
        selectedProcessRuntimeId: null,
        selectedTerminalId: null,
        busy: false,
        onSelectProcess: () => {},
        onSelectTerminal: () => {},
        onStop: () => {},
        onStart: () => {},
        onRestart: () => {},
        onCloseTerminal: () => {},
      },
    });

    expect(getByText("api").className).toContain("select-none");
    expect(getByText("running").className).toContain("select-none");
  });

  it("keeps the log viewport explicitly selectable", () => {
    const { container } = render(LogViewer, {
      props: {
        logs: [logEntry],
        processName: "api",
        truncatedCount: 0,
        onClear: () => {},
      },
    });

    expect(container.querySelector('[data-native-selectable="logs"]')).not.toBeNull();
  });
});