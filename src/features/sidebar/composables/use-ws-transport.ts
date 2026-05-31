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

export function useWsTransport() {
  const socket = ref<WebSocket | null>(null);
  const connected = ref(false);
  const connecting = ref(false);
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
        authRefreshHandler?.();
      }
      item.reject(new Error(message));
      return;
    }
    item.resolve(payload.result);
  }

  function handleMessage(event: MessageEvent<string>) {
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
    if (method) emitNotification(method, payload.params);
  }

  function close() {
    const current = socket.value;
    socket.value = null;
    connected.value = false;
    connecting.value = false;
    if (current && current.readyState !== WebSocket.CLOSED) current.close();
  }

  async function connect(config: SidebarBridgeConfig) {
    close();
    bridgeConfig.value = config;
    connecting.value = true;
    errorText.value = "";
    await new Promise<void>((resolve) => {
      const ws = new WebSocket(config.chatUrl);
      socket.value = ws;
      ws.onopen = () => {
        connected.value = true;
        connecting.value = false;
        resolve();
      };
      ws.onerror = () => {
        errorText.value = "PAI 未运行";
      };
      ws.onclose = () => {
        connected.value = false;
        connecting.value = false;
        if (socket.value === ws) errorText.value = "PAI 未运行";
        for (const [id, item] of pending.entries()) {
          window.clearTimeout(item.timer);
          item.reject(new Error("连接已断开"));
          pending.delete(id);
        }
        resolve();
      };
      ws.onmessage = handleMessage;
    });
  }

  function request<T>(method: string, params: Record<string, unknown> = {}, timeoutMs = 30000): Promise<T> {
    if (!canSend.value || !socket.value) return Promise.reject(new Error("PAI 未运行"));
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

  async function reconnect() {
    const config = bridgeConfig.value;
    if (!config) return;
    await connect(config);
  }

  onBeforeUnmount(() => close());

  return {
    connected,
    connecting,
    errorText,
    canSend,
    connect,
    reconnect,
    close,
    request,
    onNotification,
    onAuthRefreshNeeded,
  };
}
