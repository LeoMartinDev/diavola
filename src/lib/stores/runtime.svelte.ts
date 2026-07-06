import { listen, type UnlistenFn } from "$lib/tauri/transport";
import { isDev } from "$lib/tauri/environment";
import { isPrimaryWindow } from "$lib/tauri/window";
import {
  checkForUpdate,
  relaunchApp,
  type PendingAppUpdate,
  type UpdaterDownloadEvent,
} from "$lib/tauri/updater";

  import {
    TAURI_EVENTS,
    closeTerminal,
    getLaunchProject,
    getSessionSnapshot,
    hasActiveSessions,
    listProjects,
    loadProjectConfig,
    openTerminal,
    resizeTerminal,
    restartProcess,
    saveProjectConfig,
    setWindowTitle,
    startProcess,
    startProject,
    stopProcess,
    stopProject,
    writeTerminal,
    type LaunchProjectInfo,
  } from "$lib/tauri/client";
  import { createGitPoller } from "$lib/utils/gitPoller";
import type {
  AppUpdateState,
  FlatRow,
  GitInfo,
  ProcessLogEvent,
  ProcessLogPayload,
  ProcessRuntimeId,
  ProjectConfigDocument,
  ProjectId,
  ProjectRecord,
  RunSessionSnapshot,
  SessionStatusEvent,
  TerminalEvent,
  TerminalOutputEvent,
  TerminalSessionId,
  TerminalSnapshot,
} from "$lib/types";

export const MAX_LOG_LINES_PER_PROCESS = 10_000;

export type Selection =
  | { kind: "process"; runtimeId: ProcessRuntimeId }
  | { kind: "terminal"; terminalId: TerminalSessionId }
  | null;

class RuntimeStore {
  projects = $state<ProjectRecord[]>([]);
  session = $state<RunSessionSnapshot | null>(null);
  terminals = $state<TerminalSnapshot[]>([]);
  projectId = $state<ProjectId | null>(null);
  launchLocked = $state<boolean>(false);
  selectedProcessRuntimeId = $state<ProcessRuntimeId | null>(null);
  selectedTerminalId = $state<TerminalSessionId | null>(null);
  processLogs = $state<Record<string, FlatRow[]>>({});
  processLogTruncation = $state<Record<string, number>>({});
  terminalOutput = $state<Record<string, string>>({});
  projectConfig = $state<ProjectConfigDocument | null>(null);
  busy = $state(false);
  gitInfo = $state<GitInfo | null>(null);
  appUpdate = $state<AppUpdateState>({ status: "idle" });
  logActions = $state<{ copy: () => void; clear: () => void } | null>(null);
  #gitPoller = createGitPoller();
  #focusHandler: (() => void) | null = null;
  #pendingUpdate: PendingAppUpdate | null = null;
  #updateInstallInFlight = false;

  #initialized = false;
  #unlisteners: UnlistenFn[] = [];

