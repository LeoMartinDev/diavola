import { describe, expect, it } from "vitest";
import { render } from "@testing-library/svelte";

import IconButton from "./IconButton.svelte";

describe("IconButton", () => {
  it("supports a compact ghost remove control without a resting border", () => {
    const { getByRole } = render(IconButton, {
      props: {
        label: "Remove variable EXAMPLE_ENV",
        variant: "ghost",
        size: "sm",
      },
    });

    const button = getByRole("button", { name: "Remove variable EXAMPLE_ENV" });

    expect(button).toHaveAccessibleName("Remove variable EXAMPLE_ENV");
    expect(button).toHaveAttribute("type", "button");
    expect(button.className).toContain("h-7");
    expect(button.className).toContain("w-7");
    expect(button.className).not.toContain("border");
  });
});