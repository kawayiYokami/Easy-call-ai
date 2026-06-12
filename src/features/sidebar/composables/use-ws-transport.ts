import { computed, onBeforeUnmount, ref } from "vue";

type PendingRequest = {
  resolve: (value: unknown) => void;
  reject: (reason?: unknown) => void;
  timer: number;
};

export type SidebarBridgeConfig = {
  chatUrl: string;
  token?: string;
};

const SIDEBAR_BRIDGE_TOKEN_STORAGE_PREFIX = "easy_call.sidebar.bridge_token.v1:";

function sidebarBridgeTokenStorageKey(chatUrl: string): string {
  return `${SIDEBAR_BRIDGE_TOKEN_STORAGE_PREFIX}${chatUrl.trim()}`;
}

function readPersistedSidebarBridgeToken(chatUrl: string): string {
  if (typeof window === "undefined") return "";
  return String(window.localStorage.getItem(sidebarBridgeTokenStorageKey(chatUrl)) || "").trim();
}

function persistSidebarBridgeToken(chatUrl: string, token: string) {
  if (typeof window === "undefined") return;
  const normalizedChatUrl = String(chatUrl || "").trim();
  if (!normalizedChatUrl) return;
  const normalizedToken = String(token || "").trim();
  if (!normalizedToken) {
    window.localStorage.removeItem(sidebarBridgeTokenStorageKey(normalizedChatUrl));
    return;
  }
  window.localStorage.setItem(sidebarBridgeTokenStorageKey(normalizedChatUrl), normalizedToken);
}

function clearPersistedSidebarBridgeToken(chatUrl: string) {
  persistSidebarBridgeToken(chatUrl, "");
}

