import { type Ref, type ShallowRef } from "vue";
import { i18n } from "../../../i18n";
import { invokeTauri } from "../../../services/tauri-api";
import type { ChatMentionTarget, ChatMessage } from "../../../types/app";

const t = i18n.global.t;

type RewindConversationResult = {
  removedCount: number;
  remainingCount: number;
  recalledUserMessage?: ChatMessage;
};

type RecallConfirmMode = "with_patch" | "message_only" | "cancel";

type UseChatRewindActionsOptions = {
  activeApiConfigId: Ref<string>;
  activeAgentId: Ref<string>;
  currentConversationId: Ref<string>;
  allMessages: ShallowRef<ChatMessage[]>;
  maybeUpdateConversationOverviewFromLoadedMessages: (
    conversationId: string,
    messages: ChatMessage[],
    remainingCount: number,
  ) => void;
  chatting: Ref<boolean>;
  trimming: Ref<boolean>;
  compactingConversation: Ref<boolean>;
  chatErrorText: Ref<string>;
  chatInput: Ref<string>;
  selectedMentions: Ref<ChatMentionTarget[]>;
  clipboardImages: Ref<Array<{ mime: string; bytesBase64: string; savedPath?: string }>>;
  deleteUnarchivedConversationFromArchives: (conversationId: string) => Promise<void>;
  sendChat: () => Promise<void>;
  setStatusError: (key: string, error: unknown) => void;
  setChatErrorText: (text: string) => void;
  removeBinaryPlaceholders: (text: string) => string;
  messageText: (message: ChatMessage) => string;
  extractMessageImages: (message: ChatMessage) => Array<{ mime: string; bytesBase64?: string; mediaRef?: string }>;
  requestRecallMode: (payload: { turnId: string; targetUserMessageId: string }) => Promise<RecallConfirmMode>;
  requestCreateConversationBranchFromMessageConfirm: (payload: { turnId: string; targetUserMessageId: string }) => Promise<boolean>;
  createConversationBranchFromMessage: (payload: { turnId: string; targetUserMessageId: string }) => Promise<void>;
  branchingConversation: Ref<boolean>;
  refreshForegroundConversationAfterRewind: (conversationId: string) => Promise<void>;
};

