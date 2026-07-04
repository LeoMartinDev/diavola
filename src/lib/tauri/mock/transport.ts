// Browser-only mock implementation of the Tauri transport.
//
// Loaded exclusively via a dynamic import (see $lib/tauri/transport.ts) when the
// Tauri internals are not present, i.e. when `deno task dev` runs the frontend
// in a plain browser. It implements the same `invoke` / `listen` contract the
// store and client rely on, backed by static fixtures and an in-memory event
// bus, so the UI can be exercised end-to-end without the Rust runtime.

import { parse } from "yaml";

import type {
  ProjectConfigDocument,
  ProjectId,
  ProjectRecord,
  RunSessionSnapshot,
  TerminalSnapshot,
} from "$lib/types";
import {
  MOCK_TERMINAL_BANNER,
  mockConfigDocument,
  mockProjectFixture,
  mockProjectRecord,
  mockProjects,
  mockTerminalSnapshot,
  type MockProcessLogs,
} from "./data";

// ---------------------------------------------------------------------------
// Types mirroring the Tauri transport contract
// ---------------------------------------------------------------------------

/**
 * Event callback signature, matching Tauri's `listen`.
 * The handler receives an object shaped like Tauri's `Event<T>`.
 */
type MockEventHandler<T = unknown> = (event: { event: string; id: number; payload: T }) => void;

type MockUnlistenFn = () => void;

export type { MockUnlistenFn as UnlistenFn };

// ---------------------------------------------------------------------------
// In-memory event bus
// ---------------------------------------------------------------------------

const listeners: { [event: string]: Array<MockEventHandler> } = {};
let nextEventId = 0;

function dispatch<T>(event: string, payload: T): void {
  const handlers = listeners[event];
  if (!handlers) {
    return;
  }
  const id = nextEventId++;
  // Defer so callers (e.g. startProject) can settle their state — such as the
  // store assigning this.session — before the event handler runs, mirroring the
  // async nature of real Tauri events.
  queueMicrotask(() => {
    for (const handler of [...handlers]) {
      handler({ event, id, payload });
    }
  });
}

function subscribe<T>(event: string, handler: MockEventHandler<T>): MockUnlistenFn {
  const list = listeners[event] ?? (listeners[event] = []);
  list.push(handler as MockEventHandler);
  return () => {
    const index = list.indexOf(handler as MockEventHandler);
    if (index >= 0) {
      list.splice(index, 1);
    }
  };
}

// ---------------------------------------------------------------------------
// Mutable mock state
// ---------------------------------------------------------------------------

// Seed an active session immediately so the UI shows processes and logs as soon
// as the app loads, without the user having to hit Start. The store picks this
// up via getSessionSnapshot() during init().
let currentSession: RunSessionSnapshot | null = cloneSession(
  mockProjects[0].sessionTemplate,
);
let terminalCounter = 0;
let defaultLogsEmitted = false;

/**
 * Emits the seeded session's canned logs the first time it runs. Called when the
 * store registers its process-log listener, so the events land after the
 * listener is attached (dispatch defers via microtask). The store's session-id
 * guard accepts them because getSessionSnapshot() already set this.session.
 */
function emitDefaultLogs(): void {
  if (defaultLogsEmitted || !currentSession) {
    return;
  }
  const fixture = mockProjectFixture(currentSession.projectId);
  if (!fixture) {
    return;
  }
  defaultLogsEmitted = true;
  emitLogs(fixture.logs, currentSession.sessionId);
}

function allProjects(): ProjectRecord[] {
  return mockProjects.map((project) => project.record);
}

function cloneSession(session: RunSessionSnapshot): RunSessionSnapshot {
  return {
    ...session,
    processes: session.processes.map((process) => ({ ...process })),
  };
}

function emitLogs(logs: MockProcessLogs, sessionId: string): void {
  for (const [runtimeId, entries] of Object.entries(logs)) {
    for (const entry of entries) {
      dispatch("process-log", { payload: { ...entry, sessionId } });
    }
  }
}

// ---------------------------------------------------------------------------
// invoke
// ---------------------------------------------------------------------------

/**
 * Returns the nested `request` payload when present (Tauri's wrapper convention
 * used by the real commands), otherwise treats `args` itself as the payload.
 */
function requestOf(args: Record<string, unknown> | undefined): Record<string, unknown> {
  if (!args) {
    return {};
  }
  const request = args.request;
  return typeof request === "object" && request !== null
    ? (request as Record<string, unknown>)
    : args;
}

