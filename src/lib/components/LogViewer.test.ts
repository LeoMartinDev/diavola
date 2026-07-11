import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";

import LogViewer from "./LogViewer.svelte";
import type { FlatRow } from "$lib/types";

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

function makeRow(text: string, i: number, overrides: Partial<FlatRow> = {}): FlatRow {
  return {
    entryId: i,
    lineIndex: 0,
    isFirstLine: true,
    isContinuation: false,
    text,
    stream: "stdout" as const,
    timestamp: `2026-01-01T00:00:${i.toString().padStart(2, "0")}Z`,
    ...overrides,
  };
}

const logs = (lines: string[]): FlatRow[] => lines.map((text, i) => makeRow(text, i));

function makeProps(overrides: Record<string, unknown> = {}) {
  return {
    logs: [] as FlatRow[],
    processName: "api",
    truncatedCount: 0,
    onClear: vi.fn(),
    ...overrides,
  };
}

describe("LogViewer search", () => {
  it("does not render menu actions in the log toolbar", () => {
    const menuActions = createRawSnippet(() => ({
      render: () => '<button aria-label="Project menu">Menu</button>',
    }));

    const { queryByRole } = render(LogViewer, {
      props: makeProps({ menuActions }),
    });

    expect(queryByRole("button", { name: "Project menu" })).toBeNull();
  });

  it("keeps all rows rendered and highlights matches (no filtering)", async () => {
    const { container } = render(LogViewer, {
      props: makeProps({
        logs: logs(["listening on 3000", "worker ready", "listening on 3001"]),
      }),
    });
    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    input.value = "LISTENING";
    await fireEvent.input(input);
    await new Promise((r) => setTimeout(r, 100));
    expect(container.textContent).toContain("worker ready");
    const marks = container.querySelectorAll("mark");
    expect(marks.length).toBeGreaterThanOrEqual(2);
  });

  it("highlights regex matches without hiding non-matches", async () => {
    const { container } = render(LogViewer, {
      props: makeProps({ logs: logs(["error 42", "warn 7", "error 99"]) }),
    });
    await fireEvent.click(container.querySelector('[aria-label="Toggle regex"]')!);
    const input = container.querySelector<HTMLInputElement>(".log-search")!;
    input.value = "error \\d+";
    await fireEvent.input(input);
    await new Promise((r) => setTimeout(r, 100));
    expect(container.textContent).toContain("warn 7");
    expect(container.querySelectorAll("mark").length).toBe(2);
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
    await new Promise((r) => setTimeout(r, 100));
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
    await new Promise((r) => setTimeout(r, 100));
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

  it("renders multi-line entries without timestamps on continuation lines", async () => {
    const multiLineLogs: FlatRow[] = [
      makeRow("Error: something failed", 0),
      makeRow("  at app.ts:42", 0, { lineIndex: 1, isFirstLine: false, isContinuation: true, timestamp: '' }),
      makeRow("  at db.ts:15", 0, { lineIndex: 2, isFirstLine: false, isContinuation: true, timestamp: '' }),
    ];
    const { container } = render(LogViewer, {
      props: makeProps({ logs: multiLineLogs }),
    });
    expect(container.textContent).toContain("Error: something failed");
    expect(container.textContent).toContain("at app.ts:42");
  });

  it("copies the full entry when copy button is clicked", async () => {
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
    const entryLines: FlatRow[] = [
      makeRow("line 1", 0),
      makeRow("line 2 continuation", 0, { lineIndex: 1, isFirstLine: false, isContinuation: true, timestamp: '' }),
    ];
    const { container } = render(LogViewer, {
      props: makeProps({ logs: entryLines }),
    });
    const copyBtn = container.querySelector('[title="Copy entry"]') as HTMLButtonElement;
    if (copyBtn) {
      await fireEvent.click(copyBtn);
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith("line 1\nline 2 continuation");
    }
  });

  it("visually groups object-like continuation rows even when each line is a separate payload", async () => {
    const objectRows = [
      makeRow("currentFiscalYearConfiguration: {", 1),
      makeRow('"fiscalRegime": "is",', 2),
      makeRow('"year": 2025', 3),
      makeRow("}", 4),
      makeRow("responseTimeMs: 21", 5),
    ];

    const { container } = render(LogViewer, {
      props: makeProps({ logs: objectRows }),
    });

    const renderedRows = Array.from(container.querySelectorAll('[data-log-row="true"]'));
    expect(renderedRows).toHaveLength(5);
    expect(renderedRows[0].className).toContain("rounded-tl");
    expect(renderedRows[1].className).not.toContain("rounded-tl");
    expect(renderedRows[2].className).not.toContain("rounded-bl");
    expect(renderedRows[3].className).toContain("rounded-bl");
    expect(renderedRows[4].className).toContain("rounded-tl");
    expect(renderedRows[4].className).toContain("rounded-bl");
  });

  it("keeps nested object-like payloads in one contiguous visual group", async () => {
    const objectRows = [
      makeRow("payload: {", 1),
      makeRow('"nested": {', 2),
      makeRow('"value": 1', 3),
      makeRow("}", 4),
      makeRow("}", 5),
      makeRow("responseTimeMs: 21", 6),
    ];

    const { container } = render(LogViewer, {
      props: makeProps({ logs: objectRows }),
    });

    const renderedRows = Array.from(container.querySelectorAll('[data-log-row="true"]'));
    expect(renderedRows).toHaveLength(6);
    expect(renderedRows[0].className).toContain("rounded-tl");
    expect(renderedRows[1].className).not.toContain("rounded-tl");
    expect(renderedRows[1].className).not.toContain("rounded-bl");
    expect(renderedRows[2].className).not.toContain("rounded-tl");
    expect(renderedRows[2].className).not.toContain("rounded-bl");
    expect(renderedRows[3].className).not.toContain("rounded-tl");
    expect(renderedRows[4].className).toContain("rounded-bl");
    expect(renderedRows[5].className).toContain("rounded-tl");
    expect(renderedRows[5].className).toContain("rounded-bl");
  });

  it("keeps anonymous nested containers within the surrounding structured visual group", async () => {
    const objectRows = [
      makeRow("payload: [", 1),
      makeRow("{", 2),
      makeRow('"id": 1', 3),
      makeRow("}", 4),
      makeRow("]", 5),
      makeRow("responseTimeMs: 21", 6),
    ];

    const { container } = render(LogViewer, {
      props: makeProps({ logs: objectRows }),
    });

    const renderedRows = Array.from(container.querySelectorAll('[data-log-row="true"]'));
    expect(renderedRows).toHaveLength(6);
    expect(renderedRows[0].className).toContain("rounded-tl");
    expect(renderedRows[0].className).not.toContain("rounded-bl");
    expect(renderedRows[1].className).not.toContain("rounded-tl");
    expect(renderedRows[1].className).not.toContain("rounded-bl");
    expect(renderedRows[2].className).not.toContain("rounded-tl");
    expect(renderedRows[2].className).not.toContain("rounded-bl");
    expect(renderedRows[3].className).not.toContain("rounded-tl");
    expect(renderedRows[3].className).not.toContain("rounded-bl");
    expect(renderedRows[4].className).toContain("rounded-bl");
    expect(renderedRows[5].className).toContain("rounded-tl");
    expect(renderedRows[5].className).toContain("rounded-bl");
  });

  it("keeps adjacent top-level anonymous containers as separate visual groups", async () => {
    const objectRows = [
      makeRow("{", 1),
      makeRow('"id": 1', 2),
      makeRow("}", 3),
      makeRow("{", 4),
      makeRow('"id": 2', 5),
      makeRow("}", 6),
    ];

    const { container } = render(LogViewer, {
      props: makeProps({ logs: objectRows }),
    });

    const renderedRows = Array.from(container.querySelectorAll('[data-log-row="true"]'));
    expect(renderedRows).toHaveLength(6);
    expect(renderedRows[0].className).toContain("rounded-tl");
    expect(renderedRows[0].className).not.toContain("rounded-bl");
    expect(renderedRows[1].className).not.toContain("rounded-tl");
    expect(renderedRows[1].className).not.toContain("rounded-bl");
    expect(renderedRows[2].className).toContain("rounded-bl");
    expect(renderedRows[3].className).toContain("rounded-tl");
    expect(renderedRows[3].className).not.toContain("rounded-bl");
    expect(renderedRows[4].className).not.toContain("rounded-tl");
    expect(renderedRows[4].className).not.toContain("rounded-bl");
    expect(renderedRows[5].className).toContain("rounded-bl");
  });

  it("groups indented continuation lines with their parent entry", async () => {
    const rows = [
      makeRow("[12:00:01] INFO  Starting application...", 1),
      makeRow("  Initializing database connection pool", 2),
      makeRow("  Loading configuration from /etc/app/config.yml", 3),
      makeRow("[12:00:02] INFO  Application started successfully", 4),
    ];

    const { container } = render(LogViewer, {
      props: makeProps({ logs: rows }),
    });

    const renderedRows = Array.from(container.querySelectorAll('[data-log-row="true"]'));
    expect(renderedRows).toHaveLength(4);
    expect(renderedRows[0].className).toContain("rounded-tl");
    expect(renderedRows[0].className).not.toContain("rounded-bl");
    expect(renderedRows[1].className).not.toContain("rounded-tl");
    expect(renderedRows[1].className).not.toContain("rounded-bl");
    expect(renderedRows[2].className).toContain("rounded-bl");
    expect(renderedRows[3].className).toContain("rounded-tl");
    expect(renderedRows[3].className).toContain("rounded-bl");
  });

  it("does not merge visual groups across stdout and stderr boundaries", async () => {
    const objectRows = [
      makeRow("payload: {", 1, { stream: "stdout" }),
      makeRow('"message": "boom"', 2, { stream: "stderr" }),
      makeRow("}", 3, { stream: "stdout" }),
    ];

    const { container } = render(LogViewer, {
      props: makeProps({ logs: objectRows }),
    });

    const renderedRows = Array.from(container.querySelectorAll('[data-log-row="true"]'));
    expect(renderedRows).toHaveLength(3);
    expect(renderedRows[0].className).toContain("rounded-tl");
    expect(renderedRows[0].className).toContain("rounded-bl");
    expect(renderedRows[1].className).toContain("rounded-tl");
    expect(renderedRows[1].className).toContain("rounded-bl");
    expect(renderedRows[2].className).toContain("rounded-tl");
    expect(renderedRows[2].className).toContain("rounded-bl");
  });
});

describe("LogViewer autoscroll", () => {
  it("keeps rendering newest rows after many logs arrive while pinned to bottom", async () => {
    Element.prototype.scrollTo = vi.fn(function (this: Element, options?: ScrollToOptions | number) {
      if (typeof options === "object" && options !== null && "top" in options) {
        Object.defineProperty(this, "scrollTop", {
          configurable: true,
          value: Number(options.top ?? 0),
        });
      }
    }) as unknown as typeof Element.prototype.scrollTo;

    const initialLogs = logs(Array.from({ length: 20 }, (_, i) => `line ${i}`));
    const { container, rerender } = render(LogViewer, {
      props: makeProps({ logs: initialLogs }),
    });

    const viewport = container.querySelector('[data-native-selectable="logs"]') as HTMLDivElement;
    Object.defineProperty(viewport, "clientHeight", { configurable: true, value: 220 });
    Object.defineProperty(viewport, "scrollHeight", { configurable: true, value: 440 });
    Object.defineProperty(viewport, "scrollTop", { configurable: true, value: 220 });
    await fireEvent.scroll(viewport);

    const nextLogs = logs(Array.from({ length: 1_000 }, (_, i) => `line ${i}`));
    Object.defineProperty(viewport, "scrollHeight", { configurable: true, value: 22_000 });
    await rerender(makeProps({ logs: nextLogs }));

    await new Promise((resolve) => requestAnimationFrame(resolve));

    expect(container.textContent).toContain("line 999");
  });
});
