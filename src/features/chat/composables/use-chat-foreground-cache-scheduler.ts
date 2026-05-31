import type { ChatMessage } from "../../../types/app";

type UseChatForegroundCacheSchedulerOptions = {
  cacheConversationMessages: (conversationId: string, messages: ChatMessage[]) => void;
};

export function useChatForegroundCacheScheduler(options: UseChatForegroundCacheSchedulerOptions) {
  let foregroundConversationCacheRaf = 0;

  function clearForegroundConversationCacheRaf() {
    if (!foregroundConversationCacheRaf) return;
    cancelAnimationFrame(foregroundConversationCacheRaf);
    foregroundConversationCacheRaf = 0;
  }

  function scheduleForegroundConversationCachePersist(conversationId: string, nextMessages: ChatMessage[]) {
    clearForegroundConversationCacheRaf();
    foregroundConversationCacheRaf = requestAnimationFrame(() => {
      foregroundConversationCacheRaf = 0;
      options.cacheConversationMessages(conversationId, nextMessages);
    });
  }

  return {
    clearForegroundConversationCacheRaf,
    scheduleForegroundConversationCachePersist,
    getForegroundConversationCacheRaf: () => foregroundConversationCacheRaf,
  };
}
