import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";

import LogViewer from "./LogViewer.svelte";
import type { ProcessLogPayload } from "$lib/types";

let originalScrollTo: unknown;

beforeEach(() => {
  originalScrollTo = Element.prototype.scrollTo;
  Element.prototype.scrollTo = vi.fn() as unknown as typeof Element.prototype.scrollTo;
  vi.stubGlobal("ResizeObserver", class {
    observe() {}
    unobserve() {}
    disconnect() {}
  });
  localStorage.clear();
});

afterEach(() => {
  Element.prototype.scrollTo = originalScrollTo as typeof Element.prototype.scrollTo;
  vi.restoreAllMocks();
});

function makeLine(text: string, i: number): ProcessLogPayload {
  return {
    sessionId: "s1",
    runtimeId: "r1",
    processName: "api",
    stream: "stdout",
    line: text,
    timestamp: `2026-01-01T00:00:${i.toString().padStart(2, "0")}Z`,
  };
}

const logs = (lines: string[]): ProcessLogPayload[] => lines.map(makeLine);

function makeProps(overrides: Record<string, unknown> = {}) {
  return {
    logs: [] as ProcessLogPayload[],
    processName: "api",
    truncatedCount: 0,
    onClear: vi.fn(),
    ...overrides,
  };
}

describe("LogViewer search", () => {
  it("filters lines by a substring query (case-insensitive)", async () => {
    const { container, queryByText } = render(LogViewer, {
      props: makeProps({
        logs: logs(["listening on 3000", "worker ready", "listening on 3001"]),
      }),
    });
    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    input.value = "LISTENING";
    await fireEvent.input(input);
    expect(queryByText(/worker ready/)).toBeNull();
  });

  it("treats the query as regex when regex mode is on", async () => {
    const { container } = render(LogViewer, {
      props: makeProps({ logs: logs(["error 42", "warn 7", "error 99"]) }),
    });
    await fireEvent.click(container.querySelector('[aria-label="Toggle regex"]')!);
    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    input.value = "error \\d+";
    await fireEvent.input(input);
    expect(container.textContent).toMatch(/error 42/);
    expect(container.textContent).not.toMatch(/warn 7/);
  });

  it("shows the error popover and disables nav on an invalid regex", async () => {
    const { container, getByText } = render(LogViewer, {
      props: makeProps({ logs: logs(["api listening"]) }),
    });
    await fireEvent.click(container.querySelector('[aria-label="Toggle regex"]')!);
    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    input.value = "(unclosed";
    await fireEvent.input(input);
    await fireEvent.focus(input);
    expect(getByText(/Unterminated|Invalid|regular expression/i)).toBeInTheDocument();
    expect(container.querySelector('[aria-label="Next match"]')).toBeDisabled();
  });

  it("navigates matches with Enter / Shift+Enter and wraps around", async () => {
    const { container } = render(LogViewer, {
      props: makeProps({ logs: logs(["a one", "b one", "c one"]) }),
    });
    const counter = () =>
      container
        .querySelector('[aria-label="Match count"]')
        ?.textContent?.replace(/\s+/g, "");

    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    input.value = "one";
    await fireEvent.input(input);
    expect(counter()).toBe("1/3");

    await fireEvent.keyDown(input, { key: "Enter" });
    expect(counter()).toBe("2/3");

    await fireEvent.keyDown(input, { key: "Enter" });
    expect(counter()).toBe("3/3");

    await fireEvent.keyDown(input, { key: "Enter" });
    expect(counter()).toBe("1/3");

    await fireEvent.keyDown(input, { key: "Enter", shiftKey: true });
    expect(counter()).toBe("3/3");
  });
});
