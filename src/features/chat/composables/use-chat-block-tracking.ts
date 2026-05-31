import { computed, type Ref } from "vue";
import type { ChatMessageBlock } from "../../../types/app";
import { type ChatRenderItem, isRightAlignedMessage } from "../utils/chat-render";

export function useChatBlockTracking(
  messageBlocks: Ref<ChatMessageBlock[]>,
  chatRenderItems: Ref<ChatRenderItem[]>,
) {
  function isOwnMessage(block: ChatMessageBlock): boolean {
    return isRightAlignedMessage(block);
  }

  function isOwnUserMessage(block: ChatMessageBlock): boolean {
    if (block.remoteImOrigin) return false;
    const speakerAgentId = String(block.speakerAgentId || "").trim();
    return !speakerAgentId || speakerAgentId === "user-persona";
  }

  function blockBelongsToMessageId(block: ChatMessageBlock, messageId: string): boolean {
    const normalizedMessageId = String(messageId || "").trim();
    if (!normalizedMessageId) return false;
    const sourceMessageId = String(block.sourceMessageId || "").trim();
    const blockId = String(block.id || "").trim();
    return sourceMessageId === normalizedMessageId || blockId === normalizedMessageId;
  }

  const latestOwnMessageId = computed(() => {
    for (let idx = messageBlocks.value.length - 1; idx >= 0; idx -= 1) {
      const block = messageBlocks.value[idx];
      if (block.isExtraTextBlock) continue;
      if (!isOwnUserMessage(block)) continue;
      const messageId = String(block.sourceMessageId || block.id || "").trim();
      if (messageId) return messageId;
    }
    return "";
  });

  const latestOwnElasticItemId = computed(() => {
    const targetMessageId = latestOwnMessageId.value;
    if (!targetMessageId) return "";
    for (let idx = chatRenderItems.value.length - 1; idx >= 0; idx -= 1) {
      const item = chatRenderItems.value[idx];
      if (item.kind === "message") {
        if (blockBelongsToMessageId(item.block, targetMessageId)) return item.id;
        continue;
      }
      if (item.kind === "group") {
        if (item.items.some((groupItem) => blockBelongsToMessageId(groupItem.block, targetMessageId))) return item.id;
      }
    }
    return "";
  });

  const blockChronologicalIndexMap = computed(() => {
    const map = new Map<string, number>();
    messageBlocks.value.forEach((block, index) => {
      const blockId = String(block.id || "").trim();
      if (!blockId || map.has(blockId)) return;
      map.set(blockId, index);
    });
    return map;
  });

  const renderItemById = computed(() => {
    const map = new Map<string, ChatRenderItem>();
    chatRenderItems.value.forEach((item) => map.set(item.id, item));
    return map;
  });

  return {
    isOwnMessage,
    latestOwnMessageId,
    latestOwnElasticItemId,
    blockChronologicalIndexMap,
    renderItemById,
  };
}
