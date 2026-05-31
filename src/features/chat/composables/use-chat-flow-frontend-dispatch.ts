import type { Ref } from "vue";
import type { ChatMessage } from "../../../types/app";
import type { ConversationStreamCache } from "./use-chat-flow-stream-cache";
import { positiveRoundedNumber } from "./use-chat-flow-utils";

type UseChatFlowFrontendDispatchOptions = {
  allMessages: Ref<ChatMessage[]>;
  getDraftIdForGen: (gen: number) => string;
  isRoundActiveForGen: (gen: number) => boolean;
  syncCurrentDisplayStateToConversationStreamCache: () => void;
};

export function useChatFlowFrontendDispatch(options: UseChatFlowFrontendDispatchOptions) {
  let timer: ReturnType<typeof setInterval> | null = null;
  let timerGen = 0;
  let startedAtMs = 0;
  let elapsedMs = 0;

  function getStartedAtMs(): number {
    return startedAtMs;
  }

  function getElapsedMs(): number {
    return elapsedMs;
  }

  function currentElapsedMs(): number {
    if (startedAtMs > 0) {
      elapsedMs = Math.max(0, Date.now() - startedAtMs);
    }
    return positiveRoundedNumber(elapsedMs);
  }

  function clear() {
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
    timerGen = 0;
    startedAtMs = 0;
    elapsedMs = 0;
  }

  function updateDraftMeta(gen: number) {
    if (!gen || startedAtMs <= 0) return;
    const nextElapsedMs = currentElapsedMs();
    const draftId = options.getDraftIdForGen(gen);
    let touched = false;
    options.allMessages.value = options.allMessages.value.map((message) => {
      if (message.id !== draftId) return message;
      touched = true;
      const meta = ((message.providerMeta || {}) as Record<string, unknown>);
      return {
        ...message,
        providerMeta: {
          ...meta,
          _frontendDispatchStartedAtMs: startedAtMs,
          _frontendDispatchElapsedMs: nextElapsedMs,
        },
      };
    });
    if (touched) {
      options.syncCurrentDisplayStateToConversationStreamCache();
    }
  }

  function start(gen: number, nextStartedAtMs?: number, nextElapsedMs?: number) {
    const normalizedGen = Math.max(0, Math.round(Number(gen || 0)));
    if (!normalizedGen) return;
    const normalizedStartedAtMs = positiveRoundedNumber(nextStartedAtMs) || Date.now();
    if (
      timer
      && timerGen === normalizedGen
      && startedAtMs === normalizedStartedAtMs
    ) {
      updateDraftMeta(normalizedGen);
      return;
    }
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
    timerGen = normalizedGen;
    startedAtMs = normalizedStartedAtMs;
    elapsedMs = positiveRoundedNumber(nextElapsedMs);
    updateDraftMeta(normalizedGen);
    timer = setInterval(() => {
      if (timerGen !== normalizedGen || !options.isRoundActiveForGen(normalizedGen)) {
        clear();
        return;
      }
      updateDraftMeta(normalizedGen);
    }, 1000);
  }

  function restoreFromCache(cache: ConversationStreamCache, gen: number) {
    const cachedStartedAtMs = positiveRoundedNumber(cache.frontendDispatchStartedAtMs);
    const cachedElapsedMs = positiveRoundedNumber(cache.frontendDispatchElapsedMs);
    if (!cachedStartedAtMs && !cachedElapsedMs) return;
    if (!gen) {
      startedAtMs = cachedStartedAtMs;
      elapsedMs = cachedElapsedMs;
      return;
    }
    start(gen, cachedStartedAtMs || Date.now() - cachedElapsedMs, cachedElapsedMs);
  }

  return {
    clear,
    currentElapsedMs,
    getElapsedMs,
    getStartedAtMs,
    restoreFromCache,
    start,
  };
}
