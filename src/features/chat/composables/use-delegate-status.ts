import { ref, watch, onMounted, onBeforeUnmount, type Ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invokeTauri, isTauriRuntimeAvailable } from "../../../services/tauri-api";
import type { ConversationDelegateStatusSummary } from "../../../types/app";

const ARCHIVE_FOCUS_REQUEST_STORAGE_KEY = "easy_call.archives.focus_request.v1";
const DELEGATE_STATUS_UPDATED_EVENT = "easy-call:conversation-delegate-status-updated";

interface UseDelegateStatusOptions {
  activeConversationId: Ref<string>;
  panelOpen: Ref<boolean>;
  enabled?: Ref<boolean>;
  bridgeRequest?: Ref<((method: string, params?: Record<string, unknown>, timeoutMs?: number) => Promise<unknown>) | undefined>;
}

type DelegateStatusUpdatedPayload = {
  rootConversationId?: string;
  conversationId?: string;
  delegateId?: string;
  status?: string;
};

export function useDelegateStatus(options: UseDelegateStatusOptions) {
  const { activeConversationId, panelOpen } = options;

  const delegateStatuses = ref<ConversationDelegateStatusSummary[]>([]);
  const delegateStatusesErrorText = ref("");
  const hasBridgeRequest = () => typeof options.bridgeRequest?.value === "function";
  const enabled = () => options.enabled?.value !== false && (isTauriRuntimeAvailable() || hasBridgeRequest());

  let delegateStatusUpdatedUnlisten: UnlistenFn | null = null;
  let disposed = false;
  let requestSeq = 0;

  async function refresh() {
    const conversationId = String(activeConversationId.value || "").trim();
    if (!enabled() || !conversationId || !panelOpen.value) {
      requestSeq += 1;
      delegateStatuses.value = [];
      delegateStatusesErrorText.value = "";
      return;
    }
    const seq = ++requestSeq;
    try {
      const statuses = hasBridgeRequest()
        ? await options.bridgeRequest!.value!("delegate.statuses", { conversationId }, 10000) as ConversationDelegateStatusSummary[]
        : await invokeTauri<ConversationDelegateStatusSummary[]>(
            "list_conversation_delegate_statuses",
            { input: { conversationId } },
          );
      if (seq !== requestSeq) return;
      delegateStatuses.value = statuses;
      delegateStatusesErrorText.value = "";
    } catch (error) {
      if (seq !== requestSeq) return;
      delegateStatusesErrorText.value = `委托状态加载失败：${String(error)}`;
    }
  }

  function payloadMatchesActiveConversation(payload: DelegateStatusUpdatedPayload | null | undefined) {
    const activeId = String(activeConversationId.value || "").trim();
    if (!activeId) return false;
    const rootConversationId = String(payload?.rootConversationId || "").trim();
    if (rootConversationId) return rootConversationId === activeId;
    return String(payload?.conversationId || "").trim() === activeId;
  }

  function syncPanelState() {
    void refresh();
  }

  async function openDelegateArchiveDetail(status: ConversationDelegateStatusSummary) {
    const conversationId = String(status?.conversationId || status?.delegateId || "").trim();
    if (!conversationId) return;
    try {
      if (typeof window !== "undefined") {
        window.localStorage.setItem(ARCHIVE_FOCUS_REQUEST_STORAGE_KEY, JSON.stringify({
          conversationId,
          viewMode: "delegate",
          createdAt: Date.now(),
        }));
      }
      await invokeTauri("show_archives_window");
    } catch (error) {
      delegateStatusesErrorText.value = `打开委托归档失败：${String(error)}`;
    }
  }

  async function abortDelegate(status: ConversationDelegateStatusSummary) {
    const delegateId = String(status?.delegateId || "").trim();
    if (!delegateId) return;
    try {
      if (hasBridgeRequest()) {
        await options.bridgeRequest!.value!("delegate.abort", { delegateId }, 10000);
      } else {
        await invokeTauri("abort_delegate_conversation", {
          input: { delegateId },
        });
      }
      await refresh();
    } catch (error) {
      delegateStatusesErrorText.value = `打断委托失败：${String(error)}`;
    }
  }

  watch(
    () => [enabled(), panelOpen.value, String(activeConversationId.value || "").trim()],
    () => syncPanelState(),
    { immediate: true },
  );

  onMounted(() => {
    if (isTauriRuntimeAvailable()) {
      void listen<DelegateStatusUpdatedPayload>(DELEGATE_STATUS_UPDATED_EVENT, (event) => {
        if (!panelOpen.value || !payloadMatchesActiveConversation(event.payload)) return;
        void refresh();
      }).then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        delegateStatusUpdatedUnlisten = unlisten;
      }).catch((error) => {
        console.error("[委托状态] 监听器注册失败", error);
      });
    }
    if (typeof window !== "undefined") {
      window.addEventListener(DELEGATE_STATUS_UPDATED_EVENT, handleBridgeDelegateStatusUpdated);
    }
  });

  onBeforeUnmount(() => {
    disposed = true;
    if (delegateStatusUpdatedUnlisten) {
      delegateStatusUpdatedUnlisten();
      delegateStatusUpdatedUnlisten = null;
    }
    if (typeof window !== "undefined") {
      window.removeEventListener(DELEGATE_STATUS_UPDATED_EVENT, handleBridgeDelegateStatusUpdated);
    }
  });

  function handleBridgeDelegateStatusUpdated(event: Event) {
    const payload = (event as CustomEvent<DelegateStatusUpdatedPayload>).detail;
    if (!panelOpen.value || !payloadMatchesActiveConversation(payload)) return;
    void refresh();
  }

  return {
    delegateStatuses,
    delegateStatusesErrorText,
    openDelegateArchiveDetail,
    abortDelegate,
  };
}
