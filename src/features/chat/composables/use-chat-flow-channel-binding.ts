import { Channel } from "@tauri-apps/api/core";
import {
  readAssistantEvent,
  type AssistantDeltaEvent,
} from "./use-chat-flow-events";
import { normalizeConversationId } from "./use-chat-flow-utils";

export type ChatFlowDeltaSource = "sendChat" | "bound";

type UseChatFlowChannelBindingOptions = {
  debug?: boolean;
  getConversationId?: () => string;
  invokeBindActiveChatViewStream?: (input: {
    conversationId?: string;
    onDelta: Channel<AssistantDeltaEvent>;
  }) => Promise<void>;
  getRoundStreamingGen: () => number;
  getCurrentGeneration: () => number;
  markHistoryFlushedReceived: (gen: number) => void;
  handleHistoryFlushed: (
    gen: number,
    parsed: AssistantDeltaEvent,
    source: ChatFlowDeltaSource,
  ) => Promise<void>;
  handleStreamingEvent: (gen: number, parsed: AssistantDeltaEvent) => void;
  formatRequestFailed: (error: unknown) => string;
  setChatErrorText: (text: string) => void;
};

export function useChatFlowChannelBinding(options: UseChatFlowChannelBindingOptions) {
  let boundConversationId = "";
  let boundConversationInitialized = false;
  let boundDisplayGeneration = 0;
  let boundDeltaChannel: Channel<AssistantDeltaEvent> | null = null;

  function getBoundDisplayGeneration(): number {
    return boundDisplayGeneration;
  }

  function setBoundDisplayGeneration(gen: number) {
    boundDisplayGeneration = Math.max(0, Math.round(Number(gen || 0)));
  }

  function attachDeltaHandler(
    channel: Channel<AssistantDeltaEvent>,
    source: ChatFlowDeltaSource,
    getGen: () => number,
    nextGenOnHistoryFlushed: () => number,
  ) {
    channel.onmessage = (event) => {
      const parsed = readAssistantEvent(event);

      if (parsed.kind === "history_flushed") {
        const hfGen = nextGenOnHistoryFlushed();
        if (source === "sendChat" && hfGen !== options.getCurrentGeneration()) {
          return;
        }
        if (source === "sendChat") {
          options.markHistoryFlushedReceived(hfGen);
        }
        void options.handleHistoryFlushed(hfGen, parsed, source).catch((err) => {
          console.error("[聊天] history_flushed 处理失败", {
            message: String((err as { message?: string })?.message ?? err ?? ""),
            gen: hfGen,
          });
          options.setChatErrorText(options.formatRequestFailed(err));
        });
        return;
      }

      const currentGen = getGen();
      options.handleStreamingEvent(currentGen, parsed);
    };
  }

  function hasActiveBoundDeltaChannel(conversationId?: string | null): boolean {
    if (!boundDeltaChannel || !boundConversationInitialized) return false;
    const cid = normalizeConversationId(conversationId || (options.getConversationId ? options.getConversationId() : ""));
    const boundCid = normalizeConversationId(boundConversationId);
    return !!cid && !!boundCid && cid === boundCid;
  }

  async function bindActiveConversationStream(conversationId: string, force = false) {
    if (!options.invokeBindActiveChatViewStream) return;
    const id = String(conversationId || "").trim();
    if (!force && boundConversationInitialized && id === boundConversationId) return;
    const channel = new Channel<AssistantDeltaEvent>();
    attachDeltaHandler(
      channel,
      "bound",
      () => options.getRoundStreamingGen() || boundDisplayGeneration,
      () => options.getRoundStreamingGen() || boundDisplayGeneration,
    );
    await options.invokeBindActiveChatViewStream({
      conversationId: id || undefined,
      onDelta: channel,
    });
    boundDeltaChannel = channel;
    boundConversationId = id;
    boundConversationInitialized = true;
    if (!id) boundDisplayGeneration = 0;
    if (options.debug) {
      console.debug("[聊天] 已绑定前台流式通道", { conversationId: id });
    }
  }

  return {
    attachDeltaHandler,
    bindActiveConversationStream,
    getBoundDisplayGeneration,
    hasActiveBoundDeltaChannel,
    setBoundDisplayGeneration,
  };
}
