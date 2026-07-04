// Static fixtures for the browser-only mock backend.
//
// These are never imported when running under Tauri (see transport.ts): the
// mock is loaded via a dynamic import that only resolves when the Tauri
// internals are absent. Everything here is plain data, kept in sync with the
// shapes declared in $lib/types.

import type {
  DevappConfig,
  TrameConfig,
  ProcessLogPayload,
  ProjectConfigDocument,
  ProjectRecord,
  RunSessionSnapshot,
  TerminalSnapshot,
} from "$lib/types";

export type MockProcessLogs = Record<string, ProcessLogPayload[]>;

export type MockProjectFixture = {
  record: ProjectRecord;
  yaml: string;
  config: DevappConfig;
  /** Initial session snapshot used when the project is started. */
  sessionTemplate: RunSessionSnapshot;
  /** Logs emitted per process when the project is started. */
  logs: MockProcessLogs;
};

const NOW = "2026-06-20T09:00:00.000Z";
const STARTED_AT = "2026-06-20T09:00:01.000Z";

function projectRecord(
  id: string,
  name: string,
  baseDir: string,
): ProjectRecord {
  return {
    id,
    name,
    baseDir,
    configSource: "projectFile",
    configPath: `${baseDir}/trame.yml`,
    createdAt: NOW,
    updatedAt: NOW,
  };
}

// ---------------------------------------------------------------------------
// Project 1: deno-runner (mirrors examples/deno-runner.yml)
// ---------------------------------------------------------------------------

const denoRunnerYaml = `processes:
  setup:
    kind: task
    env:
      EXAMPLE_ENV: "hello-from-trame"
    cmd: |
      deno eval 'console.log("setup complete:", Deno.env.get("EXAMPLE_ENV"))'

  api:
    kind: service
    cmd: |
      deno eval 'console.log("api listening on http://127.0.0.1:3999"); setInterval(() => console.log("api tick", new Date().toISOString()), 1500)'
    dependsOn:
      setup: success
    ready:
      type: log
      pattern: "api listening"
      regex: false
      timeoutMs: 10000

  worker:
    kind: service
    cmd: |
      deno eval 'console.log("worker ready"); setInterval(() => console.log("worker tick", new Date().toISOString()), 2000)'
    dependsOn:
      api: ready
    ready:
      type: delay
      durationMs: 500
`;

const denoRunnerConfig: TrameConfig = {
  processes: {
    setup: {
      kind: "task",
      cmd: 'deno eval \'console.log("setup complete:", Deno.env.get("EXAMPLE_ENV"))\'',
      env: { EXAMPLE_ENV: "hello-from-trame" },
      dependsOn: {},
    },
    api: {
      kind: "service",
      cmd: 'deno eval \'console.log("api listening on http://127.0.0.1:3999"); setInterval(() => console.log("api tick", new Date().toISOString()), 1500)\'',
      env: {},
      dependsOn: { setup: "success" },
      ready: { type: "log", pattern: "api listening", regex: false, timeoutMs: 10000 },
    },
    worker: {
      kind: "service",
      cmd: 'deno eval \'console.log("worker ready"); setInterval(() => console.log("worker tick", new Date().toISOString()), 2000)\'',
      env: {},
      dependsOn: { api: "ready" },
      ready: { type: "delay", durationMs: 500 },
    },
  },
};

const denoRunnerSession: RunSessionSnapshot = {
  sessionId: "mock-session-deno-runner",
  projectId: "deno-runner",
  projectName: "deno-runner",
  baseDir: "/home/leo/dev/deno-runner",
  startedAt: STARTED_AT,
  processes: [
    { runtimeId: "proc-setup", name: "setup", kind: "task", status: "succeeded", startedAt: "2026-06-20T09:00:01.200Z", exitedAt: "2026-06-20T09:00:01.450Z", exitCode: 0 },
    { runtimeId: "proc-api", name: "api", kind: "service", status: "ready", startedAt: "2026-06-20T09:00:01.500Z" },
    { runtimeId: "proc-worker", name: "worker", kind: "service", status: "running", startedAt: "2026-06-20T09:00:02.100Z" },
  ],
};

// ---------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------

function log(
  sessionId: string,
  runtimeId: string,
  processName: string,
  stream: ProcessLogPayload["stream"],
  line: string,
  timestamp: string,
): ProcessLogPayload {
  return { sessionId, runtimeId, processName, stream, line, timestamp };
}

const denoRunnerLogs: MockProcessLogs = {
  "proc-setup": [
    log("mock-session-deno-runner", "proc-setup", "setup", "stdout", "setup complete: hello-from-trame", "2026-06-20T09:00:01.400Z"),
    log("mock-session-deno-runner", "proc-setup", "setup", "system", "process exited with code 0", "2026-06-20T09:00:01.450Z"),
  ],
  "proc-api": [
    log("mock-session-deno-runner", "proc-api", "api", "stdout", "api listening on http://127.0.0.1:3999", "2026-06-20T09:00:01.600Z"),
    log("mock-session-deno-runner", "proc-api", "api", "stdout", "api tick 2026-06-20T09:00:03.100Z", "2026-06-20T09:00:03.100Z"),
    log("mock-session-deno-runner", "proc-api", "api", "stdout", "api tick 2026-06-20T09:00:04.600Z", "2026-06-20T09:00:04.600Z"),
  ],
  "proc-worker": [
    log("mock-session-deno-runner", "proc-worker", "worker", "stdout", "worker ready", "2026-06-20T09:00:02.200Z"),
    log("mock-session-deno-runner", "proc-worker", "worker", "stdout", "worker tick 2026-06-20T09:00:04.200Z", "2026-06-20T09:00:04.200Z"),
  ],
};

// ---------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------

export const mockProjects: MockProjectFixture[] = [
  {
    record: projectRecord("deno-runner", "deno-runner", "/home/leo/dev/deno-runner"),
    yaml: denoRunnerYaml,
    config: denoRunnerConfig,
    sessionTemplate: denoRunnerSession,
    logs: denoRunnerLogs,
  },
];

export function mockProjectRecord(id: string): ProjectRecord | undefined {
  return mockProjects.find((project) => project.record.id === id)?.record;
}

export function mockConfigDocument(id: string): ProjectConfigDocument | undefined {
  const fixture = mockProjects.find((project) => project.record.id === id);
  if (!fixture) {
    return undefined;
  }
  return { project: fixture.record, yaml: fixture.yaml, config: fixture.config };
}

export function mockProjectFixture(
  id: string,
): MockProjectFixture | undefined {
  return mockProjects.find((project) => project.record.id === id);
}

/** A terminal that is always available in mock mode, with a static banner. */
export const mockTerminalSnapshot = (terminalId: string): TerminalSnapshot => ({
  terminalId,
  title: `mock terminal (${terminalId})`,
  cwd: "/home/leo/dev/trame",
  createdAt: NOW,
  isOpen: true,
});

export const MOCK_TERMINAL_BANNER =
  "\x1b[36mtrame mock terminal\x1b[0m\r\n" +
  "This is a browser-only stand-in; no shell is attached.\r\n" +
  "Keystrokes are accepted but produce no output.\r\n\r\n" +
  "$ ";
