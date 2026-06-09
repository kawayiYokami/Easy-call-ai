import { describe, expect, it, vi } from "vitest";
import { ref, shallowRef } from "vue";
import type { ChatMentionTarget, ChatMessage } from "../src/types/app";
import { useChatRewindActions } from "../src/features/chat/composables/use-chat-rewind-actions";

const hoisted = vi.hoisted(() => ({
  invokeTauriMock: vi.fn(),
}));

vi.mock("../src/services/tauri-api", () => ({
  invokeTauri: hoisted.invokeTauriMock,
}));

function textMessage(id: string, role: ChatMessage["role"], text: string): ChatMessage {
  return {
    id,
    role,
    parts: [{ type: "text", text }],
  };
}

function buildRewindActions(overrides: {
  chatting?: boolean;
  trimming?: boolean;
  compacting?: boolean;
  messages?: ChatMessage[];
  requestRecallMode?: (payload: { turnId: string; targetUserMessageId: string }) => Promise<"message_only" | "with_patch" | "cancel">;
} = {}) {
  const allMessages = shallowRef<ChatMessage[]>(
    overrides.messages ?? [
      textMessage("user-1", "user", "第一句"),
      textMessage("assistant-1", "assistant", "回复"),
      textMessage("user-2", "user", "需要撤回"),
      textMessage("assistant-2", "assistant", "后续回复"),
    ],
  );
  const chatErrorText = ref("");
  const statusErrors: unknown[] = [];
  const actions = useChatRewindActions({
    activeApiConfigId: ref("api-a"),
    activeAgentId: ref("agent-a"),
    currentConversationId: ref("conversation-a"),
    allMessages,
    maybeUpdateConversationOverviewFromLoadedMessages: vi.fn(),
    chatting: ref(Boolean(overrides.chatting)),
    trimming: ref(Boolean(overrides.trimming)),
    compactingConversation: ref(Boolean(overrides.compacting)),
    chatErrorText,
    chatInput: ref(""),
    selectedMentions: ref<ChatMentionTarget[]>([]),
    clipboardImages: ref([]),
    deleteUnarchivedConversationFromArchives: vi.fn(),
    sendChat: vi.fn(),
    setStatusError: (_key, error) => statusErrors.push(error),
    setChatErrorText: (text) => {
      chatErrorText.value = text;
    },
    removeBinaryPlaceholders: (text) => text,
    messageText: (message) => String(message.parts?.[0]?.text || ""),
    extractMessageImages: () => [],
    requestRecallMode: vi.fn(overrides.requestRecallMode ?? (async () => "message_only")),
    refreshForegroundConversationAfterRewind: vi.fn(),
  });
  return { actions, allMessages, chatErrorText, statusErrors };
}

describe("useChatRewindActions", () => {
  it("does not call backend or mutate messages when conversation is busy", async () => {
    hoisted.invokeTauriMock.mockReset();
    const { actions, allMessages, chatErrorText } = buildRewindActions({ chatting: true });
    const before = allMessages.value;

    await actions.handleRecallTurn({ turnId: "assistant-2" });

    expect(hoisted.invokeTauriMock).not.toHaveBeenCalled();
    expect(allMessages.value).toBe(before);
    expect(chatErrorText.value).toContain("当前会话正在运行或整理上下文");
  });

  it("keeps local messages unchanged when backend rejects rewind", async () => {
    hoisted.invokeTauriMock.mockReset();
    hoisted.invokeTauriMock.mockRejectedValueOnce("当前会话正在运行或整理上下文，完成后再撤回。");
    const { actions, allMessages, chatErrorText } = buildRewindActions();
    const before = allMessages.value;

    await actions.handleRecallTurn({ turnId: "assistant-2" });

    expect(hoisted.invokeTauriMock).toHaveBeenCalledWith(
      "rewind_conversation_from_message",
      expect.any(Object),
    );
    expect(allMessages.value).toBe(before);
    expect(chatErrorText.value).toContain("当前会话正在运行或整理上下文");
  });

  it("ignores duplicate rewind clicks while confirmation is pending", async () => {
    hoisted.invokeTauriMock.mockReset();
    let resolveMode: ((mode: "message_only") => void) | null = null;
    const requestRecallMode = vi.fn((_payload: { turnId: string; targetUserMessageId: string }) => new Promise<"message_only">((resolve) => {
      resolveMode = resolve;
    }));
    const { actions } = buildRewindActions({ requestRecallMode });

    const first = actions.handleRecallTurn({ turnId: "assistant-2" });
    const second = actions.handleRecallTurn({ turnId: "assistant-2" });
    resolveMode?.("message_only");
    await Promise.all([first, second]);

    expect(requestRecallMode).toHaveBeenCalledTimes(1);
    expect(requestRecallMode).toHaveBeenCalledWith({
      turnId: "assistant-2",
      targetUserMessageId: "user-2",
    });
    expect(hoisted.invokeTauriMock).toHaveBeenCalledTimes(1);
  });
});
