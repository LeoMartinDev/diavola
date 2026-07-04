import { describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/svelte";

import Button from "./Button.svelte";
import CheckboxField from "./CheckboxField.svelte";
import SegmentedControl from "./SegmentedControl.svelte";
import TextField from "./TextField.svelte";

describe("UI density defaults", () => {
  it("renders default Button with reduced text size and compact height", () => {
    const { getByRole } = render(Button, {
      props: {
        label: "Save",
      },
    });

    const button = getByRole("button");
    expect(button.className).toContain("h-8");
    expect(button.className).toContain("text-[12px]");
  });

  it("renders compact TextField with 13px input text and 12px label", () => {
    const { getByLabelText, container } = render(TextField, {
      props: {
        label: "Name",
        density: "compact",
        value: "",
      },
    });

    const label = container.querySelector("label > span");
    const input = getByLabelText("Name");
    expect(label?.className ?? "").toContain("text-[12px]");
    expect(input.className).toContain("text-[13px]");
  });

  it("renders CheckboxField with unified label sizing", () => {
    const { getByText } = render(CheckboxField, {
      props: {
        label: "Enable readiness check",
      },
    });

    const text = getByText("Enable readiness check");
    const wrapper = text.closest("label");
    expect(wrapper?.className ?? "").toContain("text-[12px]");
  });

  it("renders SegmentedControl options using compact text scale", () => {
    const onChange = vi.fn();
    const { getByRole } = render(SegmentedControl, {
      props: {
        value: "logs",
        options: [
          { value: "logs", label: "Logs" },
          { value: "terminal", label: "Terminal" },
        ],
        onChange,
      },
    });

    const active = getByRole("button", { name: "Logs" });
    expect(active.className).toContain("text-[13px]");
  });
});
