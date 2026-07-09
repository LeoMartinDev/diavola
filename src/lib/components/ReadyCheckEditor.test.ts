import { describe, expect, it, vi } from "vitest";
import { fireEvent, render } from "@testing-library/svelte";

import ReadyCheckEditor from "./ReadyCheckEditor.svelte";
import type { ProcessForm } from "$lib/config/editorModel";

const process: ProcessForm = {
  id: "process-1",
  name: "api",
  kind: "service",
  cmd: "deno task dev",
  envRows: [],
  dependencies: [],
  readyEnabled: true,
  readyType: "log",
  httpUrl: "http://localhost:3000",
  logPattern: "ready",
  logRegex: false,
  delayDurationMs: 1000,
  commandCmd: "",
  intervalMs: null,
  timeoutMs: 60000,
};

function createProcess(overrides: Partial<ProcessForm> = {}): ProcessForm {
  return {
    ...process,
    ...overrides,
  };
}

describe("ReadyCheckEditor", () => {
  it("marks the readiness section as a flat settings surface", () => {
    const { container } = render(ReadyCheckEditor, {
      props: {
        process: createProcess(),
        readyIssue: () => null,
      },
    });

    expect(container.querySelector("section")).toHaveAttribute("data-settings-surface", "flat");
  });

  it("hides readiness-specific fields when the readiness check is disabled", () => {
    const { getByLabelText, queryByLabelText } = render(ReadyCheckEditor, {
      props: {
        process: createProcess({
          readyEnabled: false,
        }),
        readyIssue: () => null,
      },
    });

    expect(getByLabelText("Enable readiness check")).not.toBeChecked();
    expect(queryByLabelText("Type")).not.toBeInTheDocument();
    expect(queryByLabelText("Pattern")).not.toBeInTheDocument();
    expect(queryByLabelText("Timeout ms")).not.toBeInTheDocument();
  });

  it("renders the enabled log readiness controls as a single visible flow", () => {
    const { getByLabelText, getByRole, queryByLabelText } = render(ReadyCheckEditor, {
      props: {
        process: createProcess(),
        readyIssue: () => null,
      },
    });

    expect(getByRole("heading", { name: "Readiness" })).toBeInTheDocument();
    expect(getByLabelText("Enable readiness check")).toBeChecked();
    expect(getByLabelText("Type")).toHaveValue("log");
    expect(getByLabelText("Pattern")).toBeInTheDocument();
    expect(getByLabelText("Regex")).not.toBeChecked();
    expect(getByLabelText("Timeout ms")).toHaveValue(60000);
    expect(queryByLabelText("Interval ms")).not.toBeInTheDocument();
  });

  it("renders command readiness fields outside the log branch", () => {
    const { getByLabelText, queryByLabelText } = render(ReadyCheckEditor, {
      props: {
        process: createProcess({
          readyType: "command",
          commandCmd: "curl -f http://localhost:3000/health",
          intervalMs: 500,
        }),
        readyIssue: () => null,
      },
    });

    expect(getByLabelText("Command")).toBeInTheDocument();
    expect(getByLabelText("Interval ms")).toBeInTheDocument();
    expect(queryByLabelText("Pattern")).not.toBeInTheDocument();
  });

  it("renders http readiness fields with polling controls", () => {
    const { getByLabelText, queryByLabelText } = render(ReadyCheckEditor, {
      props: {
        process: createProcess({
          readyType: "http",
          intervalMs: 500,
        }),
        readyIssue: () => null,
      },
    });

    expect(getByLabelText("URL")).toHaveValue("http://localhost:3000");
    expect(getByLabelText("Interval ms")).toHaveValue(500);
    expect(getByLabelText("Timeout ms")).toHaveValue(60000);
    expect(queryByLabelText("Pattern")).not.toBeInTheDocument();
  });

  it("renders delay readiness fields without polling controls", () => {
    const { getByLabelText, queryByLabelText } = render(ReadyCheckEditor, {
      props: {
        process: createProcess({
          readyType: "delay",
          delayDurationMs: 2500,
        }),
        readyIssue: () => null,
      },
    });

    expect(getByLabelText("Duration ms")).toHaveValue(2500);
    expect(queryByLabelText("Interval ms")).not.toBeInTheDocument();
    expect(queryByLabelText("Timeout ms")).not.toBeInTheDocument();
  });

  it("shows a recoverable warning for unknown readiness types and clears it after selecting a supported type", async () => {
    const { getByLabelText, getByText, queryByLabelText, queryByText } = render(ReadyCheckEditor, {
      props: {
        process: createProcess({
          readyType: "custom" as ProcessForm["readyType"],
        }),
        readyIssue: () => null,
      },
    });

    const typeField = getByLabelText("Type");
    const warning = getByText("Unknown readiness type. Choose a supported type.");

    expect(warning).toBeInTheDocument();
  expect(warning).toHaveClass("text-danger");
    expect(typeField).toBeInTheDocument();
    expect(typeField).toHaveValue("custom");
    expect(typeField).toHaveAttribute("aria-invalid", "true");
    expect(warning).toHaveAttribute("id", "process-process-1-ready-type-error");
    expect(typeField).toHaveAttribute("aria-describedby", warning.id);
    expect(queryByLabelText("Command")).not.toBeInTheDocument();
    expect(queryByLabelText("Pattern")).not.toBeInTheDocument();
    expect(queryByLabelText("Interval ms")).not.toBeInTheDocument();
    expect(queryByLabelText("Timeout ms")).not.toBeInTheDocument();

    await fireEvent.change(typeField, { target: { value: "http" } });

    expect(typeField).toHaveValue("http");
    expect(typeField).not.toHaveAttribute("aria-invalid");
    expect(typeField).not.toHaveAttribute("aria-describedby", warning.id);
    expect(queryByText("Unknown readiness type. Choose a supported type.")).not.toBeInTheDocument();
    expect(queryByLabelText("URL")).toHaveValue("http://localhost:3000");
    expect(queryByLabelText("Timeout ms")).toHaveValue(60000);
  });

  it("wires blur callbacks for readiness fields", async () => {
    const onFieldBlur = vi.fn();
    const { getByLabelText } = render(ReadyCheckEditor, {
      props: {
        process: createProcess(),
        readyIssue: () => null,
        onFieldBlur,
      },
    });

    await fireEvent.blur(getByLabelText("Pattern"));

    expect(onFieldBlur).toHaveBeenCalledWith("process.process-1.ready.logPattern");
  });

  it("wires blur callbacks for http readiness polling fields", async () => {
    const onFieldBlur = vi.fn();
    const { getByLabelText } = render(ReadyCheckEditor, {
      props: {
        process: createProcess({
          readyType: "http",
          intervalMs: 500,
        }),
        readyIssue: () => null,
        onFieldBlur,
      },
    });

    await fireEvent.blur(getByLabelText("Interval ms"));

    expect(onFieldBlur).toHaveBeenCalledWith("process.process-1.ready.intervalMs");
  });

  it("associates ready validation errors with their owning field", () => {
    const { getByLabelText, getByText } = render(ReadyCheckEditor, {
      props: {
        process: createProcess({
          readyType: "command",
          commandCmd: "",
        }),
        readyIssue: (_process, field) => (field === "commandCmd" ? "Command is required" : null),
      },
    });

    const commandField = getByLabelText("Command");
    const error = getByText("Command is required");

    expect(commandField).toHaveAttribute("aria-invalid", "true");
    expect(error).toHaveAttribute("id", "process-process-1-ready-commandCmd-error");
    expect(commandField).toHaveAttribute("aria-describedby", error.id);
  });
});