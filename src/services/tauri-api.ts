import { invoke } from "@tauri-apps/api/core";

type WebBridgeConfig = {
  chatUrl: string;
  url?: string;
  token?: string;
  workspaceRoots?: Array<{ path?: string; name?: string }>;
};

type PendingWebBridgeRequest = {
  resolve: (value: unknown) => void;
  reject: (reason?: unknown) => void;
  timer: number | null;
};

type WebBridgeState = {
  configured: boolean;
  connected: boolean;
  connecting: boolean;
  bridgeReady: boolean;
  authRequired: boolean;
  authenticated: boolean;
  errorText: string;
};

type WebBridgeGlobals = Window & {
  __PAI_SIDEBAR_BRIDGE__?: WebBridgeConfig;
  __PAI_SETTINGS_BRIDGE__?: WebBridgeConfig;
};

const WEB_BRIDGE_TOKEN_STORAGE_PREFIX = "easy_call.web_bridge_token.v1:";

function webBridgeTokenStorageKey(chatUrl: string): string {
  return `${WEB_BRIDGE_TOKEN_STORAGE_PREFIX}${chatUrl.trim()}`;
}

function readPersistedWebBridgeToken(chatUrl: string): string {
  if (typeof window === "undefined") return "";
  return String(window.localStorage.getItem(webBridgeTokenStorageKey(chatUrl)) || "").trim();
}

function persistWebBridgeToken(chatUrl: string, token: string) {
  if (typeof window === "undefined") return;
  const normalizedChatUrl = String(chatUrl || "").trim();
  if (!normalizedChatUrl) return;
  const normalizedToken = String(token || "").trim();
  if (!normalizedToken) {
    window.localStorage.removeItem(webBridgeTokenStorageKey(normalizedChatUrl));
    return;
  }
  window.localStorage.setItem(webBridgeTokenStorageKey(normalizedChatUrl), normalizedToken);
}

function clearPersistedWebBridgeToken(chatUrl: string) {
  persistWebBridgeToken(chatUrl, "");
}

let webBridgeConfig: WebBridgeConfig | null = null;
let webBridgeSocket: WebSocket | null = null;
let webBridgeConnectPromise: Promise<void> | null = null;
let webBridgeRequestId = 1;
const webBridgePending = new Map<number, PendingWebBridgeRequest>();
const webBridgeNotificationHandlers = new Map<string, Set<(payload: unknown) => void>>();
const webBridgeState: WebBridgeState = {
  configured: false,
  connected: false,
  connecting: false,
  bridgeReady: false,
  authRequired: false,
  authenticated: true,
  errorText: "",
};

const WEB_BRIDGE_DEFAULT_TIMEOUT_MS = 30000;
const WEB_BRIDGE_LONG_TIMEOUT_MS = 5 * 60 * 1000;
const WEB_BRIDGE_VERY_LONG_TIMEOUT_MS = 30 * 60 * 1000;
const WEB_BRIDGE_NO_TIMEOUT_COMMANDS = new Set([
  "apply_prepared_github_update",
  "apply_import_config_migration_package",
  "export_config_migration_package",
  "import_angel_memories",
  "import_memories",
  "install_host_runtime_prerequisite",
  "migrate_shell_workspace_directory",
  "mcp_deploy_server",
  "mcp_refresh_mcp_and_skills",
  "mcp_remove_server",
  "mcp_undeploy_server",
  "preview_import_config_migration_package",
  "run_message_store_migration",
  "save_memory_embedding_binding",
  "start_github_update",
]);

