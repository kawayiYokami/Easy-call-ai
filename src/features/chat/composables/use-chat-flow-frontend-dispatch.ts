import { shallowReactive } from "vue";
import type { ConversationStreamCache } from "./use-chat-flow-stream-cache";
import { positiveRoundedNumber } from "./use-chat-flow-utils";

// 流式耗时数字独立于消息对象存储：计时器每秒更新不应重建 allMessages 数组、
// 破坏消息块签名缓存，否则整个虚拟列表会被每秒空转重算。
// 显示层（ChatMessageItem）直接读这个 map，按消息 id 索引。
export const frontendDispatchElapsedByMessageId = shallowReactive(new Map<string, number>());

type UseChatFlowFrontendDispatchOptions = {
  getMessageIdForGen: (gen: number) => string;
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
    frontendDispatchElapsedByMessageId.clear();
  }

  function updateMessageMeta(gen: number) {
    if (!gen || startedAtMs <= 0) return;
    const nextElapsedMs = currentElapsedMs();
    const messageId = options.getMessageIdForGen(gen);
    if (!messageId) return;
    frontendDispatchElapsedByMessageId.set(messageId, nextElapsedMs);
    options.syncCurrentDisplayStateToConversationStreamCache();
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
      updateMessageMeta(normalizedGen);
      return;
    }
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
    timerGen = normalizedGen;
    startedAtMs = normalizedStartedAtMs;
    elapsedMs = positiveRoundedNumber(nextElapsedMs);
    updateMessageMeta(normalizedGen);
    timer = setInterval(() => {
      if (timerGen !== normalizedGen || !options.isRoundActiveForGen(normalizedGen)) {
        clear();
        return;
      }
      updateMessageMeta(normalizedGen);
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
