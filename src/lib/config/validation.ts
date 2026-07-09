import type { ConfigFormState, EnvRow, ProcessForm } from "$lib/config/editorModel";
import type { ProjectSource } from "$lib/types";

export type ValidationIssue = {
  key: string;
  message: string;
};

export type ValidationResult = {
  valid: boolean;
  issues: ValidationIssue[];
};

export type ProjectDetailsInput = {
  name: string;
  baseDir: string;
  configSource: ProjectSource | "";
};

const VALID_CONFIG_SOURCES = new Set<ProjectDetailsInput["configSource"]>([
  "projectFile",
  "appConfigFile",
  "",
]);

export function validateProjectDetails(input: ProjectDetailsInput): ValidationResult {
  const issues: ValidationIssue[] = [];
  if (input.name.trim().length === 0) {
    issues.push({ key: "project.name", message: "Name is required." });
  }
  if (input.baseDir.trim().length === 0) {
    issues.push({ key: "project.baseDir", message: "Base directory is required." });
  }
  if (!VALID_CONFIG_SOURCES.has(input.configSource)) {
    issues.push({ key: "project.configSource", message: "Choose a valid configuration source." });
  }
  return toResult(issues);
}

export function validateConfigForm(formState: ConfigFormState): ValidationResult {
  const issues: ValidationIssue[] = [];
  validateProcesses(formState.processes, issues);
  return toResult(issues);
}

function validateProcessEnvRows(processId: string, rows: EnvRow[], issues: ValidationIssue[]) {
  const seen = new Map<string, string>();
  for (const row of rows) {
    const key = row.key.trim();
    if (key.length === 0 && row.value.trim().length > 0) {
      issues.push({ key: `process.${processId}.env.${row.id}.key`, message: "Environment key is required." });
      continue;
    }
    if (key.length === 0) {
      continue;
    }
    const firstRowId = seen.get(key);
    if (firstRowId) {
      issues.push({ key: `process.${processId}.env.${row.id}.key`, message: "Environment keys must be unique." });
      issues.push({ key: `process.${processId}.env.${firstRowId}.key`, message: "Environment keys must be unique." });
    } else {
      seen.set(key, row.id);
    }
  }
}

function validateProcesses(processes: ProcessForm[], issues: ValidationIssue[]) {
  const names = new Map<string, ProcessForm>();
  for (const process of processes) {
    const name = process.name.trim();
    if (name.length === 0) {
      issues.push({ key: `process.${process.id}.name`, message: "Process name is required." });
    } else {
      const existing = names.get(name);
      if (existing) {
        issues.push({ key: `process.${process.id}.name`, message: "Process names must be unique." });
        issues.push({
          key: `process.${existing.id}.name`,
          message: "Process names must be unique.",
        });
      } else {
        names.set(name, process);
      }
    }

    if (process.cmd.trim().length === 0) {
      issues.push({ key: `process.${process.id}.cmd`, message: "Command is required." });
    }

    validateProcessEnvRows(process.id, process.envRows, issues);
    validateDependencies(process, names, processes, issues);
    validateReadyConfig(process, issues);
  }
}

function validateDependencies(
  process: ProcessForm,
  currentNames: Map<string, ProcessForm>,
  processes: ProcessForm[],
  issues: ValidationIssue[],
) {
  const allNames = new Map(processes.map((candidate) => [candidate.name.trim(), candidate]));
  for (const dependency of process.dependencies) {
    const dependencyName = dependency.processName.trim();
    if (dependencyName.length === 0) {
      issues.push({
        key: `process.${process.id}.dependency.${dependency.id}`,
        message: "Choose a dependency process.",
      });
      continue;
    }
    const target = allNames.get(dependencyName) ?? currentNames.get(dependencyName);
    if (!target) {
      issues.push({
        key: `process.${process.id}.dependency.${dependency.id}`,
        message: "Dependency must reference an existing process.",
      });
      continue;
    }
    if (target.id === process.id || dependencyName === process.name.trim()) {
      issues.push({
        key: `process.${process.id}.dependency.${dependency.id}`,
        message: "A process cannot depend on itself.",
      });
    }
  }
}

function validateReadyConfig(process: ProcessForm, issues: ValidationIssue[]) {
  if (!process.readyEnabled) {
    return;
  }
  if (process.readyType === "http") {
    const urlIssue = validateHttpUrl(process.httpUrl);
    if (urlIssue) {
      issues.push({ key: `process.${process.id}.ready.httpUrl`, message: urlIssue });
    }
    validateNonNegativeNumber(process.intervalMs, `process.${process.id}.ready.intervalMs`, issues);
    validateNonNegativeNumber(process.timeoutMs, `process.${process.id}.ready.timeoutMs`, issues);
    return;
  }
  if (process.readyType === "log") {
    if (process.logPattern.trim().length === 0) {
      issues.push({
        key: `process.${process.id}.ready.logPattern`,
        message: "Log pattern is required.",
      });
    }
    validateNonNegativeNumber(process.timeoutMs, `process.${process.id}.ready.timeoutMs`, issues);
    return;
  }
  if (process.readyType === "delay") {
    validateNonNegativeNumber(
      process.delayDurationMs,
      `process.${process.id}.ready.delayDurationMs`,
      issues,
    );
    return;
  }
  if (process.commandCmd.trim().length === 0) {
    issues.push({
      key: `process.${process.id}.ready.commandCmd`,
      message: "Readiness command is required.",
    });
  }
  validateNonNegativeNumber(process.intervalMs, `process.${process.id}.ready.intervalMs`, issues);
  validateNonNegativeNumber(process.timeoutMs, `process.${process.id}.ready.timeoutMs`, issues);
}

function validateHttpUrl(value: string) {
  try {
    const url = new URL(value);
    if (url.protocol !== "http:" && url.protocol !== "https:") {
      return "URL must use http or https.";
    }
    return null;
  } catch {
    return "Enter a valid HTTP URL.";
  }
}

function validateNonNegativeNumber(
  value: number | string | null,
  key: string,
  issues: ValidationIssue[],
) {
  if (value === null || value === "") {
    return;
  }
  const numericValue = Number(value);
  if (!Number.isFinite(numericValue) || numericValue < 0) {
    issues.push({ key, message: "Enter a finite number greater than or equal to 0." });
  }
}

function toResult(issues: ValidationIssue[]): ValidationResult {
  return { valid: issues.length === 0, issues };
}
