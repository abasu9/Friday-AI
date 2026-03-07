/**
 * Tauri compatibility layer for browser-only dev mode.
 *
 * When Tauri is available, delegates to `invoke()`.
 * When running in a plain browser, maps known commands to HTTP calls
 * against the Python backend at http://localhost:5167.
 */

const BACKEND = "http://localhost:5167";

export function isTauriAvailable(): boolean {
  return (
    typeof window !== "undefined" && !!(window as any).__TAURI_INTERNALS__
  );
}

// Maps Tauri command names → { method, path, bodyMapper? }
// bodyMapper transforms invoke args into a fetch body (POST only).
const COMMAND_MAP: Record<
  string,
  {
    method: "GET" | "POST";
    path: (args?: any) => string;
    bodyMapper?: (args?: any) => any;
  }
> = {
  api_get_meetings: {
    method: "GET",
    path: () => "/get-meetings",
  },
  api_search_transcripts: {
    method: "POST",
    path: () => "/search-transcripts",
    bodyMapper: (args) => ({ query: args?.query }),
  },
  api_get_summary: {
    method: "GET",
    path: (args) => `/get-summary/${args?.meetingId}`,
  },
  api_get_model_config: {
    method: "GET",
    path: () => "/get-model-config",
  },
  api_get_transcript_config: {
    method: "GET",
    path: () => "/get-transcript-config",
  },
  api_get_api_key: {
    method: "POST",
    path: () => "/get-api-key",
    bodyMapper: (args) => ({ provider: args?.provider }),
  },
  api_save_model_config: {
    method: "POST",
    path: () => "/save-model-config",
    bodyMapper: (args) => ({
      provider: args?.provider,
      model: args?.model,
      whisper_model: args?.whisperModel,
      api_key: args?.apiKey,
      ollama_endpoint: args?.ollamaEndpoint,
    }),
  },
  api_save_transcript_config: {
    method: "POST",
    path: () => "/save-transcript-config",
    bodyMapper: (args) => ({
      provider: args?.provider,
      model: args?.model,
      api_key: args?.apiKey,
    }),
  },
  api_delete_meeting: {
    method: "POST",
    path: () => "/delete-meeting",
    bodyMapper: (args) => ({ meeting_id: args?.meetingId }),
  },
  api_save_meeting_title: {
    method: "POST",
    path: () => "/save-meeting-title",
    bodyMapper: (args) => ({
      meeting_id: args?.meetingId,
      title: args?.title,
    }),
  },
  api_get_meeting: {
    method: "GET",
    path: (args) => `/get-meeting/${args?.meetingId}`,
  },
};

async function httpFallback<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const mapping = COMMAND_MAP[cmd];
  if (!mapping) {
    throw new Error(
      `Command "${cmd}" is not available in browser mode (requires Tauri desktop app).`
    );
  }

  const url = `${BACKEND}${mapping.path(args)}`;
  const init: RequestInit = {
    method: mapping.method,
    headers: { "Content-Type": "application/json" },
  };
  if (mapping.method === "POST" && mapping.bodyMapper) {
    init.body = JSON.stringify(mapping.bodyMapper(args));
  }

  const res = await fetch(url, init);
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText);
    throw new Error(`${cmd} failed: ${res.status} ${text}`);
  }
  return res.json() as Promise<T>;
}

/**
 * Drop-in replacement for Tauri `invoke()`.
 * Uses Tauri IPC when available, falls back to HTTP in browser mode.
 */
export async function safeInvoke<T = any>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> {
  if (isTauriAvailable()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  return httpFallback<T>(cmd, args);
}

/**
 * Safe wrapper for Tauri `listen()`. Returns a no-op unlisten in browser mode.
 */
export async function safeListen<T = any>(
  event: string,
  handler: (event: { payload: T }) => void
): Promise<() => void> {
  if (isTauriAvailable()) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<T>(event, handler);
  }
  // No-op in browser mode
  return () => {};
}
