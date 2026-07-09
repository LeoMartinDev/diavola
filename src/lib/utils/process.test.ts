import { describe, it, expect } from "vitest";

import { processRowAction, processActionEnabled } from "./process";
import type { ProcessSnapshot } from "$lib/types";

function makeProcess(overrides: Partial<ProcessSnapshot> = {}): ProcessSnapshot {
  return {
    runtimeId: "r1",
    name: "api",
    kind: "service",
    status: "running",
    ...overrides,
  };
}

describe("processRowAction", () => {
  describe("for a service", () => {
    it.each(["running", "ready", "starting"] as const)(
      "returns 'stop' for status %s",
      (status) => {
        expect(processRowAction(makeProcess({ status }))).toBe("stop");
      },
    );

    it.each(["stopped", "failed", "succeeded"] as const)(
      "returns 'start' for status %s",
      (status) => {
        expect(processRowAction(makeProcess({ status }))).toBe("start");
      },
    );

    it.each(["stopping", "pending", "blocked"] as const)(
      "returns null for transitional status %s",
      (status) => {
        expect(processRowAction(makeProcess({ status }))).toBeNull();
      },
    );
  });

  describe("for a task", () => {
    it("returns null regardless of status", () => {
      for (const status of ["running", "stopped", "failed", "stopping"] as const) {
        expect(processRowAction(makeProcess({ kind: "task", status }))).toBeNull();
      }
    });
  });
});

describe("processActionEnabled", () => {
  it("is enabled when not busy and action is available", () => {
    expect(processActionEnabled(makeProcess({ status: "running" }), false)).toBe(true);
  });

  it("is disabled when busy even if action is available", () => {
    expect(processActionEnabled(makeProcess({ status: "running" }), true)).toBe(false);
  });

  it("is disabled for transitional statuses", () => {
    expect(processActionEnabled(makeProcess({ status: "stopping" }), false)).toBe(false);
  });
});
