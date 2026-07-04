import type {
  CommandReadyConfig,
  DependencyCondition,
  DevappConfig,
  DiavolaConfig,
  ProcessConfig,
  ProcessKind,
  ReadyConfig,
} from "$lib/types";

export type EnvRow = {
  id: string;
  key: string;
  value: string;
};

export type DependencyRow = {
  id: string;
  processName: string;
  condition: DependencyCondition;
};

export type ProcessForm = {
  id: string;
  name: string;
  kind: ProcessKind;
  cmd: string;
  envRows: EnvRow[];
  dependencies: DependencyRow[];
  readyEnabled: boolean;
  readyType: ReadyConfig["type"];
  httpUrl: string;
  logPattern: string;
  logRegex: boolean;
  delayDurationMs: number | string;
  commandCmd: string;
  intervalMs: number | string | null;
  timeoutMs: number | string | null;
};

export type ConfigFormState = {
  globalEnvRows: EnvRow[];
  processes: ProcessForm[];
};

export type IdFactory = (prefix: string) => string;

function defaultNextId(prefix: string) {
  return `${prefix}-${crypto.randomUUID()}`;
}

export function createProcess(name = "api", nextId: IdFactory = defaultNextId): ProcessForm {
  return {
    id: nextId("process"),
    name,
    kind: "service",
    cmd: "",
    envRows: [],
    dependencies: [],
    readyEnabled: false,
    readyType: "http",
    httpUrl: "http://localhost:3000",
    logPattern: "ready",
    logRegex: false,
    delayDurationMs: 1000,
    commandCmd: "",
    intervalMs: null,
    timeoutMs: 60000,
  };
}

export function toProcessForm(
  name: string,
  config: ProcessConfig,
  nextId: IdFactory = defaultNextId,
): ProcessForm {
  const ready = config.ready;
  return {
    ...createProcess(name, nextId),
    name,
    kind: config.kind,
    cmd: config.cmd,
    envRows: Object.entries(config.env ?? {}).map(([key, value]) => ({
      id: nextId("env"),
      key,
      value,
    })),
    dependencies: Object.entries(config.dependsOn ?? {}).map(([processName, condition]) => ({
      id: nextId("dependency"),
      processName,
      condition,
    })),
    readyEnabled: Boolean(ready),
    readyType: ready?.type ?? "http",
    httpUrl: ready?.type === "http" ? ready.url : "http://localhost:3000",
    logPattern: ready?.type === "log" ? ready.pattern : "ready",
    logRegex: ready?.type === "log" ? ready.regex : false,
    delayDurationMs: ready?.type === "delay" ? ready.durationMs : 1000,
    commandCmd: ready?.type === "command" ? ready.cmd : "",
    intervalMs:
      ready?.type === "http" || ready?.type === "command" ? (ready.intervalMs ?? null) : null,
    timeoutMs:
      ready?.type === "http" || ready?.type === "log" || ready?.type === "command"
        ? (ready.timeoutMs ?? null)
        : null,
  };
}

export function buildConfig(form: ConfigFormState): DiavolaConfig {
  const processEntries = form.processes
    .map((process) => [process.name.trim(), buildProcessConfig(process)] as const)
    .filter(([name]) => name.length > 0);
  const globalEnv = Object.fromEntries(
    form.globalEnvRows
      .map((row) => [row.key.trim(), row.value] as const)
      .filter(([key]) => key.length > 0),
  );
  return {
    env: Object.keys(globalEnv).length > 0 ? globalEnv : undefined,
    processes: Object.fromEntries(processEntries),
  };
}

export function buildProcessConfig(process: ProcessForm): ProcessConfig {
  const env = Object.fromEntries(
    process.envRows
      .map((row) => [row.key.trim(), row.value] as const)
      .filter(([key]) => key.length > 0),
  );
  return {
    kind: process.kind,
    cmd: process.cmd,
    env,
    dependsOn: Object.fromEntries(
      process.dependencies
        .map((dependency) => [dependency.processName.trim(), dependency.condition] as const)
        .filter(([processName]) => processName.length > 0),
    ),
    ready: process.readyEnabled ? buildReadyConfig(process) : undefined,
  };
}