const WEB_BRIDGE_COMMAND_TIMEOUT_MS: Record<string, number> = {
  check_github_update: WEB_BRIDGE_LONG_TIMEOUT_MS,
  cleanup_storage_legacy_items: WEB_BRIDGE_VERY_LONG_TIMEOUT_MS,
  codex_get_rate_limits: WEB_BRIDGE_LONG_TIMEOUT_MS,
  codex_start_oauth_login: WEB_BRIDGE_LONG_TIMEOUT_MS,
  disable_agent_private_memory: WEB_BRIDGE_VERY_LONG_TIMEOUT_MS,
  export_agent_private_memories: WEB_BRIDGE_VERY_LONG_TIMEOUT_MS,
  export_memories: WEB_BRIDGE_LONG_TIMEOUT_MS,
  export_memories_to_path: WEB_BRIDGE_VERY_LONG_TIMEOUT_MS,
  fetch_model_metadata: WEB_BRIDGE_LONG_TIMEOUT_MS,
  mcp_list_server_tools: WEB_BRIDGE_LONG_TIMEOUT_MS,
  preview_import_angel_memories: WEB_BRIDGE_LONG_TIMEOUT_MS,
  quick_genai_chat: WEB_BRIDGE_LONG_TIMEOUT_MS,
  refresh_models: WEB_BRIDGE_LONG_TIMEOUT_MS,
  remote_im_weixin_oc_get_login_status: WEB_BRIDGE_LONG_TIMEOUT_MS,
  remote_im_weixin_oc_start_login: WEB_BRIDGE_LONG_TIMEOUT_MS,
  remote_im_weixin_oc_sync_contacts: WEB_BRIDGE_LONG_TIMEOUT_MS,
  remote_im_restart_channel: WEB_BRIDGE_LONG_TIMEOUT_MS,
  search_chat_history_slices: WEB_BRIDGE_LONG_TIMEOUT_MS,
  search_memories_mixed: WEB_BRIDGE_LONG_TIMEOUT_MS,
  task_optimize_draft: WEB_BRIDGE_LONG_TIMEOUT_MS,
  test_embedding_connection: WEB_BRIDGE_LONG_TIMEOUT_MS,
  test_memory_embedding_provider: WEB_BRIDGE_LONG_TIMEOUT_MS,
  test_memory_rerank_provider: WEB_BRIDGE_LONG_TIMEOUT_MS,
  test_rerank_connection: WEB_BRIDGE_LONG_TIMEOUT_MS,
  test_voice_connection: WEB_BRIDGE_LONG_TIMEOUT_MS,
};

export function isTauriRuntimeAvailable(): boolean {
  if (typeof window === "undefined") return false;
  const internals = (window as Window & { __TAURI_INTERNALS__?: { invoke?: unknown } }).__TAURI_INTERNALS__;
  return typeof internals?.invoke === "function";
}

export function invokeTauri<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntimeAvailable()) {
    return invokeWebBridge<T>(command, args);
  }
  return invoke<T>(command, args);
}

export function onWebBridgeNotification(method: string, handler: (payload: unknown) => void): () => void {
  const normalized = method.trim();
  if (!normalized) return () => {};
  const handlers = webBridgeNotificationHandlers.get(normalized) || new Set<(payload: unknown) => void>();
  handlers.add(handler);
  webBridgeNotificationHandlers.set(normalized, handlers);
  return () => {
    handlers.delete(handler);
    if (handlers.size === 0) {
      webBridgeNotificationHandlers.delete(normalized);
    }
  };
}

export function getWebBridgeState(): WebBridgeState {
  ensureWebBridgeConfig();
  return { ...webBridgeState };
}

export function getWebBridgeConfig(): WebBridgeConfig | null {
  return ensureWebBridgeConfig();
}

export function configureWebBridge(config: WebBridgeConfig | null | undefined): WebBridgeConfig | null {
  const normalized = normalizeWebBridgeConfig(config || null);
  if (!normalized) return null;
  webBridgeConfig = normalized;
  webBridgeState.configured = true;
  return normalized;
}

export async function connectWebBridge(): Promise<WebBridgeState> {
  const config = ensureWebBridgeConfig();
  if (!config) {
    webBridgeState.configured = false;
    webBridgeState.errorText = "缺少 PAI Web 桥接配置。";
    throw new Error(webBridgeState.errorText);
  }
  if (webBridgeSocket?.readyState === WebSocket.OPEN && webBridgeState.bridgeReady) {
    return getWebBridgeState();
  }
  await openWebBridgeSocket(config);
  return getWebBridgeState();
}

export async function loginWebBridge(password: string): Promise<WebBridgeState> {
  await invokeWebBridge("auth.login", { password }, 10000);
  return getWebBridgeState();
}

function normalizeWebBridgeConfig(config: WebBridgeConfig | null): WebBridgeConfig | null {
  const chatUrl = String(config?.chatUrl || "").trim();
  const fallbackUrl = String(config?.url || "").trim();
  const resolvedChatUrl = chatUrl || fallbackUrl.replace(/\/ide-context$/, "/chat");
  if (!resolvedChatUrl) return null;
  const persistedToken = config?.token ? "" : readPersistedWebBridgeToken(resolvedChatUrl);
  const token = String(config?.token || persistedToken || "").trim();
  return {
    ...config,
    chatUrl: resolvedChatUrl,
    token: token || undefined,
  };
}

