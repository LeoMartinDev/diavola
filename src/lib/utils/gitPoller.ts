import { getGitInfo } from "$lib/tauri/client";
import type { GitInfo } from "$lib/types";

export function createGitPoller() {
  let timer: ReturnType<typeof setInterval> | null = null;

  async function fetch(baseDir: string): Promise<GitInfo | null> {
    try {
      return await getGitInfo(baseDir);
    } catch {
      return null;
    }
  }

  function start(baseDir: string, onUpdate: (info: GitInfo | null) => void) {
    stop();
    fetch(baseDir).then(onUpdate);
    timer = setInterval(() => {
      fetch(baseDir).then(onUpdate);
    }, 60_000);
  }

  function stop() {
    if (timer !== null) {
      clearInterval(timer);
      timer = null;
    }
  }

  return { start, stop, fetch };
}
