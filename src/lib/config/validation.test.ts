import { describe, it, expect } from "vitest";
import { validateConfigForm } from "$lib/config/validation";
import { createProcess } from "$lib/config/editorModel";

describe("gracePeriodMs validation", () => {
  it("rejects a sub-minimum value", () => {
    const form = createProcess("api");
    form.gracePeriodMs = 500;
    const result = validateConfigForm({ globalGracePeriodMs: null, globalEnvRows: [], processes: [form] });
    expect(result.issues.some((i) => i.key.endsWith(".gracePeriodMs"))).toBe(true);
  });

  it("accepts a valid value", () => {
    const form = createProcess("api");
    form.gracePeriodMs = 5000;
    const result = validateConfigForm({ globalGracePeriodMs: null, globalEnvRows: [], processes: [form] });
    expect(result.issues.some((i) => i.key.endsWith(".gracePeriodMs"))).toBe(false);
  });
});
