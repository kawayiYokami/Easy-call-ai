import { ref, watch, onBeforeUnmount, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { ConversationDelegateStatusSummary } from "../../../types/app";

const ARCHIVE_FOCUS_REQUEST_STORAGE_KEY = "easy_call.archives.focus_request.v1";
const POLL_INTERVAL_MS = 1000;

interface UseDelegateStatusOptions {
  activeConversationId: Ref<string>;
  panelOpen: Ref<boolean>;
}

export function useDelegateStatus(options: UseDelegateStatusOptions) {
  const { activeConversationId, panelOpen } = options;

  const delegateStatuses = ref<ConversationDelegateStatusSummary[]>([]);
  const delegateStatusesLoading = ref(false);
  const delegateStatusesErrorText = ref("");

  let pollTimer: number | null = null;
  let requestSeq = 0;

  async function refresh() {
    const conversationId = String(activeConversationId.value || "").trim();
    if (!conversationId || !panelOpen.value) {
      delegateStatuses.value = [];
      delegateStatusesErrorText.value = "";
      return;
    }
    const seq = ++requestSeq;
    delegateStatusesLoading.value = true;
    try {
      const statuses = await invokeTauri<ConversationDelegateStatusSummary[]>(
        "list_conversation_delegate_statuses",
        { input: { conversationId } },
      );
      if (seq !== requestSeq) return;
      delegateStatuses.value = statuses;
      delegateStatusesErrorText.value = "";
    } catch (error) {
      if (seq !== requestSeq) return;
      delegateStatusesErrorText.value = `委托状态加载失败：${String(error)}`;
    } finally {
      if (seq === requestSeq) delegateStatusesLoading.value = false;
    }
  }

  function clearPollTimer() {
    if (pollTimer === null) return;
    window.clearInterval(pollTimer);
    pollTimer = null;
  }

  function syncPolling() {
    clearPollTimer();
    if (!panelOpen.value || !String(activeConversationId.value || "").trim()) return;
    void refresh();
    pollTimer = window.setInterval(() => void refresh(), POLL_INTERVAL_MS);
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
      await invokeTauri("abort_delegate_conversation", {
        input: { delegateId },
      });
      await refresh();
    } catch (error) {
      delegateStatusesErrorText.value = `打断委托失败：${String(error)}`;
    }
  }

  watch(
    () => [panelOpen.value, String(activeConversationId.value || "").trim()],
    () => syncPolling(),
    { immediate: true },
  );

  onBeforeUnmount(() => clearPollTimer());

  return {
    delegateStatuses,
    delegateStatusesLoading,
    delegateStatusesErrorText,
    openDelegateArchiveDetail,
    abortDelegate,
  };
}
