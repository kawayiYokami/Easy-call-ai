import { ref, shallowRef } from "vue";
import { describe, expect, it } from "vitest";
import type { AssistantStreamBlock, ChatMessage } from "../../../types/app";
import { mergeAuthoritativeConversationMessages } from "./chat-message-state-machine";
import { useChatFlowDrafts } from "./use-chat-flow-drafts";

function createRuntime() {
  const allMessages = shallowRef<ChatMessage[]>([]);
  const runtime = useChatFlowDrafts({
    allMessages,
    latestUserText: ref(""),
    getConversationId: () => "conversation-1",
    getActiveRoundAgentId: () => "agent-1",
    getSendStartedAtMs: () => 0,
    getActiveHistoryMessageCount: () => allMessages.value.length,
    getFrontendDispatchStartedAtMs: () => 0,
    currentFrontendDispatchElapsedMs: () => 0,
  });
  return { allMessages, runtime };
}

describe("useChatFlowDrafts shared message projection", () => {
  it("routes APP deltas and completion through the shared state machine", () => {
    const { allMessages, runtime } = createRuntime();
    runtime.insertStreamingAssistantMessage("assistant-1", 1);
    const stableRenderId = allMessages.value[0].providerMeta?._stableRenderId;

    runtime.applyAssistantEventToMessage("assistant-1", { delta: "流式正文" });
    runtime.finalizeMessage("assistant-1", {
      id: "assistant-1",
      role: "assistant",
      parts: [{ type: "text", text: "正式正文" }],
      providerMeta: {
        planCard: { action: "present", path: ".pai/plan/example.md" },
      },
    });

    expect(allMessages.value).toHaveLength(1);
    expect(allMessages.value[0].contentBlocks?.[0]?.text).toBe("流式正文");
    expect(allMessages.value[0].providerMeta?._streaming).toBeUndefined();
    expect(allMessages.value[0].providerMeta?._stableRenderId).toBe(stableRenderId);
    expect(allMessages.value[0].providerMeta?.planCard).toEqual({
      action: "present",
      path: ".pai/plan/example.md",
    });
  });

  it("keeps visible failed content but removes an empty failed bubble", () => {
    const visible = createRuntime();
    visible.runtime.insertStreamingAssistantMessage("assistant-visible", 1);
    visible.runtime.applyAssistantDeltaToMessage("assistant-visible", "保留正文");
    visible.runtime.failMessage("assistant-visible", new Error("失败"));

    expect(visible.allMessages.value[0].contentBlocks?.[0]?.text).toBe("保留正文");
    expect(visible.allMessages.value[0].providerMeta?._streaming).toBeUndefined();

    const empty = createRuntime();
    empty.runtime.insertStreamingAssistantMessage("assistant-empty", 1);
    empty.runtime.failMessage("assistant-empty", new Error("失败"));

    expect(empty.allMessages.value).toEqual([]);
  });

  it("stopping settles a thinking and tool projection without losing its content blocks", () => {
    const { allMessages, runtime } = createRuntime();
    runtime.insertStreamingAssistantMessage("assistant-tool-blocks", 1);
    runtime.updateMessageText("assistant-tool-blocks", [{
      reasoning: "先读取配置。",
      tools: [{
        toolCallId: "call-1",
        name: "read_file",
        argsText: "{\"path\":\"app.toml\"}",
        resultText: "配置已读取",
        status: "done",
      }],
    }]);

    runtime.settleStreamingAssistantMessages();

    expect(allMessages.value).toHaveLength(1);
    expect(allMessages.value[0].providerMeta?._streaming).toBeUndefined();
    expect(allMessages.value[0].contentBlocks?.[0]).toMatchObject({
      reasoning: "先读取配置。",
      tools: [expect.objectContaining({
        toolCallId: "call-1",
        name: "read_file",
        argsText: "{\"path\":\"app.toml\"}",
        resultText: "配置已读取",
        status: "done",
      })],
    });
  });

  it("keeps completion metadata when an external same-id formal message arrives first", () => {
    const { allMessages, runtime } = createRuntime();
    runtime.insertStreamingAssistantMessage("assistant-1", 1);
    runtime.applyAssistantDeltaToMessage("assistant-1", "流式正文");
    allMessages.value = mergeAuthoritativeConversationMessages(allMessages.value, [{
      id: "assistant-1",
      role: "assistant",
      parts: [{ type: "text", text: "正式正文" }],
    }]);

    runtime.finalizeMessage("assistant-1", {
      id: "assistant-1",
      role: "assistant",
      parts: [{ type: "text", text: "迟到正式正文" }],
      providerMeta: {
        planCard: { action: "present", path: ".pai/plan/example.md" },
      },
    });

    expect(allMessages.value[0].contentBlocks?.[0]?.text).toBe("流式正文");
    expect(allMessages.value[0].providerMeta?.planCard).toEqual({
      action: "present",
      path: ".pai/plan/example.md",
    });
    expect(allMessages.value[0].providerMeta?._streaming).toBeUndefined();
  });

  it("keeps round identity after an authoritative same-id message arrives before terminal", () => {
    const { allMessages, runtime } = createRuntime();
    runtime.insertStreamingAssistantMessage("assistant-1", 1);
    runtime.applyAssistantEventToMessage("assistant-1", {
      activationId: "activation-new",
      requestId: "request-new",
      delta: "新轮正文",
    });
    allMessages.value = mergeAuthoritativeConversationMessages(allMessages.value, [{
      id: "assistant-1",
      role: "assistant",
      parts: [{ type: "text", text: "正式正文" }],
    }]);

    runtime.finalizeMessage("assistant-1", {
      id: "assistant-1",
      role: "assistant",
      parts: [{ type: "text", text: "旧轮正文" }],
      providerMeta: { planCard: { action: "present", path: ".pai/plan/old.md" } },
    }, {
      activationId: "activation-old",
      requestId: "request-old",
    });

    expect(allMessages.value[0].providerMeta?.planCard).toBeUndefined();

    runtime.finalizeMessage("assistant-1", {
      id: "assistant-1",
      role: "assistant",
      parts: [{ type: "text", text: "新轮正式正文" }],
      providerMeta: { planCard: { action: "present", path: ".pai/plan/new.md" } },
    }, {
      activationId: "activation-new",
      requestId: "request-new",
    });

    expect(allMessages.value[0].providerMeta?.planCard).toEqual({
      action: "present",
      path: ".pai/plan/new.md",
    });
  });
});
