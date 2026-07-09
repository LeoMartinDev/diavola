import { describe, it, expect } from "vitest";
import { validateConfigForm } from "$lib/config/validation";
import { createProcess } from "$lib/config/editorModel";

describe("config validation", () => {
  it("accepts a valid process", () => {
    const form = createProcess("api");
    form.cmd = "deno task dev";
    const result = validateConfigForm({ globalEnvRows: [], processes: [form] });
    expect(result.valid).toBe(true);
  });

  it("rejects an empty command", () => {
    const form = createProcess("api");
    form.cmd = "";
    const result = validateConfigForm({ globalEnvRows: [], processes: [form] });
    expect(result.issues.some((i) => i.key.endsWith(".cmd"))).toBe(true);
  });

  it("rejects duplicate process names", () => {
    const a = createProcess("api");
    const b = createProcess("api");
    a.cmd = "echo a";
    b.cmd = "echo b";
    const result = validateConfigForm({ globalEnvRows: [], processes: [a, b] });
    expect(result.issues.some((i) => i.key.endsWith(".name"))).toBe(true);
  });
});