export async function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  switch (cmd) {
    case "list_projects":
      return allProjects() as unknown as T;

    case "get_launch_project":
      return {
        project: mockProjects[0]?.record ?? null,
        locked: true,
        autoRun: true,
        error: null,
      } as unknown as T;

    case "get_session_snapshot":
      return currentSession as unknown as T;

    case "has_active_sessions":
      return (!!currentSession && !currentSession.stoppedAt) as unknown as T;

    case "load_project_config": {
      const request = requestOf(args) as { projectId: ProjectId; yaml?: string };
      if (request.yaml !== undefined) {
        // Raw-mode preview: parse the provided YAML and echo it back.
        const record = mockProjectRecord(request.projectId) ??
          allProjects().find((project) => project.id === request.projectId) ??
          projectFallback(request.projectId);
        const config = parse(request.yaml);
        const document: ProjectConfigDocument = {
          project: record,
          yaml: request.yaml,
          config,
        };
        return document as unknown as T;
      }
      const document = mockConfigDocument(request.projectId);
      if (!document) {
        throw new Error(`mock: unknown project "${request.projectId}"`);
      }
      return document as unknown as T;
    }

    case "save_project_config": {
      const request = requestOf(args) as { projectId: ProjectId; yaml: string };
      const record = mockProjectRecord(request.projectId) ??
        allProjects().find((project) => project.id === request.projectId) ??
        projectFallback(request.projectId);
      const config = parse(request.yaml);
      const document: ProjectConfigDocument = {
        project: record,
        yaml: request.yaml,
        config,
      };
      return document as unknown as T;
    }

    case "start_project": {
      const request = requestOf(args) as { projectId: ProjectId };
      const fixture = mockProjectFixture(request.projectId);
      if (!fixture) {
        throw new Error(`mock: cannot start unknown project "${request.projectId}"`);
      }
      currentSession = cloneSession(fixture.sessionTemplate);
      // Emit canned logs on the next tick so the store has time to register the
      // session before the process-log listener's session guard runs.
      setTimeout(() => emitLogs(fixture.logs, currentSession!.sessionId), 0);
      return currentSession as unknown as T;
    }

    case "stop_project": {
      if (currentSession) {
        currentSession = {
          ...currentSession,
          stoppedAt: new Date().toISOString(),
          processes: currentSession.processes.map((process) => ({
            ...process,
            status: process.status === "succeeded" || process.status === "failed"
              ? process.status
              : "stopped",
          })),
        };
        dispatch("session-snapshot", { snapshot: currentSession });
      }
      return currentSession as unknown as T;
    }

    case "restart_process": {
      const request = requestOf(args) as { processName: string };
      return mutateProcess(request.processName, (process) => {
        process.status = process.kind === "task" ? "starting" : "running";
        process.startedAt = new Date().toISOString();
        process.exitedAt = undefined;
        process.exitCode = undefined;
      }) as unknown as T;
    }

    case "stop_process": {
      const request = requestOf(args) as { processName: string };
      return mutateProcess(request.processName, (process) => {
        if (process.status === "succeeded" || process.status === "failed") {
          return;
        }
        process.status = "stopped";
        process.exitedAt = new Date().toISOString();
      }) as unknown as T;
    }

    case "open_terminal": {
      const request = requestOf(args) as { projectId: ProjectId; title?: string };
      const terminalId = `mock-terminal-${++terminalCounter}`;
      const snapshot = {
        ...mockTerminalSnapshot(terminalId),
        title: request.title ?? `mock terminal (${terminalId})`,
      };
      dispatch("terminal-snapshot", { snapshot });
      setTimeout(() => {
        dispatch("terminal-output", {
          payload: {
            terminalId,
            chunk: MOCK_TERMINAL_BANNER,
            timestamp: new Date().toISOString(),
          },
        });
      }, 0);
      return snapshot as unknown as T;
    }

    case "write_terminal":
    case "resize_terminal":
      // Static terminal: accept the call but produce no output.
      return undefined as unknown as T;

    case "close_terminal": {
      const request = requestOf(args) as { terminalId: string };
      const snapshot: TerminalSnapshot = {
        terminalId: request.terminalId,
        title: `mock terminal (${request.terminalId})`,
        cwd: "/home/leo/dev/trame",
        createdAt: new Date().toISOString(),
        isOpen: false,
      };
      dispatch("terminal-snapshot", { snapshot });
      return snapshot as unknown as T;
    }

    default:
      throw new Error(`mock: unhandled invoke command "${cmd}"`);
  }
}

function mutateProcess(
  processName: string,
  mutate: (process: RunSessionSnapshot["processes"][number]) => void,
): RunSessionSnapshot | null {
  if (!currentSession) {
    return null;
  }
  currentSession = cloneSession(currentSession);
  const target = currentSession.processes.find((process) => process.name === processName);
  if (target) {
    mutate(target);
  }
  dispatch("session-snapshot", { snapshot: currentSession });
  return currentSession;
}

function projectFallback(projectId: ProjectId): ProjectRecord {
  const existing = allProjects().find((project) => project.id === projectId);
  if (existing) {
    return existing;
  }
  // A project created at runtime has no fixture config; synthesise a minimal
  // record so the config editor still has something to anchor on.
  const now = new Date().toISOString();
  return {
    id: projectId,
    name: projectId,
    baseDir: `/home/leo/dev/${projectId}`,
    configSource: "projectFile",
    configPath: `/home/leo/dev/${projectId}/trame.yml`,
    createdAt: now,
    updatedAt: now,
  };
}

// ---------------------------------------------------------------------------
// listen
// ---------------------------------------------------------------------------

export async function listen<T = unknown>(
  event: string,
  handler: MockEventHandler<T>,
): Promise<MockUnlistenFn> {
  const unlisten = subscribe(event, handler);
  // Once the store has registered for process logs, replay the seeded session's
  // canned logs so they appear on first paint.
  if (event === "process-log") {
    emitDefaultLogs();
  }
  return unlisten;
}
