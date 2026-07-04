// Transport seam between the real Tauri APIs and the browser-only mock.
//
// The whole frontend talks to the backend through two functions, `invoke` and
// `listen`, re-exported from here. When the app runs inside Tauri (the
// webview exposes `window.__TAURI_INTERNALS__`) we delegate to the real
// `@tauri-apps/api`. Otherwise — i.e. `deno task dev` in a plain browser — we
// dynamically import the mock implementation, which Vite ships as a separate
// chunk that is never fetched under Tauri.
//
// The exposed signatures intentionally match `@tauri-apps/api/core#invoke` and
// `@tauri-apps/api/event#listen`, so `client.ts` and the runtime store only
// need a one-line import change.

import type { InvokeArgs } from "@tauri-apps/api/core";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import {
  listen as tauriListen,
  type EventCallback,
  type UnlistenFn,
} from "@tauri-apps/api/event";

import { isTauriRuntime } from "./environment";

const isTauri = isTauriRuntime();

// The mock module is imported lazily and cached so the seam resolves the same
// instance on every call without paying the dynamic-import cost twice.
type MockTransport = {
  invoke: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
  listen: <T = unknown>(
    event: string,
    handler: (event: { event: string; id: number; payload: T }) => void,
  ) => Promise<UnlistenFn>;
};

let mockPromise: Promise<MockTransport> | null = null;

function loadMock(): Promise<MockTransport> {
  if (!mockPromise) {
    // `import()` is the whole point: it keeps the mock out of the Tauri bundle.
    mockPromise = import("./mock/transport").then(
      (module) =>
        module as unknown as MockTransport,
    );
  }
  return mockPromise;
}

export type { UnlistenFn, EventCallback };

export async function invoke<T = unknown>(
  cmd: string,
  args?: InvokeArgs,
): Promise<T> {
  if (isTauri) {
    return tauriInvoke<T>(cmd, args);
  }
  const mock = await loadMock();
  return mock.invoke<T>(cmd, args as Record<string, unknown> | undefined);
}

export function listen<T = unknown>(
  event: string,
  handler: EventCallback<T>,
): Promise<UnlistenFn> {
  if (isTauri) {
    return tauriListen<T>(event, handler);
  }
  // `loadMock` is async, so wrap the mock subscription in a Promise that
  // resolves once the listener is registered, mirroring Tauri's contract.
  return loadMock().then((mock) => mock.listen<T>(event, handler));
}
