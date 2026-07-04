import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";

import Menu from "./Menu.svelte";

const baseProps = {
  label: "Actions",
  items: [
    { label: "First", onSelect: vi.fn() },
    { label: "Second", onSelect: vi.fn() },
    { label: "Dangerous", onSelect: vi.fn(), danger: true },
  ],
};

describe("Menu", () => {
  it("does not render menu items until opened", () => {
    const { getByRole, queryByRole } = render(Menu, { props: baseProps });

    // Trigger is always present.
    expect(getByRole("button", { name: "Actions" })).toBeInTheDocument();
    // Items are not yet in the document.
    expect(queryByRole("menuitem", { name: "First" })).toBeNull();
  });

  it("opens on trigger click and exposes the items", async () => {
    const { getByRole } = render(Menu, { props: baseProps });

    await fireEvent.click(getByRole("button", { name: "Actions" }));

    expect(getByRole("menuitem", { name: "First" })).toBeInTheDocument();
    expect(getByRole("menuitem", { name: "Second" })).toBeInTheDocument();
    expect(getByRole("menuitem", { name: "Dangerous" })).toBeInTheDocument();
  });

  it("calls the item's onSelect and closes the menu when an item is chosen", async () => {
    const onFirst = vi.fn();
    const { getByRole, queryByRole } = render(Menu, {
      props: {
        label: "Actions",
        items: [
          { label: "First", onSelect: onFirst },
          { label: "Second", onSelect: vi.fn() },
        ],
      },
    });

    await fireEvent.click(getByRole("button", { name: "Actions" }));
    await fireEvent.click(getByRole("menuitem", { name: "First" }));

    expect(onFirst).toHaveBeenCalledOnce();
    // Menu closes after selection.
    expect(queryByRole("menuitem", { name: "First" })).toBeNull();
  });

  it("renders disabled items and does not call their onSelect when clicked", async () => {
    const onDisabled = vi.fn();
    const { getByRole } = render(Menu, {
      props: {
        label: "Actions",
        items: [{ label: "Off", onSelect: onDisabled, disabled: true }],
      },
    });

    await fireEvent.click(getByRole("button", { name: "Actions" }));
    const item = getByRole("menuitem", { name: "Off" });

    expect(item).toBeDisabled();
    await fireEvent.click(item);

    expect(onDisabled).not.toHaveBeenCalled();
  });

  it("closes when Escape is pressed", async () => {
    const { getByRole, queryByRole } = render(Menu, { props: baseProps });

    await fireEvent.click(getByRole("button", { name: "Actions" }));
    expect(getByRole("menuitem", { name: "First" })).toBeInTheDocument();

    await fireEvent.keyDown(window, { key: "Escape" });

    expect(queryByRole("menuitem", { name: "First" })).toBeNull();
  });

  it("closes on pointerdown outside the menu", async () => {
    const { getByRole, queryByRole, baseElement } = render(Menu, { props: baseProps });

    await fireEvent.click(getByRole("button", { name: "Actions" }));
    expect(getByRole("menuitem", { name: "First" })).toBeInTheDocument();

    // Simulate a click somewhere outside the menu root.
    await fireEvent.pointerDown(baseElement);

    expect(queryByRole("menuitem", { name: "First" })).toBeNull();
  });

  it("focuses the first enabled item when opened and supports arrow navigation", async () => {
    const { getByRole } = render(Menu, {
      props: {
        label: "Actions",
        items: [
          { label: "Disabled", onSelect: vi.fn(), disabled: true },
          { label: "First", onSelect: vi.fn() },
          { label: "Second", onSelect: vi.fn() },
        ],
      },
    });

    await fireEvent.click(getByRole("button", { name: "Actions" }));

    const first = getByRole("menuitem", { name: "First" });
    const second = getByRole("menuitem", { name: "Second" });

    expect(document.activeElement).toBe(first);

    await fireEvent.keyDown(window, { key: "ArrowDown" });
    expect(document.activeElement).toBe(second);

    await fireEvent.keyDown(window, { key: "ArrowUp" });
    expect(document.activeElement).toBe(first);
  });
});