  #onGitInfo = (info: GitInfo | null) => {
    this.gitInfo = info;
  };

  get windowTitle(): string {
    const proj = this.project;
    if (!proj) return "Diavola";

    const relPath = this.gitInfo?.displayPath ?? proj.name;

    let context = "";
    if (this.gitInfo?.worktree) {
      context = ` — ${this.gitInfo.worktree}`;
    } else if (this.gitInfo?.branch) {
      context = ` — ${this.gitInfo.branch}`;
    }

    return `${relPath}${context} — Diavola`;
  }

  #syncGitPolling() {
    const dir = this.project?.baseDir;
    if (dir) {
      this.#gitPoller.start(dir, this.#onGitInfo);
    } else {
      this.#gitPoller.stop();
      this.gitInfo = null;
    }
  }

  #setProjectId(id: ProjectId | null) {
    this.projectId = id;
    this.#syncGitPolling();
  }

  #upsertProject(project: ProjectRecord) {
    const next = this.projects.filter((existing) => existing.id !== project.id);
    next.unshift(project);
    this.projects = next;
  }

  async init() {
    if (this.#initialized) {
      return;
    }
    this.#initialized = true;
    try {
      await this.refreshProjects();
    } catch (error) {
      console.error("[init] refreshProjects failed:", error);
    }
    try {
      this.session = await getSessionSnapshot();
    } catch (error) {
      console.error("[init] getSessionSnapshot failed:", error);
    }
    if (this.session?.projectId) {
      this.#setProjectId(this.session.projectId);
    }
    try {
      await this.#applyLaunchParams();
    } catch (error) {
      console.error("[init] applyLaunchParams failed:", error);
    }
    this.#focusHandler = async () => {
      const dir = this.project?.baseDir;
      if (dir) {
        const info = await this.#gitPoller.fetch(dir);
        this.#onGitInfo(info);
      }
    };
    window.addEventListener("focus", this.#focusHandler);
    this.syncProcessSelection();
    this.#syncGitPolling();
    await this.#attachEventListeners();
    try {
      await setWindowTitle(this.windowTitle);
    } catch {
      // window title permission may not be available yet
    }
    void this.#checkForAppUpdate();
  }

  async teardown() {
    this.#gitPoller.stop();
    this.gitInfo = null;
    this.appUpdate = { status: "idle" };
    this.#pendingUpdate = null;
    if (this.#focusHandler !== null) {
      window.removeEventListener("focus", this.#focusHandler);
      this.#focusHandler = null;
    }
    for (const unlisten of this.#unlisteners) {
      await unlisten();
    }
    this.#unlisteners = [];
    this.#initialized = false;
  }

  get project(): ProjectRecord | null {
    return this.projects.find((project) => project.id === this.projectId) ?? null;
  }

  get selectedProcess() {
    return (
      this.session?.processes.find(
        (process) => process.runtimeId === this.selectedProcessRuntimeId,
      ) ?? null
    );
  }

  get selectedTerminal(): TerminalSnapshot | null {
    return this.terminals.find((terminal) => terminal.terminalId === this.selectedTerminalId) ?? null;
  }

  get selection(): Selection {
    if (this.selectedProcessRuntimeId) {
      return { kind: "process", runtimeId: this.selectedProcessRuntimeId };
    }
    if (this.selectedTerminalId) {
      return { kind: "terminal", terminalId: this.selectedTerminalId };
    }
    return null;
  }

  logsForSelectedProcess(): FlatRow[] {
    return this.selectedProcessRuntimeId
      ? (this.processLogs[this.selectedProcessRuntimeId] ?? [])
      : [];
  }

  truncatedLogCountForSelectedProcess() {
    return this.selectedProcessRuntimeId
      ? (this.processLogTruncation[this.selectedProcessRuntimeId] ?? 0)
      : 0;
  }

  async #checkForAppUpdate() {
    // Only the primary window manages updates so multiple open windows don't
    // each independently check for and download the same update.
    if (!isPrimaryWindow() || isDev()) {
      return;
    }
    this.appUpdate = { status: "checking" };
    try {
      const update = await checkForUpdate();
      if (!update) {
        this.#pendingUpdate = null;
        this.appUpdate = { status: "idle" };
        return;
      }

      this.#pendingUpdate = update;
      let downloadedBytes = 0;
      let contentLength: number | null = null;
      this.appUpdate = {
        status: "downloading",
        version: update.version,
        downloadedBytes,
        contentLength,
      };

      await update.download((event: UpdaterDownloadEvent) => {
        if (event.event === "Started") {
          contentLength = event.data.contentLength ?? null;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength ?? 0;
        }

        if (event.event !== "Finished") {
          this.appUpdate = {
            status: "downloading",
            version: update.version,
            downloadedBytes,
            contentLength,
          };
        }
      });

      this.appUpdate = { status: "ready", version: update.version };
    } catch (error) {
      this.#pendingUpdate = null;
      this.appUpdate = {
        status: "error",
        message: error instanceof Error ? error.message : String(error),
      };
      console.error("[update] download failed:", error);
    }
  }

  async installDownloadedUpdate() {
    if (isDev()) {
      return;
    }
    if (this.appUpdate.status !== "ready" || this.#pendingUpdate === null) {
      return;
    }

    // Synchronous guard set before the first await: a fast double-click (or
    // any other re-entrant call) would otherwise see `status === "ready"`
    // twice and kick off two concurrent install/relaunch flows. The flag is
    // cleared in the finally block below so a failed attempt can be retried.
    if (this.#updateInstallInFlight) {
      return;
    }
    this.#updateInstallInFlight = true;

    try {
      // Installing/relaunching would otherwise silently orphan supervised
      // processes running in this window or any other open window. Fail safe:
      // refuse to install while any project session is still active anywhere.
      let activeElsewhere: boolean;
      try {
        activeElsewhere = await hasActiveSessions();
      } catch (error) {
        console.error("[update] install blocked: could not verify active sessions", error);
        return;
      }
      if (activeElsewhere) {
        console.warn("[update] install blocked: active sessions are still running");
        return;
      }

      const pendingUpdate = this.#pendingUpdate;
      const version = pendingUpdate.version;
      this.appUpdate = { status: "installing", version };
      try {
        await pendingUpdate.install();
      } catch (error) {
        this.appUpdate = { status: "ready", version };
        console.error("[update] install failed:", error);
        throw error;
      }

      try {
        await relaunchApp();
      } catch (error) {
        this.appUpdate = {
          status: "error",
          message: error instanceof Error ? error.message : String(error),
        };
        console.error("[update] relaunch failed after installing update:", error);
        throw error;
      }
    } finally {
      this.#updateInstallInFlight = false;
    }
  }

  async refreshProjects() {
    try {
      this.projects = await listProjects();
      if (this.launchLocked) {
        return;
      }
      if (!this.projectId && this.projects.length > 0) {
        this.#setProjectId(this.projects[0].id);
      }
      if (
        this.projectId &&
        !this.projects.some((project) => project.id === this.projectId)
      ) {
        this.#setProjectId(this.projects[0]?.id ?? null);
      }
    } catch {
      // ignore - refresh will retry on next focus
    }
  }

  async startCurrentProject() {
    if (!this.projectId || (this.session && !this.session.stoppedAt)) {
      return;
    }
    this.busy = true;
    this.processLogs = {};
    this.processLogTruncation = {};
    try {
      this.session = await startProject(this.projectId);
      this.#setProjectId(this.session.projectId);
      this.syncProcessSelection();
    } finally {
      this.busy = false;
    }
  }

  async stopCurrentProject() {
    this.busy = true;
    try {
      await stopProject();
      this.syncProcessSelection();
    } finally {
      this.busy = false;
    }
  }

  async restartSessionProcess(processName: string) {
    this.busy = true;
    try {
      this.session = await restartProcess(processName);
      this.syncProcessSelection();
    } catch {
      this.#markProcessFailed(processName);
    } finally {
      this.busy = false;
    }
  }

  async startSessionProcess(processName: string) {
    this.busy = true;
    try {
      this.session = await startProcess(processName);
      this.syncProcessSelection();
    } catch {
      this.#markProcessFailed(processName);
    } finally {
      this.busy = false;
    }
  }

  async stopSessionProcess(processName: string) {
    this.busy = true;
    try {
      this.session = await stopProcess(processName);
      this.syncProcessSelection();
    } finally {
      this.busy = false;
    }
  }

  async loadConfig(projectId = this.projectId, yaml?: string) {
    if (!projectId) {
      this.projectConfig = null;
      return null;
    }
    const document = await loadProjectConfig(projectId, yaml);
    if (!yaml) {
      this.projectConfig = document;
    }
    return document;
  }

  async saveConfig(yaml: string, projectId = this.projectId) {
    if (!projectId) {
      return null;
    }
    this.busy = true;
    try {
      const document = await saveProjectConfig(projectId, yaml);
      this.projectConfig = document;
      return document;
    } finally {
      this.busy = false;
    }
  }

  async openProjectTerminal(projectId = this.projectId, title?: string) {
    if (!projectId) {
      return null;
    }
    this.busy = true;
    try {
      const terminal = await openTerminal(projectId, title);
      this.upsertTerminal(terminal);
      this.selectedTerminalId = terminal.terminalId;
      this.selectedProcessRuntimeId = null;
      this.terminalOutput[terminal.terminalId] ??= "";
      return terminal;
    } finally {
      this.busy = false;
    }
  }

  async openTitledTerminal(projectId = this.projectId) {
    if (!projectId) {
      return null;
    }
    const openCount = this.terminals.filter((t) => t.isOpen).length;
    const title = openCount === 0 ? "bash" : `bash ${openCount + 1}`;
    return this.openProjectTerminal(projectId, title);
  }

  async closeSelectedTerminal() {
    if (!this.selectedTerminalId) {
      return;
    }
    this.busy = true;
    try {
      const closed = await closeTerminal(this.selectedTerminalId);
      if (closed) {
        this.upsertTerminal(closed);
      }
      this.selectedTerminalId =
        this.terminals.find((terminal) => terminal.isOpen)?.terminalId ?? null;
    } finally {
      this.busy = false;
    }
  }

  async writeToTerminal(data: string) {
    if (!this.selectedTerminalId) {
      return;
    }
    await writeTerminal(this.selectedTerminalId, data);
  }

  async resizeSelectedTerminal(cols: number, rows: number) {
    if (!this.selectedTerminalId) {
      return;
    }
    await resizeTerminal(this.selectedTerminalId, cols, rows);
  }

  clearSelectedProcessLogs() {
    if (!this.selectedProcessRuntimeId) {
      return;
    }
    this.processLogs[this.selectedProcessRuntimeId] = [];
    this.processLogTruncation[this.selectedProcessRuntimeId] = 0;
  }

  #markProcessFailed(processName: string) {
    if (!this.session) return;
    const processes = this.session.processes.map((p) =>
      p.name === processName ? { ...p, status: "failed" as const } : p
    );
    this.session = { ...this.session, processes };
  }

  selectProcess(runtimeId: ProcessRuntimeId) {
    this.selectedProcessRuntimeId = runtimeId;
    this.selectedTerminalId = null;
  }

  selectTerminal(terminalId: TerminalSessionId) {
    this.selectedTerminalId = terminalId;
    this.selectedProcessRuntimeId = null;
  }

  syncProcessSelection() {
    const processes = this.session?.processes ?? [];
    if (processes.length === 0) {
      this.selectedProcessRuntimeId = null;
      return;
    }
    if (
      this.selectedProcessRuntimeId &&
      processes.some((process) => process.runtimeId === this.selectedProcessRuntimeId)
    ) {
      return;
    }
    this.selectedProcessRuntimeId = processes[0]?.runtimeId ?? null;
  }

  upsertTerminal(snapshot: TerminalSnapshot) {
    const next = [...this.terminals];
    const index = next.findIndex((terminal) => terminal.terminalId === snapshot.terminalId);
    if (index >= 0) {
      next[index] = snapshot;
    } else {
      next.unshift(snapshot);
    }
    this.terminals = next;
  }

  #entryCounter = 0;

  #flattenPayload(payload: ProcessLogPayload): FlatRow[] {
    const entryId = this.#entryCounter++;
    return payload.lines.map((text, i) => ({
      entryId,
      lineIndex: i,
      isFirstLine: i === 0,
      isContinuation: i > 0,
      text,
      stream: payload.stream,
      timestamp: i === 0 ? payload.timestamp : '',
    }));
  }

  async #attachEventListeners() {
    this.#unlisteners.push(
      await listen<SessionStatusEvent>(TAURI_EVENTS.sessionSnapshot, (event) => {
        const snapshot = event.payload.snapshot;
        if (this.projectId && snapshot && snapshot.projectId !== this.projectId) {
          return;
        }
        if (snapshot) {
          this.#setProjectId(snapshot.projectId);
        }
        this.session = event.payload.snapshot;
        this.syncProcessSelection();
      }),
    );
    this.#unlisteners.push(
      await listen<ProcessLogEvent>(TAURI_EVENTS.processLog, (event) => {
        const payload = event.payload.payload;
        if (this.session && payload.sessionId !== this.session.sessionId) {
          return;
        }
        const current = this.processLogs[payload.runtimeId] ?? [];
        const newRows = this.#flattenPayload(payload);
        const appended = [...current, ...newRows];
        const overflow = Math.max(0, appended.length - MAX_LOG_LINES_PER_PROCESS);
        this.processLogs[payload.runtimeId] = overflow > 0 ? appended.slice(overflow) : appended;
        this.processLogTruncation[payload.runtimeId] =
          (this.processLogTruncation[payload.runtimeId] ?? 0) + overflow;
      }),
    );
    this.#unlisteners.push(
      await listen<TerminalOutputEvent>(TAURI_EVENTS.terminalOutput, (event) => {
        const payload = event.payload.payload;
        if (!this.terminals.some((terminal) => terminal.terminalId === payload.terminalId)) {
          return;
        }
        this.terminalOutput[payload.terminalId] =
          (this.terminalOutput[payload.terminalId] ?? "") + payload.chunk;
      }),
    );
    this.#unlisteners.push(
      await listen<TerminalEvent>(TAURI_EVENTS.terminalSnapshot, (event) => {
        this.upsertTerminal(event.payload.snapshot);
      }),
    );
  }

  async #applyLaunchParams() {
    if (typeof window === "undefined") {
      return;
    }
    const params = new URLSearchParams(window.location.search);
    const urlProjectId = params.get("projectId");
    const launchInfo = await getLaunchProject();
    this.launchLocked = launchInfo.locked;
    if (launchInfo.project) {
      this.#upsertProject(launchInfo.project);
    }

    if (launchInfo.locked && launchInfo.project) {
      this.#setProjectId(launchInfo.project.id);
      if (launchInfo.autoRun && this.session == null) {
        await this.startCurrentProject();
      }
      return;
    }

    const projectId = urlProjectId ?? launchInfo.project?.id ?? null;
    const autorun = params.get("autorun") === "1";
    if (!projectId || !this.projects.some((project) => project.id === projectId)) {
      return;
    }
    this.#setProjectId(projectId);
    if ((autorun || (launchInfo.autoRun && launchInfo.project?.id === projectId)) && !this.session) {
      await this.startCurrentProject();
    }
  }
}

export const runtimeStore = new RuntimeStore();
