import {
  buildConfig as buildConfigFromForm,
  createProcess,
  serializeConfig,
  toProcessForm,
  type ConfigFormState,
  type EnvRow,
  type ProcessForm as ProcessFormState,
} from "$lib/config/editorModel";
import {
  validateConfigForm,
  type ValidationIssue,
} from "$lib/config/validation";
import { runtimeStore } from "$lib/stores/runtime.svelte";
import type { ProjectRecord } from "$lib/types";

type PendingConfigSave = {
  projectId: string;
  yaml: string;
};

export function useConfigEditor(
  project: () => ProjectRecord | null,
  open: () => boolean,
) {
  let pendingSave: PendingConfigSave | null = null;
  let saveInFlight = false;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  const s = $state({
    globalEnvRows: [] as EnvRow[],
    globalGracePeriodMs: null as number | string | null,
    globalLogTimestampPattern: "" as string,
    processes: [] as ProcessFormState[],
    selectedProcessId: null as string | null,
    loadedProjectId: null as string | null,
    loading: false,
    saving: false,
    status: null as string | null,
    loadError: null as string | null,
    dialogWasOpen: false,
    validationIssues: [] as ValidationIssue[],
    suppressDirty: false,
    touchedFields: new Set<string>(),
    lastSavedYaml: null as string | null,
    activeSection: "settings-general" as string,
    processesViewMode: "list" as "list" | "detail",

    get selectedProcess(): ProcessFormState | null {
      return s.processes.find((p) => p.id === s.selectedProcessId) ?? s.processes[0] ?? null;
    },
    get formState(): ConfigFormState {
      return { globalEnvRows: s.globalEnvRows, globalGracePeriodMs: s.globalGracePeriodMs, globalLogTimestampPattern: s.globalLogTimestampPattern, processes: s.processes };
    },
    get formIssueCount(): number {
      return s.validationIssues.length;
    },
    get statusIsError(): boolean {
      return s.status !== null &&
        s.status !== "Settings saved" &&
        s.status !== "Saving settings...";
    },
    get projectSourceLabel(): string {
      return !project()
        ? "Auto-detect"
        : project()!.configSource === "projectFile"
          ? "Project file"
          : "App config file";
    },
  });

  function nextId(prefix: string) {
    return `${prefix}-${crypto.randomUUID()}`;
  }

  function newProcess(name = "api") {
    return createProcess(name, nextId);
  }

  function currentYaml() {
    return serializeConfig(
      buildConfigFromForm({ globalEnvRows: s.globalEnvRows, globalGracePeriodMs: s.globalGracePeriodMs, globalLogTimestampPattern: s.globalLogTimestampPattern, processes: s.processes }),
    );
  }

  function resetEmpty() {
    s.globalEnvRows = [];
    s.globalGracePeriodMs = null;
    s.globalLogTimestampPattern = "";
    s.processes = [newProcess("api")];
    s.selectedProcessId = s.processes[0].id;
    s.processesViewMode = "list";
  }

  function resetUnloaded() {
      s.globalEnvRows = [];
    s.globalGracePeriodMs = null;
    s.globalLogTimestampPattern = "";
    s.processes = [];
    s.selectedProcessId = null;
    s.processesViewMode = "list";
  }

  function errorMessage(error: unknown) {
    if (error instanceof Error) return error.message;
    if (typeof error === "string") return error;
    return "Unable to load the project configuration.";
  }

  async function load(projectId: string) {
    s.loading = true;
    s.status = null;
    s.loadError = null;
    s.validationIssues = [];
    s.touchedFields = new Set();
    try {
      const document = await runtimeStore.loadConfig(projectId);
      const config = document?.config;
      if (!config) {
        s.suppressDirty = true;
        resetEmpty();
        s.lastSavedYaml = currentYaml();
        s.suppressDirty = false;
        s.loadedProjectId = projectId;
        return;
      }
      s.suppressDirty = true;
      s.globalGracePeriodMs = config.gracePeriodMs ?? null;
      s.globalLogTimestampPattern = config.logTimestampPattern ?? "";
      s.globalEnvRows = Object.entries(config.env ?? {}).map(([key, value]) => ({
        id: nextId("env"),
        key,
        value,
      }));
      s.processes = Object.entries(config.processes ?? {}).map(
        ([name, process]) => toProcessForm(name, process, nextId),
      );
      if (s.processes.length === 0) {
        s.processes = [newProcess("api")];
      }
      s.selectedProcessId = s.processes[0].id;
      s.processesViewMode = "list";
      s.lastSavedYaml = currentYaml();
      s.suppressDirty = false;
      s.loadedProjectId = projectId;
    } catch (error) {
      s.loadError = errorMessage(error);
      s.loadedProjectId = null;
      s.lastSavedYaml = null;
      resetUnloaded();
    } finally {
      s.loading = false;
    }
  }

  function addEnvRow(process: ProcessFormState) {
    process.envRows = [...process.envRows, { id: nextId("env"), key: "", value: "" }];
  }

  function removeEnvRow(process: ProcessFormState, rowId: string) {
    process.envRows = process.envRows.filter((row) => row.id !== rowId);
  }

  function addGlobalEnvRow() {
    s.globalEnvRows = [...s.globalEnvRows, { id: nextId("env"), key: "", value: "" }];
  }

  function removeGlobalEnvRow(rowId: string) {
    s.globalEnvRows = s.globalEnvRows.filter((row) => row.id !== rowId);
  }

  function addProcess() {
    const name = uniqueProcessName("process");
    const proc = newProcess(name);
    s.processes = [...s.processes, proc];
    s.selectedProcessId = proc.id;
    s.processesViewMode = "detail";
  }

  function removeProcess(id: string) {
    const removedName = s.processes.find((p) => p.id === id)?.name;
    s.processes = s.processes.filter((p) => p.id !== id);
    if (removedName) {
      for (const p of s.processes) {
        p.dependencies = p.dependencies.filter((d) => d.processName !== removedName);
      }
    }
    s.selectedProcessId = s.processes[0]?.id ?? null;
    if (s.processes.length === 0) s.processesViewMode = "list";
  }

  function uniqueProcessName(base: string) {
    const used = new Set(s.processes.map((p) => p.name));
    if (!used.has(base)) return base;
    let index = 2;
    while (used.has(`${base}-${index}`)) index += 1;
    return `${base}-${index}`;
  }

  function addDependency(process: ProcessFormState) {
    const target = s.processes.find((c) => c.id !== process.id);
    process.dependencies = [
      ...process.dependencies,
      { id: nextId("dependency"), processName: target?.name ?? "", condition: "ready" as const },
    ];
  }

  function removeDependency(process: ProcessFormState, depId: string) {
    process.dependencies = process.dependencies.filter((d) => d.id !== depId);
  }

  async function drainSaveQueue() {
    if (saveInFlight) return;
    saveInFlight = true;
    s.saving = true;
    try {
      while (pendingSave) {
        const save = pendingSave;
        pendingSave = null;
        s.status = "Saving settings...";
        try {
          await runtimeStore.saveConfig(save.yaml, save.projectId);
          if (s.loadedProjectId === save.projectId) {
            s.lastSavedYaml = save.yaml;
            s.status = "Settings saved";
          }
        } catch (error) {
          if (s.loadedProjectId === save.projectId) {
            s.status = errorMessage(error);
          }
        }
      }
    } finally {
      saveInFlight = false;
      s.saving = false;
    }
  }

  function autoSave(projectId: string) {
    if (s.loadError) return;
    const validation = validateConfigForm(s.formState);
    s.validationIssues = validation.issues;
    if (!validation.valid) {
      for (const issue of s.validationIssues) s.touchedFields.add(issue.key);
      s.status = "Fix validation issues to save settings.";
      return;
    }
    const yaml = serializeConfig(buildConfigFromForm(s.formState));
    if (yaml === s.lastSavedYaml) return;
    pendingSave = { projectId, yaml };
    void drainSaveQueue();
  }

  function isTouched(key: string) { return s.touchedFields.has(key); }

  function issueFor(key: string) {
    const issue = s.validationIssues.find((i) => i.key === key);
    if (!issue || !isTouched(key)) return null;
    return issue.message;
  }

  function processIssue(process: ProcessFormState, field: string) {
    return issueFor(`process.${process.id}.${field}`);
  }

  function readyIssue(process: ProcessFormState, field: string) {
    return issueFor(`process.${process.id}.ready.${field}`);
  }

  function dependencyIssue(process: ProcessFormState, depId: string) {
    return issueFor(`process.${process.id}.dependency.${depId}`);
  }

  function envRowIssue(process: ProcessFormState, rowId: string) {
    return issueFor(`process.${process.id}.env.${rowId}.key`);
  }

  function handleNav(sectionId: string) {
    s.activeSection = sectionId;
    if (sectionId !== "settings-processes") s.processesViewMode = "list";
  }

  function processOptionId(processId: string) { return `settings-process-option-${processId}`; }
  function processPanelId(processId: string) { return `settings-process-panel-${processId}`; }

  function focusProcessOption(processId: string) {
    if (typeof document === "undefined") return;
    document.getElementById(processOptionId(processId))?.focus();
  }

  function selectProcessByIndex(index: number) {
    const proc = s.processes[index];
    if (!proc) return;
    s.selectedProcessId = proc.id;
    focusProcessOption(proc.id);
  }

  function handleProcessOptionKeydown(event: KeyboardEvent, processId: string) {
    const idx = s.processes.findIndex((p) => p.id === processId);
    if (idx === -1) return;
    if (event.key === "ArrowDown" || event.key === "ArrowRight") {
      event.preventDefault();
      selectProcessByIndex((idx + 1) % s.processes.length);
    } else if (event.key === "ArrowUp" || event.key === "ArrowLeft") {
      event.preventDefault();
      selectProcessByIndex((idx - 1 + s.processes.length) % s.processes.length);
    } else if (event.key === "Home") {
      event.preventDefault();
      selectProcessByIndex(0);
    } else if (event.key === "End") {
      event.preventDefault();
      selectProcessByIndex(s.processes.length - 1);
    }
  }

  function markTouched(key: string) { s.touchedFields.add(key); }

  function globalGracePeriodError() {
    return issueFor("global.gracePeriodMs");
  }

  function onGlobalGracePeriodChange(value: number | string | null) {
    s.globalGracePeriodMs = value;
  }

  return Object.assign(s, {
    nextId,
    newProcess,
    resetEmpty,
    resetUnloaded,
    errorMessage,
    currentYaml,
    load,
    addEnvRow,
    removeEnvRow,
    addGlobalEnvRow,
    removeGlobalEnvRow,
    addProcess,
    removeProcess,
    uniqueProcessName,
    addDependency,
    removeDependency,
    drainSaveQueue,
    autoSave,
    isTouched,
    issueFor,
    processIssue,
    readyIssue,
    dependencyIssue,
    envRowIssue,
    handleNav,
    processOptionId,
    processPanelId,
    focusProcessOption,
    selectProcessByIndex,
    handleProcessOptionKeydown,
    markTouched,
    globalGracePeriodError,
    onGlobalGracePeriodChange,
    get debounceTimer(): ReturnType<typeof setTimeout> | null { return debounceTimer; },
    set debounceTimer(v: ReturnType<typeof setTimeout> | null) { debounceTimer = v; },
  });
}