function bridgeUrlFromCurrentLocation(): string {
  if (typeof window === "undefined" || !window.location?.host) return "";
  if (!/^https?:$/i.test(window.location.protocol)) return "";
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}/chat`;
}

function ensureWebBridgeConfig(): WebBridgeConfig | null {
  if (webBridgeConfig) return webBridgeConfig;
  if (typeof window === "undefined") return null;
  const globals = window as WebBridgeGlobals;
  const injected = normalizeWebBridgeConfig(globals.__PAI_SETTINGS_BRIDGE__ || globals.__PAI_SIDEBAR_BRIDGE__ || null);
  if (injected) return configureWebBridge(injected);
  const params = new URLSearchParams(window.location.search || "");
  const fromQuery = normalizeWebBridgeConfig({
    chatUrl: params.get("chatUrl") || bridgeUrlFromCurrentLocation(),
    token: params.get("token") || undefined,
  });
  if (fromQuery) return configureWebBridge(fromQuery);
  return null;
}

function resetWebBridgeConnectionState(errorText = "") {
  webBridgeSocket = null;
  webBridgeConnectPromise = null;
  webBridgeState.connected = false;
  webBridgeState.connecting = false;
  webBridgeState.bridgeReady = false;
  webBridgeState.authRequired = false;
  webBridgeState.authenticated = true;
  webBridgeState.errorText = errorText;
  for (const [id, request] of webBridgePending.entries()) {
    if (request.timer !== null) window.clearTimeout(request.timer);
    request.reject(new Error(errorText || "连接已断开"));
    webBridgePending.delete(id);
  }
}

function settleWebBridgeRequest(id: number, payload: Record<string, unknown>) {
  const request = webBridgePending.get(id);
  if (!request) return;
  webBridgePending.delete(id);
  if (request.timer !== null) window.clearTimeout(request.timer);
  if (payload.error) {
    const error = payload.error as { message?: string };
    const message = String(error?.message || "请求失败");
    const rejected = new Error(message) as Error & { code?: string; type?: string };
    if (message.includes("token expired") || message.includes("discovery refreshed") || message.includes("invalid authToken")) {
      const currentChatUrl = String(webBridgeConfig?.chatUrl || "").trim();
      if (currentChatUrl) {
        clearPersistedWebBridgeToken(currentChatUrl);
      }
      if (webBridgeConfig) {
        webBridgeConfig = { ...webBridgeConfig, token: undefined };
      }
    }
    const codeMatch = message.match(/^([A-Z][A-Z0-9_]+):\s*/);
    if (codeMatch?.[1]) {
      rejected.code = codeMatch[1];
      rejected.type = codeMatch[1];
    }
    request.reject(rejected);
    return;
  }
  if (payload.result && typeof payload.result === "object" && (payload.result as { authenticated?: unknown }).authenticated === true) {
    const authToken = String((payload.result as { authToken?: unknown }).authToken || "").trim();
    const currentChatUrl = String(webBridgeConfig?.chatUrl || "").trim();
    if (authToken && currentChatUrl) {
      persistWebBridgeToken(currentChatUrl, authToken);
      if (webBridgeConfig) {
        webBridgeConfig = { ...webBridgeConfig, token: authToken };
      }
    }
    webBridgeState.authenticated = true;
    webBridgeState.authRequired = false;
  }
  request.resolve(payload.result);
}

function emitWebBridgeNotification(method: string, payload: unknown) {
  const handlers = webBridgeNotificationHandlers.get(method);
  if (!handlers) return;
  for (const handler of handlers) handler(payload);
}

function handleWebBridgeMessage(event: MessageEvent<string>, ready: () => void) {
  let payload: Record<string, unknown>;
  try {
    payload = JSON.parse(String(event.data || "{}"));
  } catch {
    return;
  }
  if (typeof payload.id === "number") {
    settleWebBridgeRequest(payload.id, payload);
    return;
  }
  const method = String(payload.method || "");
  if (method === "bridge.ready") {
    const params = (payload.params || {}) as { authRequired?: unknown };
    const hasAuthToken = !!String(webBridgeConfig?.token || "").trim();
    webBridgeState.bridgeReady = true;
    webBridgeState.authRequired = !!params.authRequired;
    webBridgeState.authenticated = !webBridgeState.authRequired || hasAuthToken;
    ready();
    return;
  }
  if (method === "bridge.shutdown") {
    emitWebBridgeNotification(method, payload.params);
    webBridgeState.errorText = "网络访问已关闭";
    try {
      webBridgeSocket?.close();
    } catch {
      resetWebBridgeConnectionState("网络访问已关闭");
    }
    return;
  }
  if (method) emitWebBridgeNotification(method, payload.params);
}

function openWebBridgeSocket(config: WebBridgeConfig): Promise<void> {
  if (webBridgeConnectPromise) return webBridgeConnectPromise;
  if (webBridgeSocket && webBridgeSocket.readyState !== WebSocket.CLOSED) {
    try {
      webBridgeSocket.close();
    } catch {
      // ignore close errors before reconnecting
    }
  }
  webBridgeState.configured = true;
  webBridgeState.connecting = true;
  webBridgeState.connected = false;
  webBridgeState.bridgeReady = false;
  webBridgeState.authRequired = false;
  webBridgeState.authenticated = true;
  webBridgeState.errorText = "";

  webBridgeConnectPromise = new Promise<void>((resolve, reject) => {
    let settled = false;
    let readyTimer: number | null = null;
    const finishReady = () => {
      if (settled) return;
      settled = true;
      if (readyTimer !== null) window.clearTimeout(readyTimer);
      webBridgeState.connecting = false;
      webBridgeState.connected = true;
      resolve();
    };
    const fail = (error: unknown) => {
      if (readyTimer !== null) window.clearTimeout(readyTimer);
      resetWebBridgeConnectionState(String(error || "PAI 未运行"));
      if (!settled) {
        settled = true;
        reject(error instanceof Error ? error : new Error(String(error || "PAI 未运行")));
      }
    };
    try {
      const socket = new WebSocket(config.chatUrl);
      webBridgeSocket = socket;
      socket.onopen = () => {
        webBridgeState.connected = true;
        readyTimer = window.setTimeout(() => {
          if (!webBridgeState.bridgeReady) fail(new Error("等待 PAI Web 桥接就绪超时"));
        }, 5000);
      };
      socket.onerror = () => fail(new Error("PAI 未运行"));
      socket.onclose = () => {
        if (!settled) {
          fail(new Error("PAI 未运行"));
        } else {
          resetWebBridgeConnectionState("连接已断开");
        }
      };
      socket.onmessage = (event) => handleWebBridgeMessage(event, finishReady);
    } catch (error) {
      fail(error);
    }
  }).finally(() => {
    webBridgeConnectPromise = null;
  });
  return webBridgeConnectPromise;
}

function webBridgeTimeoutForCommand(command: string, requestedTimeoutMs?: number): number | null {
  if (typeof requestedTimeoutMs === "number") return requestedTimeoutMs;
  if (WEB_BRIDGE_NO_TIMEOUT_COMMANDS.has(command)) return null;
  return WEB_BRIDGE_COMMAND_TIMEOUT_MS[command] ?? WEB_BRIDGE_DEFAULT_TIMEOUT_MS;
}

async function invokeWebBridge<T>(command: string, args?: Record<string, unknown>, timeoutMs?: number): Promise<T> {
  const config = ensureWebBridgeConfig();
  if (!config) throw new Error("缺少 PAI Web 桥接配置。");
  await connectWebBridge();
  if (!webBridgeSocket || webBridgeSocket.readyState !== WebSocket.OPEN) {
    throw new Error("PAI 未运行");
  }
  if (webBridgeState.authRequired && !webBridgeState.authenticated && command !== "auth.login") {
    throw new Error("远程访问需要先输入密码");
  }
  const id = webBridgeRequestId++;
  const authToken = String(config.token || "").trim();
  const params = authToken ? { authToken, ...(args || {}) } : (args || {});
  const body = { jsonrpc: "2.0", id, method: command, params };
  return new Promise<T>((resolve, reject) => {
    const resolvedTimeoutMs = webBridgeTimeoutForCommand(command, timeoutMs);
    const timer = resolvedTimeoutMs === null
      ? null
      : window.setTimeout(() => {
          webBridgePending.delete(id);
          reject(new Error("请求超时"));
        }, resolvedTimeoutMs);
    webBridgePending.set(id, { resolve: resolve as (value: unknown) => void, reject, timer });
    webBridgeSocket?.send(JSON.stringify(body));
  });
}
