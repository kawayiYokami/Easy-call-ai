import { i18n } from "../../../i18n";
import { invokeTauri } from "../../../services/tauri-api";
import type { AppConfig } from "../../../types/app";
import { resolveModelRoleApiConfigId } from "../../config/utils/model-role-options";
import type { Ref } from "vue";

const t = i18n.global.t;

type ConversationPreferredModelBindings = {
  config: AppConfig;
  currentChatConversationId: Ref<string>;
  currentChatPreferredApiConfigId: Ref<string>;
  detachedTemporaryApiConfigId: Ref<string>;
  setStatus: (value: string) => void;
  setStatusError: (key: string, error: unknown) => void;
  isTextRequestFormat?: (format: string) => boolean;
};

export function useConversationPreferredModel(bindings: ConversationPreferredModelBindings) {
  const preferredModelPersistPending = new Map<string, Promise<boolean>>();

  function updateConversationPreferredApiConfigId(value: string) {
    void updateConversationPreferredApiConfig(value);
  }

  async function waitPendingConversationPreferredModelPersist(conversationId?: string | null): Promise<boolean> {
    const cid = String(conversationId || bindings.currentChatConversationId.value || "").trim();
    if (!cid) return true;
    const pending = preferredModelPersistPending.get(cid);
    return pending ? await pending : true;
  }

  function currentConversationPreferredModelId(conversationId: string): string {
    const cid = String(conversationId || "").trim();
    if (!cid || cid !== String(bindings.currentChatConversationId.value || "").trim()) return "";
    return String(bindings.currentChatPreferredApiConfigId.value || "").trim();
  }

  async function updateConversationPreferredApiConfig(value: string) {
    const nextId = String(value || "").trim();
    const resolvedId = resolveModelRoleApiConfigId(nextId, bindings.config);
    if (nextId && !bindings.config.apiConfigs.some((item: any) =>
      item.id === resolvedId
      && item.enableText
      && (!bindings.isTextRequestFormat || bindings.isTextRequestFormat(item.requestFormat))
    )) {
      bindings.setStatus(t("chat.localTools.modelNotAvailable"));
      return;
    }
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!conversationId) {
      bindings.setStatus(t("chat.localTools.noSwitchableSession"));
      return;
    }
    const previousId = String(bindings.currentChatPreferredApiConfigId.value || "").trim();
    if (previousId === nextId) return;
    bindings.currentChatPreferredApiConfigId.value = nextId;
    bindings.detachedTemporaryApiConfigId.value = "";
    let persist!: Promise<boolean>;
    persist = (async () => {
      try {
        await invokeTauri("set_conversation_preferred_model", {
          input: {
            conversationId,
            preferredApiConfigId: nextId || null,
          },
        });
      } catch (error) {
        console.error("[会话首选模型] 保存失败，准备回滚", {
          conversationId,
          preferredApiConfigId: nextId || null,
          previousPreferredApiConfigId: previousId || null,
          error,
        });
        const isLatestPersist = preferredModelPersistPending.get(conversationId) === persist;
        const currentPreferredId = currentConversationPreferredModelId(conversationId);
        if (isLatestPersist && currentPreferredId === nextId) {
          bindings.currentChatPreferredApiConfigId.value = previousId;
        }
        bindings.setStatusError("status.saveConfigFailed", error);
        return false;
      } finally {
        if (preferredModelPersistPending.get(conversationId) === persist) {
          preferredModelPersistPending.delete(conversationId);
        }
      }
      bindings.setStatus(t("chat.localTools.modelSwitched"));
      return true;
    })();
    preferredModelPersistPending.set(conversationId, persist);
    await persist;
  }

  return {
    updateConversationPreferredApiConfigId,
    updateConversationPreferredApiConfig,
    waitPendingConversationPreferredModelPersist,
  };
}
