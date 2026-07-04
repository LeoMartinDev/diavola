import { afterEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, waitFor, within } from "@testing-library/svelte";

import ConfigEditor from "./ConfigEditor.svelte";
import { runtimeStore } from "$lib/stores/runtime.svelte";
import type { ProjectConfigDocument, ProjectRecord } from "$lib/types";

const loadedProject: ProjectRecord = {
  id: "project-1",
  name: "Demo app",
  baseDir: "/tmp/demo-app",
  configSource: "projectFile",
  configPath: "/tmp/demo-app/trame.yml",
  createdAt: "2026-06-27T00:00:00Z",
  updatedAt: "2026-06-27T00:00:00Z",
};

const loadedDocument: ProjectConfigDocument = {
  project: loadedProject,
  yaml: "processes:\n  api:\n    kind: service\n    cmd: deno task dev\n  worker:\n    kind: task\n    cmd: deno task worker\n",
  config: {
    env: {
      GLOBAL_TOKEN: "demo",
    },
    processes: {
      api: {
        kind: "service",
        cmd: "deno task dev",
        env: {
          API_TOKEN: "demo",
        },
        dependsOn: {},
        ready: {
          type: "log",
          pattern: "ready",
          regex: false,
          timeoutMs: 60000,
        },
      },
      worker: {
        kind: "task",
        cmd: "deno task worker",
        env: {},
        dependsOn: {
          api: "ready",
        },
      },
    },
  },
};

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

function renderLoadedEditor() {
  vi.spyOn(runtimeStore, "loadConfig").mockResolvedValue(loadedDocument);

  return render(ConfigEditor, {
    props: {
      open: true,
      project: loadedProject,
      onClose: vi.fn(),
    },
  });
}

