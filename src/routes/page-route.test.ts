import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/svelte";

vi.mock("$lib/tauri/client", async () => {
  const actual = await vi.importActual<typeof import("$lib/tauri/client")>("$lib/tauri/client");
  return {
    ...actual,
    setWindowTitle: vi.fn(async () => undefined),
  };
});

import Page from "./+page.svelte";
import { runtimeStore } from "$lib/stores/runtime.svelte";

describe("workspace empty state", () => {
  beforeEach(() => {
    vi.spyOn(runtimeStore, "init").mockResolvedValue();
    vi.spyOn(runtimeStore, "teardown").mockResolvedValue();
    runtimeStore.projects = [];
    runtimeStore.projectId = null;
    runtimeStore.session = null;
    runtimeStore.terminals = [];
    runtimeStore.selectedProcessRuntimeId = null;
    runtimeStore.selectedTerminalId = null;
    runtimeStore.uiError = null;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("shows a settings hint instead of the welcome screen when no config is loaded", () => {
    const { queryByText, getByText } = render(Page);

    expect(queryByText("Welcome to Diavola")).toBeNull();
    expect(getByText(/Open Runtime config in Settings to add processes/)).toBeInTheDocument();
  });
});
