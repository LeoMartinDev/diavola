export type ProcessKind = "task" | "service";

export type DependencyCondition = "success" | "ready";

export type ReadyConfig =
  | HttpReadyConfig
  | LogReadyConfig
  | DelayReadyConfig
  | CommandReadyConfig;

export type HttpReadyConfig = {
  type: "http";
  url: string;
  intervalMs?: number;
  timeoutMs?: number;
};

export type LogReadyConfig = {
  type: "log";
  pattern: string;
  regex: boolean;
  timeoutMs?: number;
};

export type DelayReadyConfig = {
  type: "delay";
  durationMs: number;
};

export type CommandReadyConfig = {
  type: "command";
  cmd: string;
  intervalMs?: number;
  timeoutMs?: number;
};

export type ProcessConfig = {
  kind: ProcessKind;
  cmd: string;
  env: Record<string, string>;
  dependsOn: Record<string, DependencyCondition>;
  ready?: ReadyConfig;
  gracePeriodMs?: number;
  logEntryPattern?: string;
};

export type DiavolaConfig = {
  env?: Record<string, string>;
  gracePeriodMs?: number;
  logEntryPattern?: string;
  processes: Record<string, ProcessConfig>;
};

export type ProcessStatus =
  | "pending"
  | "blocked"
  | "starting"
  | "running"
  | "ready"
  | "succeeded"
  | "failed"
  | "stopping"
  | "stopped";

export type LogStream = "stdout" | "stderr" | "system";

export type AppUpdateState =
  | { status: "idle" }
  | { status: "checking" }
  | {
      status: "downloading";
      version: string;
      downloadedBytes: number;
      contentLength: number | null;
    }
  | { status: "ready"; version: string }
  | { status: "installing"; version: string }
  | { status: "error"; message: string };

export type ProjectSource = "projectFile" | "appConfigFile";

export type ProjectId = string;

export type RunSessionId = string;

export type ProcessRuntimeId = string;

export type TerminalSessionId = string;

export type ProjectRecord = {
  id: ProjectId;
  name: string;
  baseDir: string;
  configSource: ProjectSource;
  configPath: string;
  createdAt: string;
  updatedAt: string;
};

export type ProcessSnapshot = {
  runtimeId: ProcessRuntimeId;
  name: string;
  kind: ProcessKind;
  status: ProcessStatus;
  startedAt?: string;
  exitedAt?: string;
  exitCode?: number;
};

export type RunSessionSnapshot = {
  sessionId: RunSessionId;
  projectId: ProjectId;
  projectName: string;
  baseDir: string;
  startedAt: string;
  stoppedAt?: string;
  processes: ProcessSnapshot[];
};

export type ProcessLogPayload = {
  sessionId: RunSessionId;
  runtimeId: ProcessRuntimeId;
  processName: string;
  stream: LogStream;
  lines: string[];
  timestamp: string;
};

export type SearchProcessLogsReply = {
  matchCount: number;
  matchIndices: number[];
};

export type FlatRow = {
  entryId: number;
  lineIndex: number;
  isFirstLine: boolean;
  isContinuation: boolean;
  text: string;
  stream: LogStream;
  timestamp: string;
};

export type TerminalSnapshot = {
  terminalId: TerminalSessionId;
  title: string;
  cwd: string;
  createdAt: string;
  isOpen: boolean;
};

export type TerminalOutputPayload = {
  terminalId: TerminalSessionId;
  chunk: string;
  timestamp: string;
};

export type ProjectConfigDocument = {
  project: ProjectRecord;
  yaml: string;
  config: DiavolaConfig;
};

export type SessionStatusEvent = {
  snapshot: RunSessionSnapshot | null;
};

export type ProcessLogEvent = {
  payload: ProcessLogPayload;
};

export type TerminalEvent = {
  snapshot: TerminalSnapshot;
};

export type TerminalOutputEvent = {
  payload: TerminalOutputPayload;
};

export type RuntimeErrorEvent = {
  type: "runtimeError";
  message: string;
};

export interface GitInfo {
  repoName: string | null;
  branch: string | null;
  worktree: string | null;
  isWorktree: boolean;
  displayPath: string | null;
}

export type ErrorCode =
  | "configNotFound"
  | "configParseFailed"
  | "configValidationFailed"
  | "configDependencyCycle"
  | "configUnknownProcess"
  | "projectNotFound"
  | "projectAlreadyRunning"
  | "launchLocked"
  | "processNotFound"
  | "processStartFailed"
  | "processCannotRestart"
  | "readinessTimeout"
  | "readinessCheckFailed"
  | "ioError"
  | "terminalError";
