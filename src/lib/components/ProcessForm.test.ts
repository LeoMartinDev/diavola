import { describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/svelte";

import ProcessForm from "./ProcessForm.svelte";
import type { ProcessForm as ProcessFormState } from "$lib/config/editorModel";

const process: ProcessFormState = {
  id: "process-1",
  name: "api",
  kind: "service",
  cmd: "deno task dev",
  envRows: [],
  dependencies: [],
  readyEnabled: false,
  readyType: "http",
  httpUrl: "http://localhost:3000",
  logPattern: "ready",
  logRegex: false,
  delayDurationMs: 1000,
  commandCmd: "",
  intervalMs: null,
  timeoutMs: 60000,
};

describe("ProcessForm", () => {
  it("uses an icon-only remove action for the selected process", () => {
    const { getByRole, queryByText } = render(ProcessForm, {
      props: {
        process,
        processCount: 2,
        processIssue: () => null,
        onRemove: vi.fn(),
      },
    });

    expect(getByRole("button", { name: "Remove process" })).toBeInTheDocument();
    expect(queryByText("Remove process")).toBeNull();
  });

  it("places the command editor below name and kind", () => {
    const { getByText } = render(ProcessForm, {
      props: {
        process,
        processCount: 2,
        processIssue: () => null,
        onRemove: vi.fn(),
      },
    });

    const nameField = getByText("Name").closest("label");
    const kindField = getByText("Kind").closest("label");
    const commandField = getByText("Command").closest(".command-code-editor");

    expect(nameField).not.toBeNull();
    expect(kindField).not.toBeNull();
    expect(commandField).not.toBeNull();
    expect(
      nameField!.compareDocumentPosition(kindField!) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(
      kindField!.compareDocumentPosition(commandField!) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  it("renders the command field with a compact CodeMirror editor", () => {
    const { container } = render(ProcessForm, {
      props: {
        process,
        processCount: 2,
        processIssue: () => null,
        onRemove: vi.fn(),
      },
    });

    expect(container.querySelector(".command-code-editor .cm-editor"))
      .toBeInTheDocument();
    expect(container.querySelector(".command-code-editor .cm-content"))
      .toHaveTextContent("deno task dev");
  });

  it("renders always-visible line numbers in the command editor", () => {
    const { container } = render(ProcessForm, {
      props: {
        process,
        processCount: 2,
        processIssue: () => null,
        onRemove: vi.fn(),
      },
    });

    expect(container.querySelector(".command-code-editor .cm-gutters"))
      .toBeInTheDocument();
    expect(container.querySelector(".command-code-editor .cm-lineNumbers"))
      .toBeInTheDocument();
    const gutterValues = Array.from(
      container.querySelectorAll(".command-code-editor .cm-gutterElement"),
    )
      .map((element) => element.textContent?.trim() ?? "")
      .filter(Boolean);

    expect(gutterValues.length).toBeGreaterThan(0);
    expect(gutterValues.every((value) => /^\d+$/.test(value))).toBe(true);
  });
});
