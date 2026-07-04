import { describe, expect, it } from "vitest";
import { render } from "@testing-library/svelte";

import Button from "./Button.svelte";

describe("Button", () => {
  it("supports an extra-small button density", () => {
    const { getByRole } = render(Button, {
      props: {
        size: "xs",
      },
    });

    const button = getByRole("button");
    expect(button).toHaveClass("button-xs");
    expect(button).not.toHaveClass("text-[10px]");
  });

  it("uses a smaller text size for small buttons", () => {
    const { getByRole } = render(Button, {
      props: {
        size: "sm",
      },
    });

    const button = getByRole("button");
    expect(button).toHaveClass("text-[10px]");
  });

  it("uses a smaller text size for medium buttons", () => {
    const { getByRole } = render(Button);
    const button = getByRole("button");
    expect(button).toHaveClass("text-[12px]");
  });
});