describe("ConfigEditor", () => {
  it("renders as a full page settings surface by default", () => {
    const { queryByRole, getByRole } = render(ConfigEditor, {
      props: {
        open: true,
        project: null,
        onClose: vi.fn(),
      },
    });

    expect(queryByRole("dialog")).toBeNull();
    expect(getByRole("link", { name: "Go back" })).toBeInTheDocument();
  });

  it("renders as a full page with a go back control", async () => {
    const onClose = vi.fn();
    const { getByRole, queryByRole } = render(ConfigEditor, {
      props: {
        open: true,
        project: null,
        onClose,
      },
    });

    expect(queryByRole("dialog")).toBeNull();

    const backLink = getByRole("link", { name: "Go back" });
    await fireEvent.click(backLink);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("shows a simple menu list with back as the first item", async () => {
    const { findByRole, getByRole, queryByText } = renderLoadedEditor();

    const navigation = getByRole("navigation", { name: "Menu sections" });
    const sidebar = navigation.closest("aside");
    if (!sidebar) {
      throw new Error("Sidebar should exist");
    }
    const menuItems = within(navigation).getAllByRole("link");

    await findByRole("heading", { name: "General" });

    expect(navigation).toBeInTheDocument();
    expect(menuItems[0]).toHaveAccessibleName("Go back");
    expect(menuItems[0]).toHaveClass("rounded-md");
    expect(menuItems[0]).toHaveClass("px-3");
    expect(menuItems[0]).toHaveClass("py-1.5");
    expect(menuItems[0]).toHaveClass("settings-nav-item");
    expect(menuItems[0]).toHaveClass("inline-flex");
    expect(menuItems[0]).toHaveClass("items-center");
    expect(menuItems[0]).toHaveClass("gap-2");
    expect(menuItems[0]).toHaveClass("text-[13px]");
    expect(menuItems[0]).toHaveClass("transition-colors");
    expect(menuItems[0]).toHaveClass("duration-75");
    expect(menuItems[0]).toHaveClass("text-text-subtle");
    expect(menuItems[0]).toHaveClass("hover:bg-surface-hover/70");
    expect(menuItems[0]).toHaveClass("hover:text-text");
    expect(menuItems[0]).not.toHaveClass("text-[12px]");
    expect(menuItems[0]).not.toHaveClass("py-1");
    const backIcon = menuItems[0].querySelector("svg");
    expect(backIcon).not.toBeNull();
    expect(backIcon?.getAttribute("width")).toBeTruthy();
    expect(getByRole("link", { name: "General" })).toBeInTheDocument();
    const environmentLink = getByRole("link", { name: "Environment" });
    expect(environmentLink).toBeInTheDocument();
    expect(environmentLink).toHaveClass("settings-nav-item");
    expect(getByRole("link", { name: "Processes" })).toBeInTheDocument();
    expect(within(sidebar).queryByText("Settings")).toBeNull();
    expect(within(sidebar).queryByText("Demo app")).toBeNull();
    expect(within(sidebar).queryByText("/tmp/demo-app")).toBeNull();
  });

  it("tracks the active settings section and scrolls to it from the left navigation", async () => {
    const { findByRole, getByRole } = render(ConfigEditor, {
      props: {
        open: true,
        project: null,
        onClose: vi.fn(),
      },
    });

    const generalLink = getByRole("link", { name: "General" });
    const processesLink = getByRole("link", { name: "Processes" });

    expect(generalLink.getAttribute("aria-current")).toBe("page");

    await fireEvent.click(processesLink);

    await findByRole("heading", { name: "Processes" });
    expect(processesLink.getAttribute("aria-current")).toBe("page");
    expect(generalLink.getAttribute("aria-current")).toBeNull();
  });

  it("does not render explicit save or cancel actions", () => {
    const { getByRole, queryByRole } = render(ConfigEditor, {
      props: {
        open: true,
        project: null,
        onClose: vi.fn(),
      },
    });

    expect(getByRole("link", { name: "Go back" })).toBeInTheDocument();
    expect(queryByRole("button", { name: "Save" })).toBeNull();
    expect(queryByRole("button", { name: "Cancel" })).toBeNull();
    expect(queryByRole("button", { name: "Save settings" })).toBeNull();
  });

  it("renders processes as a compact list with summary rows", async () => {
    const { findByRole, getByRole, queryByRole, queryByText } =
      renderLoadedEditor();

    await fireEvent.click(getByRole("link", { name: "Processes" }));
    const processesHeading = await findByRole("heading", { name: "Processes" });
    const processesSection = processesHeading.closest("section");
    if (!processesSection) {
      throw new Error("Processes section should exist");
    }
    const apiRow = getByRole("button", { name: "Open process api (service)" });
    const workerRow = getByRole("button", {
      name: "Open process worker (task)",
    });

    const newProcessButton = within(processesSection).getByRole("button", {
      name: "New",
    });
    expect(newProcessButton).toBeInTheDocument();
    expect(newProcessButton).not.toHaveClass("border");
    expect(newProcessButton.querySelector("svg")).not.toBeNull();
    expect(apiRow).toBeInTheDocument();
    expect(workerRow).toBeInTheDocument();
    expect(apiRow).toHaveTextContent("api");
    expect(apiRow).toHaveTextContent("service");
    expect(apiRow.querySelector("svg")).not.toBeNull();
    expect(workerRow.querySelector("svg")).not.toBeNull();
    expect(queryByRole("heading", { name: "Process: api" })).toBeNull();
  });

  it("opens a dedicated process detail view and allows navigating back to list", async () => {
    const { findByRole, getByRole, queryByRole, getByTestId } =
      renderLoadedEditor();

    await fireEvent.click(getByRole("link", { name: "Processes" }));
    await findByRole("heading", { name: "Processes" });
    await fireEvent.click(
      getByRole("button", { name: "Open process api (service)" }),
    );

    expect(getByRole("heading", { name: "api" })).toBeInTheDocument();
    expect(queryByRole("heading", { name: "Process: api" })).toBeNull();
    expect(getByRole("heading", { name: "Details" })).toBeInTheDocument();
    expect(getByRole("heading", { name: "Dependencies" })).toBeInTheDocument();
    expect(
      getByRole("button", { name: "Back to processes" }),
    ).toBeInTheDocument();
    expect(
      getByRole("navigation", { name: "Menu sections" }),
    ).toBeInTheDocument();
    expect(queryByRole("heading", { name: "General" })).toBeNull();
    expect(getByRole("heading", { name: "Environment" })).toBeInTheDocument();
    expect(queryByRole("heading", { name: "YAML preview" })).toBeNull();
    expect(queryByRole("button", { name: "Cancel" })).toBeNull();
    expect(queryByRole("button", { name: "Save" })).toBeNull();
    expect(getByTestId("process-detail-scroll")).toHaveClass("overflow-y-auto");
    expect(getByTestId("process-detail-scroll")).toHaveClass("h-full");

    await fireEvent.click(getByRole("button", { name: "Back to processes" }));

    expect(
      getByRole("button", { name: "Open process api (service)" }),
    ).toBeInTheDocument();
    expect(queryByRole("heading", { name: "api" })).toBeNull();
    expect(
      getByRole("navigation", { name: "Menu sections" }),
    ).toBeInTheDocument();
  });

  it("renders process detail as split cards under a page title", async () => {
    const { findByRole, getAllByTestId, getByRole, queryByRole } =
      renderLoadedEditor();

    await fireEvent.click(getByRole("link", { name: "Processes" }));
    await findByRole("heading", { name: "Processes" });
    await fireEvent.click(
      getByRole("button", { name: "Open process api (service)" }),
    );

    expect(getByRole("heading", { name: "api" })).toBeInTheDocument();
    expect(queryByRole("heading", { name: "Process: api" })).toBeNull();
    expect(getByRole("heading", { name: "Details" })).toBeInTheDocument();
    expect(getByRole("heading", { name: "Environment" })).toBeInTheDocument();
    expect(getByRole("heading", { name: "Dependencies" })).toBeInTheDocument();
    expect(getByRole("heading", { name: "Readiness" })).toBeInTheDocument();
    expect(getAllByTestId("process-detail-card")).toHaveLength(4);
  });

  it("auto-saves valid runtime config changes after a debounce", async () => {
    const saveConfig = vi.spyOn(runtimeStore, "saveConfig").mockResolvedValue({
      ...loadedDocument,
      yaml: "",
    });
    const { findByRole, getByRole, queryByRole, queryByText } =
      renderLoadedEditor();

    await findByRole("heading", { name: "General" });
    vi.useFakeTimers();

    await fireEvent.click(getByRole("link", { name: "Environment" }));
    await fireEvent.input(
      getByRole("textbox", { name: "Global environment variable value 1" }),
      {
        target: { value: "updated-token" },
      },
    );

    await vi.advanceTimersByTimeAsync(399);
    expect(saveConfig).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(1);

    expect(saveConfig).toHaveBeenCalledTimes(1);
    expect(saveConfig).toHaveBeenCalledWith(
      expect.stringContaining('GLOBAL_TOKEN: "updated-token"'),
      loadedProject.id,
    );
    expect(queryByText("Saving settings...")).toBeNull();
    expect(queryByText("Settings saved")).toBeNull();
    expect(queryByRole("button", { name: "Save" })).toBeNull();
    expect(queryByRole("button", { name: "Cancel" })).toBeNull();
  });

  it("serializes auto-saves so stale requests cannot overwrite newer config", async () => {
    let resolveFirstSave: ((document: ProjectConfigDocument) => void) | null =
      null;
    const firstSave = new Promise<ProjectConfigDocument>((resolve) => {
      resolveFirstSave = resolve;
    });
    const saveConfig = vi
      .spyOn(runtimeStore, "saveConfig")
      .mockReturnValueOnce(firstSave)
      .mockResolvedValue({
        ...loadedDocument,
        yaml: "",
      });
    const { findByRole, getByRole } = renderLoadedEditor();

    await findByRole("heading", { name: "General" });
    vi.useFakeTimers();

    await fireEvent.click(getByRole("link", { name: "Environment" }));
    const globalValue = getByRole("textbox", {
      name: "Global environment variable value 1",
    });
    await fireEvent.input(globalValue, {
      target: { value: "first-token" },
    });
    await vi.advanceTimersByTimeAsync(400);

    expect(saveConfig).toHaveBeenCalledTimes(1);
    expect(saveConfig).toHaveBeenLastCalledWith(
      expect.stringContaining('GLOBAL_TOKEN: "first-token"'),
      loadedProject.id,
    );

    await fireEvent.input(globalValue, {
      target: { value: "second-token" },
    });
    await vi.advanceTimersByTimeAsync(400);

    expect(saveConfig).toHaveBeenCalledTimes(1);

    resolveFirstSave?.({
      ...loadedDocument,
      yaml: "",
    });
    await vi.runAllTimersAsync();

    expect(saveConfig).toHaveBeenCalledTimes(2);
    expect(saveConfig).toHaveBeenLastCalledWith(
      expect.stringContaining('GLOBAL_TOKEN: "second-token"'),
      loadedProject.id,
    );
  });

  it("does not auto-save invalid runtime config changes", async () => {
    const saveConfig = vi.spyOn(runtimeStore, "saveConfig").mockResolvedValue({
      ...loadedDocument,
      yaml: "",
    });
    const { findByRole, getAllByRole, getByRole, getByText } =
      renderLoadedEditor();

    await findByRole("heading", { name: "General" });
    vi.useFakeTimers();

    await fireEvent.click(getByRole("link", { name: "Processes" }));
    await fireEvent.click(
      getByRole("button", { name: "Open process worker (task)" }),
    );
    await findByRole("heading", { name: "worker" });
    await fireEvent.input(getAllByRole("textbox", { name: "Name" })[0], {
      target: { value: "api" },
    });
    await vi.advanceTimersByTimeAsync(400);

    expect(saveConfig).not.toHaveBeenCalled();
    expect(getByText("Process names must be unique.")).toBeInTheDocument();
  });

  it("keeps environment add action at section titles without nested env subtitles", async () => {
    const { findByRole, getByRole, queryByRole } = renderLoadedEditor();

    await fireEvent.click(getByRole("link", { name: "Environment" }));
    const globalEnvironmentHeading = await findByRole("heading", {
      name: "Environment",
    });
    const globalEnvironmentSection =
      globalEnvironmentHeading.closest("section");
    if (!globalEnvironmentSection) {
      throw new Error("Environment section should exist");
    }

    expect(
      within(globalEnvironmentSection).getByRole("button", { name: "New" }),
    ).toBeInTheDocument();
    expect(
      within(globalEnvironmentSection).queryByRole("heading", {
        name: "Environment variables",
      }),
    ).toBeNull();

    await fireEvent.click(getByRole("link", { name: "Processes" }));
    await fireEvent.click(
      getByRole("button", { name: "Open process api (service)" }),
    );

    const processEnvironmentHeading = await findByRole("heading", {
      name: "Environment",
    });
    const processEnvironmentSection = processEnvironmentHeading.closest("div");
    if (!processEnvironmentSection) {
      throw new Error("Process environment section should exist");
    }

    expect(
      within(processEnvironmentSection).getByRole("button", { name: "New" }),
    ).toBeInTheDocument();
    expect(
      queryByRole("heading", { name: "Environment variables" }),
    ).toBeNull();
  });

  it("does not render YAML preview controls in settings mode", async () => {
    const { findByRole, queryByRole, queryByText } = renderLoadedEditor();

    await findByRole("heading", { name: "General" });

    expect(queryByRole("link", { name: "YAML preview" })).toBeNull();
    expect(queryByRole("heading", { name: "YAML preview" })).toBeNull();
    expect(queryByRole("button", { name: "Preview YAML" })).toBeNull();
    expect(queryByText("Generated output")).toBeNull();
  });
});
