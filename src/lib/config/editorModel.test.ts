import { describe, it, expect } from "vitest";
import {
  buildConfig,
  createProcess,
  toProcessForm,
  serializeConfig,
} from "$lib/config/editorModel";

describe("gracePeriodMs round-trip", () => {
  it("emits gracePeriodMs on a process when set", () => {
    const form = createProcess("api");
    form.gracePeriodMs = 30000;
    const config = buildConfig({ globalEnvRows: [], processes: [form] });
    expect(config.processes["api"]?.gracePeriodMs).toBe(30000);
  });

  it("omits gracePeriodMs when blank", () => {
    const form = createProcess("api");
    form.gracePeriodMs = "";
    const config = buildConfig({ globalGracePeriodMs: null, globalEnvRows: [], processes: [form] });
    expect(config.processes["api"]?.gracePeriodMs).toBeUndefined();
  });

  it("round-trips through toProcessForm", () => {
    const form = createProcess("api");
    form.gracePeriodMs = 7000;
    const config = buildConfig({ globalGracePeriodMs: null, globalEnvRows: [], processes: [form] });
    const back = toProcessForm("api", config.processes["api"]!);
    expect(back.gracePeriodMs).toBe(7000);
  });

  it("serializes global gracePeriodMs", () => {
    const form = createProcess("api");
    const config = buildConfig({ globalGracePeriodMs: 12000, globalEnvRows: [], processes: [form] });
    const yaml = serializeConfig(config);
    expect(yaml).toContain("gracePeriodMs: 12000");
  });
});
