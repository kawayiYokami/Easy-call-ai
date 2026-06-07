import {
  assistantEventHasVisibleProgress,
  readAssistantEvent,
  readDeltaMessage,
  readHistoryFlushedPayload,
  readRoundCompletedPayload,
  readRoundFailedPayload,
  readRoundStartedPayload,
} from "./use-chat-flow-events";
import { stringifyExternalEventPayload } from "./use-chat-flow-utils";

type UseChatFlowExternalEventsOptions = {
  debug?: boolean;
  getCurrentConversationId: () => string;
  getActiveActivationId: () => string;
  setActiveActivationId: (value: string) => void;
  getRound: () => { phase: "idle" } | { phase: "queued"; gen: number } | { phase: "streaming"; gen: number; draftId: string };
  getSendChatActiveGen: () => number;
  nextGeneration: () => number;
  channelBinding: {
    bindActiveConversationStream: (conversationId: string, force?: boolean) => Promise<void>;
    hasActiveBoundDeltaChannel: (conversationId?: string | null) => boolean;
    setBoundDisplayGeneration: (gen: number) => void;
  };
  handleHistoryFlushed: (gen: number, parsed: any, source: "sendChat" | "bound") => Promise<void>;
  beginAssistantActivationFromEvent: (payload: any) => number;
  markRoundStarted: (gen: number) => Promise<void>;
  payloadMatchesActiveActivation: (payload: { activationId?: string; requestId?: string } | null | undefined) => boolean;
  handleRoundCompleted: (gen: number, result: any) => void;
  handleRoundFailed: (gen: number, error: unknown) => Promise<void>;
  clearConversationStreamCache: (conversationId?: string | null) => void;
  clearFrontendDispatchTimer: () => void;
  onReloadMessages: () => Promise<void>;
  setChatErrorText: (text: string, conversationId?: string | null) => void;
  formatRequestFailed: (error: unknown) => string;
  latestAssistantText: { value: string };
  chatting: { value: boolean };
  reasoningStartedAtMs: { value: number };
  applyAssistantEventToConversationStreamCache: (conversationId: string, parsed: any) => boolean;
  writeConversationStreamCacheSnapshot: (conversationId: string, snapshot?: any) => void;
  applyConversationStreamCacheToDisplay: (
    conversationId?: string | null,
    input?: { ignoreActivationId?: boolean; skipStreamBlocks?: boolean },
  ) => boolean;
  hasAssistantDraftInMessages: () => boolean;
  ensureForegroundStreamingRound: () => number;
  handleStreamingEvent: (gen: number, parsed: any) => void;
  syncStreamBlocksToDraft: (draftId: string) => void;
  updateDraftText: (draftId: string) => void;
};

