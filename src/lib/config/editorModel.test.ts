import { describe, it, expect } from "vitest";
import {
  buildConfig,
  createProcess,
  toProcessForm,
  serializeConfig,
} from "$lib/config/editorModel";

describe("serializeConfig", () => {
  it("serializes a basic config with a process", () => {
    const form = createProcess("api");
    form.cmd = "deno task dev";
    const config = buildConfig({ globalEnvRows: [], processes: [form] });
    expect(config.processes["api"]?.cmd).toBe("deno task dev");
    expect(config.processes["api"]?.kind).toBe("service");
  });

  it("omits optional fields when blank", () => {
    const form = createProcess("api");
    const config = buildConfig({ globalEnvRows: [], processes: [form] });
    expect(config.processes["api"]?.ready).toBeUndefined();
    expect(config.processes["api"]?.logEntryPattern).toBeUndefined();
  });

  it("round-trips through toProcessForm", () => {
    const form = createProcess("api");
    form.cmd = "npm start";
    form.envRows = [{ id: "e1", key: "PORT", value: "3000" }];
    const config = buildConfig({ globalEnvRows: [], processes: [form] });
    const back = toProcessForm("api", config.processes["api"]!);
    expect(back.cmd).toBe("npm start");
    expect(back.envRows[0].key).toBe("PORT");
  });

  it("serializes global logEntryPattern", () => {
    const form = createProcess("api");
    const config = buildConfig({ globalLogEntryPattern: "\\[\\d+:\\d+:\\d+\\]", globalEnvRows: [], processes: [form] });
    const yaml = serializeConfig(config);
    expect(yaml).toContain("logEntryPattern:");
  });
});
