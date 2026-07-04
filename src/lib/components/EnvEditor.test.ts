import { describe, expect, it, vi } from "vitest";
import { fireEvent, render } from "@testing-library/svelte";

import EnvEditor from "./EnvEditor.svelte";

describe("EnvEditor", () => {
  it("uses distinct accessible names for global and process variable rows", () => {
    const { getByRole } = render(EnvEditor, {
      props: {
        rows: [{ id: "env-1", key: "EXAMPLE_ENV", value: "hello-from-diavola" }],
        issueFor: () => null,
        onAdd: vi.fn(),
        onRemove: vi.fn(),
      },
    });

    render(EnvEditor, {
      props: {
        processId: "process-1",
        rows: [{ id: "env-2", key: "PROCESS_ENV", value: "hello-from-process" }],
        issueFor: () => null,
        onAdd: vi.fn(),
        onRemove: vi.fn(),
      },
    });

    expect(getByRole("textbox", { name: "Global environment variable key 1" })).toBeInTheDocument();
    expect(getByRole("textbox", { name: "Global environment variable value 1" })).toBeInTheDocument();
    expect(getByRole("textbox", { name: "Process environment variable key 1" })).toBeInTheDocument();
    expect(getByRole("textbox", { name: "Process environment variable value 1" })).toBeInTheDocument();
  });

  it("uses an icon-only remove action for variable rows", () => {
    const { container, getByRole, queryByText } = render(EnvEditor, {
      props: {
        rows: [{ id: "env-1", key: "EXAMPLE_ENV", value: "hello-from-diavola" }],
        issueFor: () => null,
        onAdd: vi.fn(),
        onRemove: vi.fn(),
      },
    });

    expect(getByRole("button", { name: "Remove global environment variable 1" })).toBeInTheDocument();
    expect(queryByText("Remove")).toBeNull();
    expect(container).not.toHaveTextContent("Remove global environment variable 1");
  });

  it("attaches process-scoped key validation errors to the key field", () => {
    const { getByRole, getByText } = render(EnvEditor, {
      props: {
        processId: "process-1",
        rows: [{ id: "env-1", key: "EXAMPLE_ENV", value: "hello-from-diavola" }],
        issueFor: (key: string) =>
          key === "process.process-1.env.env-1.key" ? "Environment keys must be unique." : null,
        onAdd: vi.fn(),
        onRemove: vi.fn(),
      },
    });

    const keyInput = getByRole("textbox", { name: "Process environment variable key 1" });
    const message = getByText("Environment keys must be unique.");

    expect(keyInput).toHaveAttribute("aria-invalid", "true");
    expect(keyInput).toHaveAttribute("aria-describedby", "env-error-env-1-key");
    expect(message).toHaveAttribute("id", "env-error-env-1-key");
  });

  it("wires add, remove, and blur callbacks", async () => {
    const onAdd = vi.fn();
    const onRemove = vi.fn();
    const onFieldBlur = vi.fn();
    const { getByRole } = render(EnvEditor, {
      props: {
        processId: "process-1",
        rows: [{ id: "env-1", key: "EXAMPLE_ENV", value: "hello-from-diavola" }],
        issueFor: () => null,
        onAdd,
        onRemove,
        onFieldBlur,
      },
    });

    await fireEvent.click(getByRole("button", { name: "New" }));
    await fireEvent.click(getByRole("button", { name: "Remove process environment variable 1" }));
    await fireEvent.blur(getByRole("textbox", { name: "Process environment variable key 1" }));

    expect(onAdd).toHaveBeenCalledTimes(1);
    expect(onRemove).toHaveBeenCalledWith("env-1");
    expect(onFieldBlur).toHaveBeenCalledWith("process.process-1.env.env-1.key");
  });

  it("keeps the add action width consistent", () => {
    const { getByRole } = render(EnvEditor, {
      props: {
        rows: [{ id: "env-1", key: "EXAMPLE_ENV", value: "hello-from-diavola" }],
        issueFor: () => null,
        onAdd: vi.fn(),
        onRemove: vi.fn(),
      },
    });

    const addButton = getByRole("button", { name: "New" });
    expect(addButton).not.toHaveClass("border");
    expect(addButton.querySelector("svg[aria-hidden='true']")).not.toBeNull();
  });
});
