import { computed, onMounted, onUnmounted, ref, type ComputedRef } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type ConversationPipelineStatus = "busy" | "success" | "error";

export interface ConversationWorkStatusEvent {
  conversationId?: string;
  conversation_id?: string;
  status: "working" | "completed" | "error";
  requestId?: string;
  request_id?: string;
}

export interface PipelineState {
  conversationStatusById: ComputedRef<Record<string, ConversationPipelineStatus>>;
  markConversationRead: (conversationId: string) => void;
  clearConversationStatus: (conversationId: string, expectedStatus?: ConversationPipelineStatus) => void;
}

interface UsePipelineStatusOptions {
  activeConversationId?: ComputedRef<string>;
}

const conversationStatusByIdRef = ref<Record<string, ConversationPipelineStatus>>({});
const latestRequestIdByConversation = new Map<string, string>();
let unlisten: UnlistenFn | null = null;
let listenerStarting = false;
let consumerCount = 0;
let currentActiveConversationIdGetter: (() => string) | null = null;

function setConversationStatus(conversationId: string, status?: ConversationPipelineStatus) {
  const next = { ...conversationStatusByIdRef.value };
  if (status) {
    next[conversationId] = status;
  } else {
    delete next[conversationId];
    latestRequestIdByConversation.delete(conversationId);
  }
  conversationStatusByIdRef.value = next;
}

async function startConversationWorkStatusListener() {
  if (unlisten || listenerStarting) return;
  listenerStarting = true;
  try {
    const off = await listen<ConversationWorkStatusEvent>("conversation_work_status", (event) => {
      const payload = event.payload;
      const conversationId = String(payload.conversationId || payload.conversation_id || "").trim();
      if (!conversationId) return;

      const requestId = String(payload.requestId || payload.request_id || "").trim();
      const latestRequestId = latestRequestIdByConversation.get(conversationId) || "";
      if (requestId) {
        if (latestRequestId && requestId !== latestRequestId && payload.status !== "working") {
          return;
        }
        latestRequestIdByConversation.set(conversationId, requestId);
      }

      if (payload.status === "working") {
        setConversationStatus(conversationId, "busy");
        return;
      }
      if (payload.status === "error") {
        setConversationStatus(conversationId, "error");
        return;
      }
      const activeConversationId = currentActiveConversationIdGetter ? currentActiveConversationIdGetter() : "";
      if (conversationId === activeConversationId) {
        setConversationStatus(conversationId);
        return;
      }
      // "success" 表示后台会话有未查看的完成结果，保留到用户切入该会话后再清理。
      setConversationStatus(conversationId, "success");
    });
    if (consumerCount <= 0) {
      off();
    } else {
      unlisten = off;
    }
  } catch (error) {
    const err = error instanceof Error ? error : new Error(String(error));
    console.error("[运行状态] 注册 conversation_work_status 监听失败", {
      message: err.message,
      stack: err.stack,
      error,
    });
  } finally {
    listenerStarting = false;
  }
}

function stopConversationWorkStatusListener() {
  if (consumerCount > 0 || !unlisten) return;
  unlisten();
  unlisten = null;
}

export function usePipelineStatus(options: UsePipelineStatusOptions = {}): PipelineState {
  currentActiveConversationIdGetter = options.activeConversationId
    ? () => String(options.activeConversationId?.value || "").trim()
    : currentActiveConversationIdGetter;

  onMounted(() => {
    consumerCount += 1;
    void startConversationWorkStatusListener();
  });

  onUnmounted(() => {
    consumerCount = Math.max(0, consumerCount - 1);
    stopConversationWorkStatusListener();
  });

  return {
    conversationStatusById: computed(() => conversationStatusByIdRef.value),
    markConversationRead: (conversationId: string) => {
      const normalizedConversationId = String(conversationId || "").trim();
      if (!normalizedConversationId) return;
      if (conversationStatusByIdRef.value[normalizedConversationId] === "success") {
        setConversationStatus(normalizedConversationId);
      }
    },
    clearConversationStatus: (conversationId: string, expectedStatus?: ConversationPipelineStatus) => {
      const normalizedConversationId = String(conversationId || "").trim();
      if (!normalizedConversationId) return;
      if (expectedStatus && conversationStatusByIdRef.value[normalizedConversationId] !== expectedStatus) return;
      setConversationStatus(normalizedConversationId);
    },
  };
}
