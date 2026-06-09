import { computed, ref, watch } from "vue";
import type { ComputedRef, Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import { formatI18nError } from "../../../utils/error";
import type { UnarchivedConversationSummary } from "../../../types/app";
import { resolveConversationDisplayTitle } from "../utils/conversation-title";

type TrFn = (key: string, params?: Record<string, unknown>) => string;

type PromptPreviewResult = {
  preamble: string;
  latestUserText: string;
  latestImages: number;
  latestAudios: number;
  requestBodyJson: string;
};

export type RequestPreviewMode = "chat" | "compaction" | "archive";

type SystemPromptPreviewResult = {
  systemPrompt: string;
};

type UsePromptPreviewOptions = {
  t: TrFn;
  currentConversationId: Ref<string>;
  localConversations: ComputedRef<UnarchivedConversationSummary[]>;
};

export function usePromptPreview(options: UsePromptPreviewOptions) {
  const promptPreviewDialog = ref<HTMLDialogElement | null>(null);
  const promptPreviewLoading = ref(false);
  const promptPreviewText = ref("");
  const promptPreviewLatestUserText = ref("");
  const promptPreviewLatestImages = ref(0);
  const promptPreviewLatestAudios = ref(0);
  const promptPreviewMode = ref<RequestPreviewMode | "system" | null>(null);
  const promptPreviewApiConfigId = ref("");
  const promptPreviewAgentId = ref("");
  const promptPreviewConversationId = ref("");
  const promptPreviewConversationOptions = ref<Array<{ conversationId: string; title: string }>>([]);

  function localConversationOptionsFromSource(source: UnarchivedConversationSummary[]) {
    return (source || [])
      .map((item) => {
        const conversationId = String(item.conversationId || "").trim();
        return {
          conversationId,
          title: conversationId
            ? resolveConversationDisplayTitle(
              {
                ...item,
                conversationId,
                kind: "local_unarchived",
              },
              {
                untitledLabel: options.t("chat.untitledConversation"),
              },
            )
            : "",
        };
      })
      .filter((item) => !!item.conversationId);
  }

  async function ensurePromptPreviewConversationOptions() {
    const cached = localConversationOptionsFromSource(options.localConversations.value || []);
    if (cached.length > 0) {
      promptPreviewConversationOptions.value = cached;
      return;
    }
    try {
      const fetched = await invokeTauri<UnarchivedConversationSummary[]>("list_unarchived_conversations");
      promptPreviewConversationOptions.value = localConversationOptionsFromSource(Array.isArray(fetched) ? fetched : []);
    } catch {
      promptPreviewConversationOptions.value = [];
    }
  }

  function buildPreviewSessionInput(apiConfigId: string, agentId: string) {
    const conversationId = String(promptPreviewConversationId.value || "").trim();
    if (!conversationId) {
      throw new Error("conversationId is required.");
    }
    return {
      apiConfigId,
      agentId,
      conversationId,
    };
  }

  function resolveInitialPromptPreviewConversationId() {
    const currentConversationId = String(options.currentConversationId.value || "").trim();
    if (currentConversationId && promptPreviewConversationOptions.value.some((item) => item.conversationId === currentConversationId)) {
      return currentConversationId;
    }
    return String(promptPreviewConversationOptions.value[0]?.conversationId || "").trim();
  }

  function resetPromptPreviewState(mode: RequestPreviewMode | "system" | null) {
    promptPreviewMode.value = mode;
    promptPreviewLoading.value = false;
    promptPreviewText.value = "";
    promptPreviewLatestUserText.value = "";
    promptPreviewLatestImages.value = 0;
    promptPreviewLatestAudios.value = 0;
    promptPreviewConversationId.value = resolveInitialPromptPreviewConversationId();
    promptPreviewDialog.value?.showModal();
  }

  async function openPromptPreview(apiConfigId: string, agentId: string) {
    if (!apiConfigId || !agentId) return;
    promptPreviewApiConfigId.value = apiConfigId;
    promptPreviewAgentId.value = agentId;
    await ensurePromptPreviewConversationOptions();
    resetPromptPreviewState(null);
  }

  async function loadPromptPreview(mode: RequestPreviewMode) {
    if (!promptPreviewApiConfigId.value || !promptPreviewAgentId.value) return;
    promptPreviewMode.value = mode;
    promptPreviewLoading.value = true;
    promptPreviewText.value = "";
    promptPreviewLatestUserText.value = "";
    promptPreviewLatestImages.value = 0;
    promptPreviewLatestAudios.value = 0;
    try {
      const preview = await invokeTauri<PromptPreviewResult>("get_prompt_preview", {
        input: buildPreviewSessionInput(promptPreviewApiConfigId.value, promptPreviewAgentId.value),
        previewMode: mode,
      });
      promptPreviewText.value = preview.requestBodyJson || "";
      promptPreviewLatestUserText.value = preview.latestUserText || "";
      promptPreviewLatestImages.value = Number(preview.latestImages || 0);
      promptPreviewLatestAudios.value = Number(preview.latestAudios || 0);
    } catch (e) {
      promptPreviewText.value = formatI18nError(options.t, "status.loadRequestPreviewFailed", e);
    } finally {
      promptPreviewLoading.value = false;
    }
  }

  async function loadSystemPromptPreview() {
    if (!promptPreviewApiConfigId.value || !promptPreviewAgentId.value) return;
    promptPreviewMode.value = "system";
    promptPreviewLoading.value = true;
    promptPreviewText.value = "";
    try {
      const preview = await invokeTauri<SystemPromptPreviewResult>("get_system_prompt_preview", {
        input: buildPreviewSessionInput(promptPreviewApiConfigId.value, promptPreviewAgentId.value),
      });
      promptPreviewText.value = preview.systemPrompt || "";
    } catch (e) {
      promptPreviewText.value = formatI18nError(options.t, "status.loadSystemPromptFailed", e);
    } finally {
      promptPreviewLoading.value = false;
    }
  }

  async function openSystemPromptPreview(apiConfigId: string, agentId: string) {
    if (!apiConfigId || !agentId) return;
    promptPreviewApiConfigId.value = apiConfigId;
    promptPreviewAgentId.value = agentId;
    await ensurePromptPreviewConversationOptions();
    resetPromptPreviewState("system");
    await loadSystemPromptPreview();
  }

  function closePromptPreview() {
    promptPreviewDialog.value?.close();
  }

  async function selectPromptPreviewConversation(conversationId: string) {
    promptPreviewConversationId.value = String(conversationId || "").trim();
    if (promptPreviewMode.value === "system") {
      await loadSystemPromptPreview();
      return;
    }
    if (promptPreviewMode.value) {
      await loadPromptPreview(promptPreviewMode.value);
    }
  }

  watch(
    () => options.localConversations.value,
    (value) => {
      const next = localConversationOptionsFromSource(value || []);
      if (next.length > 0) {
        promptPreviewConversationOptions.value = next;
        if (!promptPreviewConversationId.value) {
          promptPreviewConversationId.value = resolveInitialPromptPreviewConversationId();
        }
      }
    },
    { deep: true },
  );

  return {
    promptPreviewDialog,
    promptPreviewLoading,
    promptPreviewText,
    promptPreviewLatestUserText,
    promptPreviewLatestImages,
    promptPreviewLatestAudios,
    promptPreviewMode,
    promptPreviewConversationId,
    promptPreviewConversationOptions,
    loadPromptPreview,
    openPromptPreview,
    openSystemPromptPreview,
    selectPromptPreviewConversation,
    closePromptPreview,
  };
}
