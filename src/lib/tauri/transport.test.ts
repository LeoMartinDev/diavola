import { describe, expect, it } from "vitest";

import { invoke, listen } from "./transport";

describe("transport seam", () => {
  it("uses the browser mock transport when tauri internals are absent", async () => {
    await expect(invoke("get_session_snapshot")).resolves.toMatchObject({
      sessionId: "mock-session-deno-runner",
      projectId: "deno-runner",
    });

    const handler = () => {};
    await expect(listen("process-log", handler)).resolves.toEqual(expect.any(Function));
  });
});
