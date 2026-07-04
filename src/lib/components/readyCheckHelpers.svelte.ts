import type { ProcessForm } from "$lib/config/editorModel";

type SupportedReadyType = "http" | "log" | "delay" | "command";

export const SUPPORTED_READY_TYPES: Array<{ value: SupportedReadyType; label: string }> = [
  { value: "http", label: "HTTP" },
  { value: "log", label: "Log" },
  { value: "delay", label: "Delay" },
  { value: "command", label: "Command" },
];

export function isSupportedReadyType(value: string): value is SupportedReadyType {
  return SUPPORTED_READY_TYPES.some((option) => option.value === value);
}

export function readyTypeOptions(readyType: string) {
  if (isSupportedReadyType(readyType)) return SUPPORTED_READY_TYPES;
  return [
    { value: readyType, label: `Unsupported (${readyType})` },
    ...SUPPORTED_READY_TYPES,
  ];
}

export function sanitizeIdPart(value: string): string {
  return value.replace(/[^a-zA-Z0-9_-]+/g, "-");
}

export function readyErrorId(processId: string, field: string): string {
  return `process-${sanitizeIdPart(processId)}-ready-${field}-error`;
}

export function readyFieldState(
  process: ProcessForm,
  field: string,
  readyIssue: (process: ProcessForm, field: string) => string | null,
): {
  message: string | null;
  describedBy?: string;
  invalid?: "true";
  className?: string;
} {
  const message = readyIssue(process, field);
  return {
    message,
    describedBy: message ? readyErrorId(process.id, field) : undefined,
    invalid: message ? "true" : undefined,
    className: message ? "border-danger focus:border-danger" : undefined,
  };
}

export const UNKNOWN_READY_TYPE_MESSAGE = "Unknown readiness type. Choose a supported type.";
