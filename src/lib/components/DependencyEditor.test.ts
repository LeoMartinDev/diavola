import { describe, expect, it, vi } from "vitest";
import { fireEvent, render } from "@testing-library/svelte";

import DependencyEditor from "./DependencyEditor.svelte";
import type { ProcessForm } from "$lib/config/editorModel";

const process: ProcessForm = {
  id: "process-1",
  name: "worker",
  kind: "service",
  cmd: "deno task worker",
  envRows: [],
  dependencies: [{ id: "dependency-1", processName: "api", condition: "ready" }],
  readyEnabled: false,
  readyType: "http",
  httpUrl: "http://localhost:3000",
  logPattern: "ready",
  logRegex: false,
  delayDurationMs: 1000,
  commandCmd: "",
  intervalMs: null,
  timeoutMs: 60000,
  stopTimeoutMs: null,
};

describe("DependencyEditor", () => {
  it("exposes an accessible label for each dependency row selector", () => {
    const { getByRole } = render(DependencyEditor, {
      props: {
        process,
        processes: [
          process,
          { ...process, id: "process-2", name: "api" },
        ],
        dependencyIssue: () => null,
        onAdd: vi.fn(),
        onRemove: vi.fn(),
      },
    });

    expect(getByRole("combobox", { name: "Dependency process 1" })).toBeInTheDocument();
    expect(getByRole("combobox", { name: "Dependency condition 1" })).toBeInTheDocument();
  });

  it("uses an icon-only remove action for dependency rows", () => {
    const { container, getByRole, queryByText } = render(DependencyEditor, {
      props: {
        process,
        processes: [
          process,
          { ...process, id: "process-2", name: "api" },
        ],
        dependencyIssue: () => null,
        onAdd: vi.fn(),
        onRemove: vi.fn(),
      },
    });

    expect(getByRole("button", { name: "Remove dependency 1" })).toBeInTheDocument();
    expect(queryByText("Remove")).toBeNull();
    expect(container).not.toHaveTextContent("Remove dependency 1");
  });

  it("attaches dependency validation errors to the dependency process selector", () => {
    const { getByRole, getByText } = render(DependencyEditor, {
      props: {
        process,
        processes: [
          process,
          { ...process, id: "process-2", name: "api" },
        ],
        dependencyIssue: (_process, dependencyId) =>
          dependencyId === "dependency-1" ? "Dependency must reference an existing process." : null,
        onAdd: vi.fn(),
        onRemove: vi.fn(),
      },
    });

    const dependencySelect = getByRole("combobox", { name: "Dependency process 1" });
    const message = getByText("Dependency must reference an existing process.");

    expect(dependencySelect).toHaveAttribute("aria-invalid", "true");
    expect(dependencySelect).toHaveAttribute("aria-describedby", "dependency-error-process-1-dependency-1");
    expect(message).toHaveAttribute("id", "dependency-error-process-1-dependency-1");
  });

  it("wires add, remove, and blur callbacks", async () => {
    const onAdd = vi.fn();
    const onRemove = vi.fn();
    const onFieldBlur = vi.fn();
    const { getByRole } = render(DependencyEditor, {
      props: {
        process: {
          ...process,
          dependencies: [{ id: "dependency-1", processName: "api", condition: "ready" }],
        },
        processes: [
          process,
          { ...process, id: "process-2", name: "api" },
        ],
        dependencyIssue: () => null,
        onAdd,
        onRemove,
        onFieldBlur,
      },
    });

    await fireEvent.click(getByRole("button", { name: "New" }));
    await fireEvent.click(getByRole("button", { name: "Remove dependency 1" }));
    await fireEvent.blur(getByRole("combobox", { name: "Dependency process 1" }));

    expect(onAdd).toHaveBeenCalledTimes(1);
    expect(onAdd).toHaveBeenCalledWith(expect.objectContaining({ id: "process-1" }));
    expect(onRemove).toHaveBeenCalledWith(expect.objectContaining({ id: "process-1" }), "dependency-1");
    expect(onFieldBlur).toHaveBeenCalledWith("process.process-1.dependency.dependency-1");
  });

  it("keeps the add action width consistent", () => {
    const { getByRole } = render(DependencyEditor, {
      props: {
        process: {
          ...process,
          dependencies: [],
        },
        processes: [
          process,
          { ...process, id: "process-2", name: "api" },
        ],
        dependencyIssue: () => null,
        onAdd: vi.fn(),
        onRemove: vi.fn(),
      },
    });

    const addButton = getByRole("button", { name: "New" });
    expect(addButton).not.toHaveClass("border");
    expect(addButton.querySelector("svg[aria-hidden='true']")).not.toBeNull();
  });
});