export function useWsTransport() {
  const socket = ref<WebSocket | null>(null);
  const connected = ref(false);
  const connecting = ref(false);
  const bridgeReady = ref(false);
  const authRequired = ref(false);
  const authenticated = ref(true);
  const errorText = ref("");
  const bridgeConfig = ref<SidebarBridgeConfig | null>(null);
  const notificationHandlers = new Map<string, Set<(payload: unknown) => void>>();
  const pending = new Map<number, PendingRequest>();
  let authRefreshHandler: (() => void) | null = null;
  let requestId = 1;

  const canSend = computed(() => connected.value && socket.value?.readyState === WebSocket.OPEN);

  function emitNotification(method: string, payload: unknown) {
    const handlers = notificationHandlers.get(method);
    if (!handlers) return;
    for (const handler of handlers) handler(payload);
  }

  function settle(id: number, payload: Record<string, unknown>) {
    const item = pending.get(id);
    if (!item) return;
    pending.delete(id);
    window.clearTimeout(item.timer);
    if (payload.error) {
      const error = payload.error as { message?: string };
      const message = String(error?.message || "请求失败");
      if (message.includes("token expired") || message.includes("discovery refreshed") || message.includes("invalid authToken")) {
        const currentChatUrl = String(bridgeConfig.value?.chatUrl || "").trim();
        if (currentChatUrl) {
          clearPersistedSidebarBridgeToken(currentChatUrl);
        }
        if (bridgeConfig.value) {
          bridgeConfig.value = { ...bridgeConfig.value, token: undefined };
        }
        authRefreshHandler?.();
      }
      item.reject(new Error(message));
      return;
    }
    if (payload.result && typeof payload.result === "object" && (payload.result as { authenticated?: unknown }).authenticated === true) {
      const authToken = String((payload.result as { authToken?: unknown }).authToken || "").trim();
      const currentChatUrl = String(bridgeConfig.value?.chatUrl || "").trim();
      if (authToken && currentChatUrl) {
        persistSidebarBridgeToken(currentChatUrl, authToken);
        if (bridgeConfig.value) {
          bridgeConfig.value = { ...bridgeConfig.value, token: authToken };
        }
      }
      authenticated.value = true;
      authRequired.value = false;
    }
    item.resolve(payload.result);
  }

  function handleMessage(event: MessageEvent<string>, ready?: () => void) {
    let payload: Record<string, unknown>;
    try {
      payload = JSON.parse(String(event.data || "{}"));
    } catch {
      return;
    }
    if (typeof payload.id === "number") {
      settle(payload.id, payload);
      return;
    }
    const method = String(payload.method || "");
    if (method === "bridge.ready") {
      const params = (payload.params || {}) as { authRequired?: unknown };
      const hasAuthToken = !!String(bridgeConfig.value?.token || "").trim();
      bridgeReady.value = true;
      authRequired.value = !!params.authRequired;
      authenticated.value = !authRequired.value || hasAuthToken;
      ready?.();
    }
    if (method) emitNotification(method, payload.params);
  }

  function close() {
    const current = socket.value;
    socket.value = null;
    connected.value = false;
    connecting.value = false;
    bridgeReady.value = false;
    authRequired.value = false;
    authenticated.value = true;
    for (const [id, item] of pending.entries()) {
      window.clearTimeout(item.timer);
      item.reject(new Error("连接已断开"));
      pending.delete(id);
    }
    if (current && current.readyState !== WebSocket.CLOSED) current.close();
  }

  async function connect(config: SidebarBridgeConfig) {
    close();
    const persistedToken = config.token ? "" : readPersistedSidebarBridgeToken(config.chatUrl);
    const nextConfig: SidebarBridgeConfig = {
      ...config,
      token: String(config.token || persistedToken || "").trim() || undefined,
    };
    bridgeConfig.value = nextConfig;
    connecting.value = true;
    bridgeReady.value = false;
    authRequired.value = false;
    authenticated.value = true;
    errorText.value = "";
    await new Promise<void>((resolve, reject) => {
      let settled = false;
      let readyTimer: number | null = null;
      const finishReady = () => {
        if (settled) return;
        settled = true;
        if (readyTimer !== null) window.clearTimeout(readyTimer);
        connected.value = true;
        connecting.value = false;
        resolve();
      };
      const fail = (error: unknown) => {
        if (readyTimer !== null) window.clearTimeout(readyTimer);
        connected.value = false;
        connecting.value = false;
        if (socket.value?.readyState !== WebSocket.OPEN) {
          bridgeReady.value = false;
        }
        errorText.value = String(error || "PAI 未运行");
        if (!settled) {
          settled = true;
          reject(error instanceof Error ? error : new Error(String(error || "PAI 未运行")));
        }
      };
      const ws = new WebSocket(nextConfig.chatUrl);
      socket.value = ws;
      ws.onopen = () => {
        connected.value = true;
        readyTimer = window.setTimeout(() => {
          if (!bridgeReady.value) fail(new Error("等待 PAI 侧边栏桥接就绪超时"));
        }, 5000);
      };
      ws.onerror = () => {
        fail(new Error("PAI 未运行"));
      };
      ws.onclose = () => {
        for (const [id, item] of pending.entries()) {
          window.clearTimeout(item.timer);
          item.reject(new Error("连接已断开"));
          pending.delete(id);
        }
        if (!settled) {
          fail(new Error("PAI 未运行"));
        } else if (socket.value === ws) {
          connected.value = false;
          connecting.value = false;
          bridgeReady.value = false;
          authRequired.value = false;
          authenticated.value = true;
          errorText.value = "连接已断开";
        }
      };
      ws.onmessage = (event) => handleMessage(event, finishReady);
    });
  }

  function request<T>(method: string, params: Record<string, unknown> = {}, timeoutMs = 30000): Promise<T> {
    if (!canSend.value || !socket.value) return Promise.reject(new Error("PAI 未运行"));
    if (authRequired.value && !authenticated.value && method !== "auth.login") {
      return Promise.reject(new Error("远程访问需要先输入密码"));
    }
    const id = requestId++;
    const authToken = String(bridgeConfig.value?.token || "").trim();
    const bodyParams = authToken ? { authToken, ...params } : params;
    const body = { jsonrpc: "2.0", id, method, params: bodyParams };
    return new Promise<T>((resolve, reject) => {
      const timer = window.setTimeout(() => {
        pending.delete(id);
        reject(new Error("请求超时"));
      }, timeoutMs);
      pending.set(id, { resolve: resolve as (value: unknown) => void, reject, timer });
      socket.value?.send(JSON.stringify(body));
    });
  }

  function onNotification(method: string, handler: (payload: unknown) => void) {
    const handlers = notificationHandlers.get(method) || new Set<(payload: unknown) => void>();
    handlers.add(handler);
    notificationHandlers.set(method, handlers);
    return () => handlers.delete(handler);
  }

  function onAuthRefreshNeeded(handler: () => void) {
    authRefreshHandler = handler;
  }

  async function login(password: string): Promise<void> {
    await request("auth.login", { password }, 10000);
  }

  async function reconnect() {
    const config = bridgeConfig.value;
    if (!config) return;
    await connect(config);
  }

  onBeforeUnmount(() => close());

  return {
    connected,
    connecting,
    bridgeReady,
    authRequired,
    authenticated,
    errorText,
    bridgeConfig,
    canSend,
    connect,
    reconnect,
    close,
    login,
    request,
    onNotification,
    onAuthRefreshNeeded,
  };
}
