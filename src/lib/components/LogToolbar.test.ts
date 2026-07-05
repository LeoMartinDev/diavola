import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";

import LogToolbar from "./LogToolbar.svelte";

function makeProps(overrides: Record<string, unknown> = {}) {
  return {
    query: "",
    autoScroll: true,
    paused: false,
    matchTotal: null,
    activeMatchNumber: 0,
    regexError: null,
    onPrev: vi.fn(),
    onNext: vi.fn(),
    regex: false,
    caseSensitive: false,
    onTogglePause: vi.fn(),
    onClear: vi.fn(),
    ...overrides,
  };
}

describe("LogToolbar", () => {
  it("hides the nav group when matchTotal is null (empty query)", () => {
    const { queryByRole } = render(LogToolbar, { props: makeProps() });
    expect(queryByRole("button", { name: "Next match" })).toBeNull();
    expect(queryByRole("button", { name: "Previous match" })).toBeNull();
  });

  it("renders the counter as active/total", () => {
    const { getByLabelText } = render(LogToolbar, {
      props: makeProps({ matchTotal: 7, activeMatchNumber: 3 }),
    });
    expect(getByLabelText("Match count").textContent?.replace(/\s+/g, "")).toBe("3/7");
  });

  it("invokes onPrev/onNext when the nav buttons are clicked", async () => {
    const onPrev = vi.fn();
    const onNext = vi.fn();
    const { getByRole } = render(LogToolbar, {
      props: makeProps({ matchTotal: 5, activeMatchNumber: 1, onPrev, onNext }),
    });
    await fireEvent.click(getByRole("button", { name: "Next match" }));
    expect(onNext).toHaveBeenCalledOnce();
    await fireEvent.click(getByRole("button", { name: "Previous match" }));
    expect(onPrev).toHaveBeenCalledOnce();
  });

  it("disables the nav buttons when matchTotal is 0", () => {
    const { getByRole } = render(LogToolbar, {
      props: makeProps({ matchTotal: 0, activeMatchNumber: 0 }),
    });
    expect(getByRole("button", { name: "Next match" })).toBeDisabled();
    expect(getByRole("button", { name: "Previous match" })).toBeDisabled();
  });

  it("toggles regex via the .* button and reflects aria-pressed", async () => {
    const { getByRole } = render(LogToolbar, {
      props: makeProps({ matchTotal: 1 }),
    });
    const btn = getByRole("button", { name: "Toggle regex" });
    expect(btn).toHaveAttribute("aria-pressed", "false");
    await fireEvent.click(btn);
    expect(btn).toHaveAttribute("aria-pressed", "true");
  });

  it("toggles case-sensitive via the Aa button and reflects aria-pressed", async () => {
    const { getByRole } = render(LogToolbar, {
      props: makeProps({ matchTotal: 1 }),
    });
    const btn = getByRole("button", { name: "Toggle case sensitive" });
    expect(btn).toHaveAttribute("aria-pressed", "false");
    await fireEvent.click(btn);
    expect(btn).toHaveAttribute("aria-pressed", "true");
  });

  it("disables nav and shows the popover when regexError is set", () => {
    const { getByRole, getByText } = render(LogToolbar, {
      props: makeProps({
        matchTotal: 3,
        activeMatchNumber: 1,
        regexError: "Unterminated group",
      }),
    });
    expect(getByRole("button", { name: "Next match" })).toBeDisabled();
    expect(getByText("Unterminated group")).toBeInTheDocument();
  });
});