export function useChatRewindActions(options: UseChatRewindActionsOptions) {
  let rewindInFlight = false;

  function extractRecallableImages(message: ChatMessage): Array<{ mime: string; bytesBase64: string; savedPath?: string }> {
    return options.extractMessageImages(message)
      .filter((image) => !!String(image.bytesBase64 || "").trim())
      .map((image) => ({
        mime: image.mime,
        bytesBase64: String(image.bytesBase64 || "").trim(),
      }));
  }

  function errorText(error: unknown): string {
    if (typeof error === "string") return error;
    if (error && typeof error === "object") {
      const maybeMessage = (error as { message?: unknown }).message;
      if (typeof maybeMessage === "string" && maybeMessage.trim()) return maybeMessage.trim();
      try {
        return JSON.stringify(error);
      } catch {
        return String(error);
      }
    }
    return String(error);
  }

  function extractRecallableMentions(message: ChatMessage): ChatMentionTarget[] {
    const providerMeta = (message.providerMeta || {}) as Record<string, unknown>;
    const messageMeta = ((providerMeta.message_meta || providerMeta.messageMeta || {}) as Record<string, unknown>);
    const raw = Array.isArray(messageMeta.mentions) ? messageMeta.mentions : [];
    const seen = new Set<string>();
    const mentions: ChatMentionTarget[] = [];
    for (const item of raw) {
      if (!item || typeof item !== "object") continue;
      const entry = item as Record<string, unknown>;
      const agentId = String(entry.agentId || "").trim();
      const departmentId = String(entry.departmentId || "").trim();
      if (!agentId || !departmentId) continue;
      const dedupKey = `${agentId}::${departmentId}`;
      if (seen.has(dedupKey)) continue;
      seen.add(dedupKey);
      mentions.push({
        agentId,
        agentName: String(entry.agentName || agentId).trim() || agentId,
        departmentId,
        departmentName: String(entry.departmentName || departmentId).trim() || departmentId,
        avatarUrl: undefined,
      });
    }
    return mentions;
  }

  function resolveRewindTargetUserMessage(currentMessages: ChatMessage[], turnId: string): { targetUserMessageId: string; keepCountFromLocal: number } | null {
    const turnMessageId = String(turnId || "").trim();
    if (!turnMessageId) return null;
    const directIndex = currentMessages.findIndex((item) => item.id === turnMessageId);
    if (directIndex < 0) return null;
    const directRole = String(currentMessages[directIndex]?.role || "").trim();
    if (directRole === "user") {
      return {
        targetUserMessageId: turnMessageId,
        keepCountFromLocal: directIndex,
      };
    }
    for (let i = directIndex - 1; i >= 0; i -= 1) {
      if (String(currentMessages[i]?.role || "").trim() === "user") {
        return {
          targetUserMessageId: String(currentMessages[i]?.id || "").trim(),
          keepCountFromLocal: i,
        };
      }
    }
    return null;
  }

  async function rewindConversationFromMessageId(messageId: string, undoApplyPatch: boolean): Promise<ChatMessage | null> {
    const startedAt = Date.now();
    const conversationId = String(options.currentConversationId.value || "").trim();
    console.info("[会话撤回] 开始执行", {
      messageId,
      undoApplyPatch,
      conversationId: conversationId || "(empty)",
    });
    if (!conversationId) {
      console.warn("[会话撤回] 失败：缺少 conversationId");
      options.setChatErrorText(t('dialogs.rewind.failedMissingAgentId'));
      options.setStatusError("status.rewindConversationFailed", t('dialogs.rewind.failedMissingAgentId'));
      return null;
    }
    if (!messageId) {
      console.warn("[会话撤回] 失败：缺少 messageId");
      options.setChatErrorText(t('dialogs.rewind.failedNoTarget'));
      options.setStatusError("status.rewindConversationFailed", t('dialogs.rewind.failedNoTarget'));
      return null;
    }
    try {
      console.info("[会话撤回] 调用后端命令", {
        command: "rewind_conversation_from_message",
        messageId,
        undoApplyPatch,
      });
      const result = await invokeTauri<RewindConversationResult>("rewind_conversation_from_message", {
        input: {
          session: {
            agentId: "",
            conversationId,
          },
          messageId,
          undoApplyPatch,
        },
      });
      if (conversationId) {
        await options.refreshForegroundConversationAfterRewind(conversationId);
      }
      console.info("[会话撤回] 完成", {
        removedCount: Number(result.removedCount) || 0,
        remainingCount: Number(result.remainingCount) || 0,
        elapsedMs: Date.now() - startedAt,
      });
      const currentMessages = [...options.allMessages.value];
      return result.recalledUserMessage
        ?? currentMessages.find((item) => item.id === messageId)
        ?? null;
    } catch (error) {
      const detail = errorText(error);
      console.error("[会话撤回] 失败：后端命令异常", {
        messageId,
        undoApplyPatch,
        elapsedMs: Date.now() - startedAt,
        error: detail,
      });
      options.setStatusError(
        "status.rewindConversationFailed",
        t('dialogs.rewind.failedBackendError', { error: detail || t('dialogs.rewind.failedBackendError') }),
      );
      options.setChatErrorText(t('dialogs.rewind.failedBackendError', { error: detail || t('dialogs.rewind.failedBackendError') }));
      return null;
    }
  }

  async function deleteUnarchivedConversation(conversationId: string) {
    await options.deleteUnarchivedConversationFromArchives(conversationId);
    if (String(options.currentConversationId.value || "").trim() === String(conversationId || "").trim()) {
      options.currentConversationId.value = "";
      options.allMessages.value = [];
    }
  }

  async function handleRecallTurn(payload: { turnId: string }) {
    console.info("[会话撤回] 点击撤回", {
      turnId: payload?.turnId,
      chatting: options.chatting.value,
      trimming: options.trimming.value,
      compactingConversation: options.compactingConversation.value,
      rewindInFlight,
    });
    if (rewindInFlight) {
      console.info("[会话撤回] 跳过：已有撤回流程正在进行", { turnId: payload?.turnId });
      return;
    }
    if (options.chatting.value || options.trimming.value || options.compactingConversation.value) {
      const message = t('dialogs.rewind.failedBusy');
      console.info("[会话撤回] 失败：当前会话处于忙碌状态", {
        turnId: payload?.turnId,
        chatting: options.chatting.value,
        trimming: options.trimming.value,
        compactingConversation: options.compactingConversation.value,
      });
      options.setChatErrorText(`撤回失败：${message}`);
      options.setStatusError("status.rewindConversationFailed", message);
      return;
    }
    rewindInFlight = true;
    try {
      const currentMessages = [...options.allMessages.value];
      const turnMessageId = String(payload.turnId || "").trim();
      const directIndex = currentMessages.findIndex((item) => item.id === turnMessageId);
      if (directIndex < 0) {
        console.warn("[会话撤回] 失败：未找到目标消息", {
          turnId: payload.turnId,
          messageCount: currentMessages.length,
        });
        options.setChatErrorText(t('dialogs.rewind.failedNoTarget'));
        options.setStatusError("status.rewindConversationFailed", t('dialogs.rewind.failedNoTarget'));
        return;
      }
      const directRole = String(currentMessages[directIndex]?.role || "").trim();
      // 助理消息撤回：直接用助理消息 ID 作为撤回目标，只删除该助理回复及之后的消息
      // 用户消息撤回：直接用用户消息 ID 作为撤回目标（整轮回退），撤回后回填输入框
      const targetMessageId = turnMessageId;
      const targetIsUser = directRole === "user";
      const mode = await options.requestRecallMode({
        turnId: payload.turnId,
        targetUserMessageId: targetMessageId,
      });
      console.info("[会话撤回] 弹窗选择结果", {
        mode,
        turnId: payload.turnId,
        targetMessageId,
        targetIsUser,
      });
      if (mode === "cancel") return;
      options.setChatErrorText("");
      const recalledMessage = await rewindConversationFromMessageId(targetMessageId, mode === "with_patch");
      // 只有撤回的是用户消息时，才回填输入框（用于重新发送）
      if (!targetIsUser) {
        if (!recalledMessage && !options.chatErrorText.value.trim()) {
          console.warn("[会话撤回] 结束：助理消息撤回完成，无需回填", { turnId: payload.turnId, mode });
        }
        return;
      }
      if (!recalledMessage) {
        console.warn("[会话撤回] 结束：未拿到可回填消息", { turnId: payload.turnId, mode });
        if (options.chatErrorText.value.trim()) return;
        const message = mode === "with_patch"
          ? t('dialogs.rewind.failedFileChanged')
          : t('dialogs.rewind.failedNoMessage');
        options.setChatErrorText(message);
        options.setStatusError("status.rewindConversationFailed", `${message}（可查看运行日志）`);
        return;
      }
      options.chatInput.value = options.removeBinaryPlaceholders(options.messageText(recalledMessage));
      options.selectedMentions.value = extractRecallableMentions(recalledMessage);
      options.clipboardImages.value = extractRecallableImages(recalledMessage);
      console.info("[会话撤回] 已回填输入框", {
        textLength: options.chatInput.value.length,
        mentionCount: options.selectedMentions.value.length,
        imageCount: options.clipboardImages.value.length,
        turnId: payload.turnId,
      });
    } catch (error) {
      const detail = errorText(error);
      console.error("[会话撤回] 失败：前端撤回流程异常", {
        turnId: payload?.turnId,
        error: detail,
      });
      options.setChatErrorText(t('dialogs.rewind.failedBackendError', { error: detail || t('dialogs.rewind.failedBackendError') }));
      options.setStatusError("status.rewindConversationFailed", detail);
    } finally {
      rewindInFlight = false;
    }
  }

  async function handleRegenerateTurn(payload: { turnId: string }) {
    if (rewindInFlight) {
      console.info("[重新生成] 跳过：已有撤回/重新生成流程正在进行", { turnId: payload?.turnId });
      return;
    }
    if (options.chatting.value || options.trimming.value || options.compactingConversation.value) {
      const message = t('dialogs.rewind.regenerateBusy');
      options.setChatErrorText(`重新生成失败：${message}`);
      options.setStatusError("status.rewindConversationFailed", message);
      return;
    }
    rewindInFlight = true;
    try {
      // 重新生成必须先映射到上一条用户消息，再撤回到该用户消息并重发
      const currentMessages = [...options.allMessages.value];
      const target = resolveRewindTargetUserMessage(currentMessages, payload.turnId);
      if (!target || !target.targetUserMessageId) {
        console.warn("[重新生成] 失败：未找到可撤回的用户消息", {
          turnId: payload.turnId,
          messageCount: currentMessages.length,
        });
        options.setChatErrorText(t('dialogs.rewind.failedNoTarget'));
        options.setStatusError("status.rewindConversationFailed", t('dialogs.rewind.failedNoTarget'));
        return;
      }
      const recalledUserMessage = await rewindConversationFromMessageId(target.targetUserMessageId, false);
      if (!recalledUserMessage) return;
      options.chatInput.value = options.removeBinaryPlaceholders(options.messageText(recalledUserMessage));
      options.selectedMentions.value = extractRecallableMentions(recalledUserMessage);
      options.clipboardImages.value = extractRecallableImages(recalledUserMessage);
      await options.sendChat();
    } finally {
      rewindInFlight = false;
    }
  }

  async function handleCreateConversationBranchFromTurn(payload: { turnId: string }) {
    const branchingConversation = options.branchingConversation;
    if (rewindInFlight || !!branchingConversation?.value) {
      return;
    }
    if (options.chatting.value || options.trimming.value || options.compactingConversation.value) {
      const message = t('dialogs.rewind.failedBusy');
      options.setChatErrorText(message);
      options.setStatusError("status.createBranchFailed", message);
      return;
    }
    const currentMessages = [...options.allMessages.value];
    const target = resolveRewindTargetUserMessage(currentMessages, payload.turnId);
    if (!target || !target.targetUserMessageId) {
      options.setChatErrorText(t('dialogs.rewind.failedNoTarget'));
      options.setStatusError("status.createBranchFailed", t('dialogs.rewind.failedNoTarget'));
      return;
    }
    if (
      typeof options.requestCreateConversationBranchFromMessageConfirm !== "function"
      || typeof options.createConversationBranchFromMessage !== "function"
    ) {
      options.setChatErrorText(t('sidebar.createBranchFailed'));
      options.setStatusError("status.createBranchFailed", t('sidebar.createBranchFailed'));
      return;
    }
    const confirmed = await options.requestCreateConversationBranchFromMessageConfirm({
      turnId: payload.turnId,
      targetUserMessageId: target.targetUserMessageId,
    });
    if (!confirmed) return;
    try {
      await options.createConversationBranchFromMessage({
        turnId: payload.turnId,
        targetUserMessageId: target.targetUserMessageId,
      });
    } catch (error) {
      const detail = errorText(error);
      options.setChatErrorText(detail);
      options.setStatusError("status.createBranchFailed", detail);
    }
  }

  return {
    handleRecallTurn,
    handleCreateConversationBranchFromTurn,
    handleRegenerateTurn,
    deleteUnarchivedConversation,
  };
}
