const ERROR_MESSAGES: Record<string, string> = {
  configNotFound: "Config file not found. Check that trame.yml exists.",
  projectAlreadyRunning: "A project is already running in this window.",
  launchLocked: "Project is locked by command-line launch and cannot be modified.",
  processCannotRestart: "Tasks cannot be restarted — only services support restart.",
  readinessTimeout: "Process readiness check timed out. Check the readiness config.",
  readinessCheckFailed: "Process readiness check failed. Verify the readiness command or URL.",
  projectNotFound: "Project not found. It may have been deleted.",
};

export function resolveErrorMessage(error: unknown): string {
  const raw = error instanceof Error ? error.message : String(error);
  try {
    const parsed = JSON.parse(raw) as { message: string; code: string };
    if (parsed.code && parsed.message) {
      return ERROR_MESSAGES[parsed.code] ?? parsed.message;
    }
  } catch {
    // Not JSON — use the raw string
  }
  return raw;
}