export function useChatFlowExternalEvents(options: UseChatFlowExternalEventsOptions) {
  async function handleExternalStreamRebindRequired(payload: unknown) {
    const raw = payload && typeof payload === "object" ? payload as Record<string, unknown> : null;
    const payloadConversationId = String(raw?.conversationId || "").trim();
    const currentConversationId = options.getCurrentConversationId();
    if (!payloadConversationId || !currentConversationId || payloadConversationId !== currentConversationId) {
      return;
    }
    const requestId = String(raw?.requestId || "").trim();
    const phaseId = String(raw?.phaseId || "").trim();
    const reason = String(raw?.reason || "").trim();
    if (options.debug) {
      console.debug("[聊天] 流式通道重绑 开始", {
        conversationId: currentConversationId,
        requestId,
        phaseId,
        reason,
        roundPhase: options.getRound().phase,
      });
    }
    try {
      await options.channelBinding.bindActiveConversationStream(currentConversationId, true);
      if (options.getRound().phase !== "streaming") {
        if (options.debug) {
          console.debug("[聊天流式重绑][前端] 重绑事件触发恢复草稿", {
            conversationId: currentConversationId,
            requestId,
            phaseId,
            reason,
            roundPhase: options.getRound().phase,
          });
        }
        options.ensureForegroundStreamingRound();
      }
      if (options.debug) {
        console.debug("[聊天] 流式通道重绑 完成", {
          conversationId: currentConversationId,
          requestId,
          phaseId,
          reason,
        });
      }
    } catch (error) {
      console.error("[聊天] 流式通道重绑 失败", {
        conversationId: currentConversationId,
        requestId,
        phaseId,
        reason,
        error,
      });
      throw error;
    }
  }

  async function handleExternalHistoryFlushed(payload: unknown) {
    const raw = stringifyExternalEventPayload(payload, "history_flushed");
    const parsed = readHistoryFlushedPayload(raw);
    if (!parsed) return;
    const currentConversationId = options.getCurrentConversationId();
    const payloadConversationId = String(parsed.conversationId || "").trim();
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      return;
    }
    const treatAsSendChat = options.getSendChatActiveGen() > 0 && !!parsed.activateAssistant;
    const source: "sendChat" | "bound" = treatAsSendChat ? "sendChat" : "bound";
    const gen = treatAsSendChat ? options.getSendChatActiveGen() : options.nextGeneration();
    if (!treatAsSendChat) {
      options.channelBinding.setBoundDisplayGeneration(gen);
    }
    await options.handleHistoryFlushed(
      gen,
      {
        kind: "history_flushed",
        message: JSON.stringify(parsed),
      },
      source,
    );
  }

  async function handleExternalRoundStarted(payload: unknown) {
    const raw = stringifyExternalEventPayload(payload, "round_started");
    const parsed = readRoundStartedPayload(raw);
    if (!parsed) return;
    const currentConversationId = options.getCurrentConversationId();
    const payloadConversationId = String(parsed.conversationId || "").trim();
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      return;
    }
    const gen = options.beginAssistantActivationFromEvent(parsed);
    if (!gen) return;
    await options.markRoundStarted(gen);
  }

  async function handleExternalRoundCompleted(payload: unknown) {
    const raw = stringifyExternalEventPayload(payload, "round_completed");
    const parsed = readRoundCompletedPayload(raw);
    if (!parsed) return;
    const currentConversationId = options.getCurrentConversationId();
    const payloadConversationId = String(parsed.conversationId || "").trim();
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      options.clearConversationStreamCache(payloadConversationId);
      return;
    }
    if (!options.payloadMatchesActiveActivation(parsed)) {
      options.clearConversationStreamCache(payloadConversationId || currentConversationId);
      return;
    }
    const round = options.getRound();
    if (round.phase !== "streaming" && round.phase !== "queued") {
      options.chatting.value = false;
      options.reasoningStartedAtMs.value = 0;
      options.clearConversationStreamCache(payloadConversationId || currentConversationId);
      options.clearFrontendDispatchTimer();
      options.setActiveActivationId("");
      await options.onReloadMessages();
      return;
    }
    options.handleRoundCompleted(round.gen, {
      assistantText: String(parsed.assistantText || ""),
      assistantMessage: parsed.assistantMessage,
    });
  }

  async function handleExternalRoundFailed(payload: unknown) {
    const raw = stringifyExternalEventPayload(payload, "round_failed");
    const parsed = readRoundFailedPayload(raw);
    const currentConversationId = options.getCurrentConversationId();
    const payloadConversationId = String(parsed?.conversationId || "").trim();
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      const errorDetail = parsed?.error || raw || String(raw);
      options.setChatErrorText(options.formatRequestFailed(errorDetail), payloadConversationId);
      options.clearConversationStreamCache(payloadConversationId);
      return;
    }
    if (!options.payloadMatchesActiveActivation(parsed)) {
      options.clearConversationStreamCache(payloadConversationId || currentConversationId);
      return;
    }
    const round = options.getRound();
    if (round.phase !== "streaming" && round.phase !== "queued") {
      options.latestAssistantText.value = "";
      options.chatting.value = false;
      options.reasoningStartedAtMs.value = 0;
      options.clearConversationStreamCache(payloadConversationId || currentConversationId);
      options.clearFrontendDispatchTimer();
      options.setActiveActivationId("");
      const errorDetail = parsed?.error || raw || String(raw);
      const errorObj = typeof errorDetail === "string" ? (
        (() => {
          try {
            return JSON.parse(errorDetail);
          } catch {
            return { message: errorDetail };
          }
        })()
      ) : errorDetail;
      console.error("[聊天流程] 非流式轮次失败", {
        roundPhase: round.phase,
        roundGen: null,
        error: errorObj,
        rawPayload: raw,
      });
      options.setChatErrorText(options.formatRequestFailed(errorDetail), payloadConversationId || currentConversationId);
      await options.onReloadMessages();
      return;
    }
    await options.handleRoundFailed(round.gen, parsed?.error || raw || String(raw));
  }

  async function handleExternalAssistantDelta(payload: unknown) {
    const rawObj = payload && typeof payload === "object" ? payload as Record<string, unknown> : null;
    const currentConversationId = options.getCurrentConversationId();
    const payloadConversationId = String(rawObj?.conversationId || "").trim();
    const parsed = readAssistantEvent(rawObj?.event ?? payload);
    const cacheConversationId = payloadConversationId || currentConversationId;
    const eventActivationId = String(parsed.activationId || parsed.requestId || "").trim();
    const activeActivationId = options.getActiveActivationId();
    if (activeActivationId && eventActivationId && eventActivationId !== activeActivationId) {
      console.warn("[聊天流式块][前端外部丢弃] activationId 不一致", {
        currentConversationId,
        payloadConversationId,
        activeActivationId,
        eventActivationId,
        kind: parsed.kind || "delta",
      });
      return;
    }
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      console.warn("[聊天流式块][前端外部缓存] 非当前会话，仅写缓存", {
        currentConversationId,
        payloadConversationId,
        kind: parsed.kind || "delta",
      });
      if (cacheConversationId) {
        if (parsed.streamCache) {
          options.writeConversationStreamCacheSnapshot(cacheConversationId, parsed.streamCache);
          if (parsed.kind === "tool_status") {
            options.applyAssistantEventToConversationStreamCache(cacheConversationId, parsed);
          }
        } else {
          options.applyAssistantEventToConversationStreamCache(cacheConversationId, parsed);
        }
      }
      return;
    }
    // tool_status 是调度层信号，服务头像右侧/运行态提示；它不属于气泡流式结果。
    // 后端将它作为 app-event-only 发送，所以即使 bound channel 已连接也不能在这里去重丢弃。
    if (
      parsed.kind !== "tool_status"
      && assistantEventHasVisibleProgress(parsed)
      && options.channelBinding.hasActiveBoundDeltaChannel(cacheConversationId)
    ) {
      return;
    }
    if (cacheConversationId) {
      if (parsed.streamCache) {
        options.writeConversationStreamCacheSnapshot(cacheConversationId, parsed.streamCache);
        if (parsed.kind === "tool_status") {
          options.applyAssistantEventToConversationStreamCache(cacheConversationId, parsed);
        }
      } else {
        options.applyAssistantEventToConversationStreamCache(cacheConversationId, parsed);
      }
    }
    const round = options.getRound();
    if (parsed.kind === "context_usage_update") {
      const currentGen = round.phase === "streaming" || round.phase === "queued" ? round.gen : 0;
      if (currentGen) {
        options.handleStreamingEvent(currentGen, parsed);
      }
      return;
    }
    const shouldProjectFromAppEvent =
      parsed.kind === "tool_status"
      || round.phase !== "streaming"
      || !options.hasAssistantDraftInMessages();
    const shouldResumeForegroundRound =
      shouldProjectFromAppEvent
      && assistantEventHasVisibleProgress(parsed);
    if (shouldResumeForegroundRound && options.debug) {
      console.debug("[聊天流式重绑][前端] 普通事件触发恢复前景流式", {
        currentConversationId,
        payloadConversationId,
        kind: parsed.kind || "delta",
      });
    }
    if (!shouldProjectFromAppEvent) {
      if (parsed.kind === "activity_reasoning_delta" || parsed.kind === "assistant_tool_event" || parsed.kind === "assistant_tool_result") {
        const delta = readDeltaMessage(parsed);
        if (delta && options.reasoningStartedAtMs.value === 0) {
          options.reasoningStartedAtMs.value = Date.now();
        }
        options.applyConversationStreamCacheToDisplay(cacheConversationId);
        const latestRound = options.getRound();
        if (latestRound.phase === "streaming") {
          options.syncStreamBlocksToDraft(latestRound.draftId);
          options.updateDraftText(latestRound.draftId);
        }
      }
      return;
    }
    const currentGen = shouldResumeForegroundRound
      ? options.ensureForegroundStreamingRound()
      : (round.phase === "streaming" ? round.gen : 0);
    if (!currentGen) {
      console.warn("[聊天流式块][前端外部丢弃] currentGen 为空", {
        currentConversationId,
        payloadConversationId,
        cacheConversationId,
        kind: parsed.kind || "delta",
        round: options.getRound(),
        shouldProjectFromAppEvent,
        shouldResumeForegroundRound,
      });
      return;
    }
    if (parsed.kind === "activity_reasoning_delta") {
      const delta = readDeltaMessage(parsed);
      if (delta && options.reasoningStartedAtMs.value === 0) {
        options.reasoningStartedAtMs.value = Date.now();
      }
    }
    if (parsed.kind === "tool_status") {
      options.applyConversationStreamCacheToDisplay(cacheConversationId, { skipStreamBlocks: true });
      return;
    }
    options.handleStreamingEvent(currentGen, parsed);
  }

  return {
    handleExternalAssistantDelta,
    handleExternalHistoryFlushed,
    handleExternalRoundCompleted,
    handleExternalRoundFailed,
    handleExternalRoundStarted,
    handleExternalStreamRebindRequired,
  };
}
