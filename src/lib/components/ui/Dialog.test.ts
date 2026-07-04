import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, waitFor } from "@testing-library/svelte";

import Dialog from "./Dialog.svelte";

describe("Dialog", () => {
  it("renders the panel presentation for utility popups", () => {
    const { getByRole } = render(Dialog, {
      props: {
        open: true,
        title: "Runtime configuration",
        onClose: vi.fn(),
        variant: "panel",
      },
    });

    const dialog = getByRole("dialog");

    expect(dialog.getAttribute("data-dialog-variant")).toBe("panel");
    expect(dialog.className).toContain("sm:h-[calc(100vh-56px)]");
  });

  it("keeps panel popups non-modal and click-through outside the panel", () => {
    const { getByRole, queryByLabelText } = render(Dialog, {
      props: {
        open: true,
        title: "Runtime configuration",
        onClose: vi.fn(),
        variant: "panel",
      },
    });

    const dialog = getByRole("dialog");
    const shell = dialog.parentElement;

    expect(queryByLabelText("Close dialog")).toBeNull();
    expect(dialog.getAttribute("aria-modal")).toBeNull();
    expect(shell?.className).toContain("pointer-events-none");
    expect(dialog.className).toContain("pointer-events-auto");
  });

  it("adds an in-panel close control without stealing initial focus", async () => {
    const onClose = vi.fn();
    const { getByRole } = render(Dialog, {
      props: {
        open: true,
        title: "Runtime configuration",
        onClose,
        variant: "panel",
      },
    });

    const dialog = getByRole("dialog");
    const closeButton = getByRole("button", { name: "Close panel" });

    await waitFor(() => {
      expect(document.activeElement).toBe(dialog);
    });

    await fireEvent.click(closeButton);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});