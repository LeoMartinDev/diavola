import { describe, it, expect } from "vitest";
import {
  buildConfig,
  createProcess,
  toProcessForm,
  serializeConfig,
} from "$lib/config/editorModel";

describe("stopTimeoutMs round-trip", () => {
  it("emits stopTimeoutMs on a process when set", () => {
    const form = createProcess("api");
    form.stopTimeoutMs = 30000;
    const config = buildConfig({ globalEnvRows: [], processes: [form] });
    expect(config.processes["api"]?.stopTimeoutMs).toBe(30000);
  });

  it("omits stopTimeoutMs when blank", () => {
    const form = createProcess("api");
    form.stopTimeoutMs = "";
    const config = buildConfig({ globalStopTimeoutMs: null, globalEnvRows: [], processes: [form] });
    expect(config.processes["api"]?.stopTimeoutMs).toBeUndefined();
  });

  it("round-trips through toProcessForm", () => {
    const form = createProcess("api");
    form.stopTimeoutMs = 7000;
    const config = buildConfig({ globalStopTimeoutMs: null, globalEnvRows: [], processes: [form] });
    const back = toProcessForm("api", config.processes["api"]!);
    expect(back.stopTimeoutMs).toBe(7000);
  });

  it("serializes global stopTimeoutMs", () => {
    const form = createProcess("api");
    const config = buildConfig({ globalStopTimeoutMs: 12000, globalEnvRows: [], processes: [form] });
    const yaml = serializeConfig(config);
    expect(yaml).toContain("stopTimeoutMs: 12000");
  });
});
