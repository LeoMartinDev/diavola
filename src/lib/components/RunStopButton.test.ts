import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";

import RunStopButton from "./RunStopButton.svelte";

describe("RunStopButton", () => {
  describe("when inactive (no session running)", () => {
    it("renders a Run button", () => {
      const { getByRole, queryByRole } = render(RunStopButton, {
        props: { active: false, onRun: vi.fn(), onStop: vi.fn() },
      });

      expect(getByRole("button", { name: "Run project" })).toBeInTheDocument();
      expect(queryByRole("button", { name: "Stop current run" })).toBeNull();
    });

    it("calls onRun when clicked", async () => {
      const onRun = vi.fn();
      const { getByRole } = render(RunStopButton, {
        props: { active: false, onRun, onStop: vi.fn() },
      });

      await fireEvent.click(getByRole("button", { name: "Run project" }));

      expect(onRun).toHaveBeenCalledOnce();
    });

    it("is disabled when the disabled prop is true", () => {
      const { getByRole } = render(RunStopButton, {
        props: { active: false, disabled: true, onRun: vi.fn(), onStop: vi.fn() },
      });

      expect(getByRole("button", { name: "Run project" })).toBeDisabled();
    });

    it("shows a busy indicator instead of a Run button when busy", () => {
      const { getByLabelText, queryByRole } = render(RunStopButton, {
        props: { active: false, busy: true, onRun: vi.fn(), onStop: vi.fn() },
      });

      expect(getByLabelText("Busy")).toBeInTheDocument();
      expect(queryByRole("button", { name: "Run project" })).toBeNull();
    });
  });

  describe("when active (session running)", () => {
    it("renders a Stop button", () => {
      const { getByRole, queryByRole } = render(RunStopButton, {
        props: { active: true, onRun: vi.fn(), onStop: vi.fn() },
      });

      expect(getByRole("button", { name: "Stop current run" })).toBeInTheDocument();
      expect(queryByRole("button", { name: "Run project" })).toBeNull();
    });

    it("calls onStop when clicked", async () => {
      const onStop = vi.fn();
      const { getByRole } = render(RunStopButton, {
        props: { active: true, onRun: vi.fn(), onStop },
      });

      await fireEvent.click(getByRole("button", { name: "Stop current run" }));

      expect(onStop).toHaveBeenCalledOnce();
    });

    it("shows a busy indicator instead of a Stop button when busy", () => {
      const { getByLabelText, queryByRole } = render(RunStopButton, {
        props: { active: true, busy: true, onRun: vi.fn(), onStop: vi.fn() },
      });

      expect(getByLabelText("Busy")).toBeInTheDocument();
      expect(queryByRole("button", { name: "Stop current run" })).toBeNull();
    });
  });
});
