import { describe, expect, it } from "vitest";
import { render } from "@testing-library/svelte";

import WindowControls from "./WindowControls.svelte";

describe("WindowControls", () => {
  it("keeps the window chrome visible but inert in browser mode", () => {
    const { getByRole } = render(WindowControls);

    expect(getByRole("button", { name: "Minimize" })).toBeDisabled();
    expect(getByRole("button", { name: "Maximize" })).toBeDisabled();
    expect(getByRole("button", { name: "Close" })).toBeDisabled();
  });
});
