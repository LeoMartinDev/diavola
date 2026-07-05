import { describe, it, expect } from "vitest";
import { validateConfigForm } from "$lib/config/validation";
import { createProcess } from "$lib/config/editorModel";

describe("stopTimeoutMs validation", () => {
  it("rejects a sub-minimum value", () => {
    const form = createProcess("api");
    form.stopTimeoutMs = 500;
    const result = validateConfigForm({ globalStopTimeoutMs: null, globalEnvRows: [], processes: [form] });
    expect(result.issues.some((i) => i.key.endsWith(".stopTimeoutMs"))).toBe(true);
  });

  it("accepts a valid value", () => {
    const form = createProcess("api");
    form.stopTimeoutMs = 5000;
    const result = validateConfigForm({ globalStopTimeoutMs: null, globalEnvRows: [], processes: [form] });
    expect(result.issues.some((i) => i.key.endsWith(".stopTimeoutMs"))).toBe(false);
  });
});