export function buildReadyConfig(process: ProcessForm): ReadyConfig {
  if (process.readyType === "log") {
    return optionalTimeout(
      {
        type: "log",
        pattern: process.logPattern,
        regex: process.logRegex,
      },
      process,
    );
  }
  if (process.readyType === "delay") {
    return {
      type: "delay",
      durationMs: numberOrZero(process.delayDurationMs),
    };
  }
  if (process.readyType === "command") {
    return optionalPollFields(
      {
        type: "command",
        cmd: process.commandCmd,
      },
      process,
    );
  }
  return optionalPollFields(
    {
      type: "http",
      url: process.httpUrl,
    },
    process,
  );
}

function numberOrZero(value: number | string | null) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function optionalTimeout<T extends ReadyConfig>(ready: T, process: ProcessForm): T {
  if (process.timeoutMs !== null && process.timeoutMs !== "") {
    return { ...ready, timeoutMs: Number(process.timeoutMs) } as T;
  }
  return ready;
}

function optionalPollFields<T extends Extract<ReadyConfig, { intervalMs?: number }>>(
  ready: T,
  process: ProcessForm,
): T {
  return optionalTimeout(
    {
      ...ready,
      ...(process.intervalMs !== null && process.intervalMs !== ""
        ? { intervalMs: Number(process.intervalMs) }
        : {}),
    },
    process,
  ) as T;
}

export function serializeConfig(config: DevappConfig) {
  const lines: string[] = [];
  const envEntries = Object.entries(config.env ?? {});
  if (envEntries.length > 0) {
    lines.push("env:");
    for (const [key, value] of envEntries) {
      lines.push(`  ${yamlKey(key)}: ${yamlScalar(value)}`);
    }
  }
  lines.push("processes:");
  for (const [name, process] of Object.entries(config.processes)) {
    lines.push(`  ${yamlKey(name)}:`);
    lines.push(`    kind: ${process.kind}`);
    lines.push(`    cmd: ${yamlScalar(process.cmd)}`);
    lines.push("    env:");
    const envEntries = Object.entries(process.env);
    if (envEntries.length === 0) {
      lines.push("      {}");
    } else {
      for (const [key, value] of envEntries) {
        lines.push(`      ${yamlKey(key)}: ${yamlScalar(value)}`);
      }
    }
    lines.push("    dependsOn:");
    const dependencies = Object.entries(process.dependsOn);
    if (dependencies.length === 0) {
      lines.push("      {}");
    } else {
      for (const [dependencyName, condition] of dependencies) {
        lines.push(`      ${yamlKey(dependencyName)}: ${condition}`);
      }
    }
    if (process.ready) {
      lines.push("    ready:");
      lines.push(`      type: ${process.ready.type}`);
      if (process.ready.type === "http") {
        lines.push(`      url: ${yamlScalar(process.ready.url)}`);
        appendOptionalNumber(lines, "intervalMs", process.ready.intervalMs);
        appendOptionalNumber(lines, "timeoutMs", process.ready.timeoutMs);
      }
      if (process.ready.type === "log") {
        lines.push(`      pattern: ${yamlScalar(process.ready.pattern)}`);
        lines.push(`      regex: ${process.ready.regex}`);
        appendOptionalNumber(lines, "timeoutMs", process.ready.timeoutMs);
      }
      if (process.ready.type === "delay") {
        lines.push(`      durationMs: ${process.ready.durationMs}`);
      }
      if (process.ready.type === "command") {
        lines.push(`      cmd: ${yamlScalar((process.ready as CommandReadyConfig).cmd)}`);
        appendOptionalNumber(lines, "intervalMs", process.ready.intervalMs);
        appendOptionalNumber(lines, "timeoutMs", process.ready.timeoutMs);
      }
    }
  }
  return `${lines.join("\n")}\n`;
}

export function yamlScalar(value: string | number | boolean) {
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  return JSON.stringify(value);
}

export function yamlKey(value: string) {
  return /^[A-Za-z_][A-Za-z0-9_-]*$/.test(value) ? value : JSON.stringify(value);
}

function appendOptionalNumber(lines: string[], key: string, value?: number) {
  if (value !== undefined && value !== null) {
    lines.push(`      ${key}: ${value}`);
  }
}
