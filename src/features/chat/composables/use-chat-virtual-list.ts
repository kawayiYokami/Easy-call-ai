import { computed, type Ref } from "vue";
import type { ChatMessageBlock } from "../../../types/app";
import {
  type ChatRenderItem,
  isCompactionBlock,
  isRightAlignedMessage,
  isCompactUserContinuation,
  estimateMessageBlockHeight,
  blockSizeDependencies,
} from "../utils/chat-render";
import { stableRenderIdFromBlock } from "../utils/stable-render-id";

const TIME_DIVIDER_GAP_MS = 15 * 60 * 1000;

interface UseChatVirtualListOptions {
  messageBlocks: Ref<ChatMessageBlock[]>;
  markdownIsDark: Ref<boolean>;
  playingAudioId: Ref<string>;
  userAlias: Ref<string>;
  userAvatarUrl: Ref<string>;
  personaNameMap: Ref<Record<string, string>>;
  personaAvatarUrlMap: Ref<Record<string, string>>;
  chatting: Ref<boolean>;
  conversationBusy: Ref<boolean>;
  frozen: Ref<boolean>;
  messageSelectionModeEnabled: Ref<boolean>;
  selectedMessageRenderIdSet: Ref<Set<string>>;
  isBubbleBackgroundHidden: (block: ChatMessageBlock) => boolean;
  canToggleBubbleBackground: (block: ChatMessageBlock) => boolean;
  canRegenerateBlock: (block: ChatMessageBlock, blockIndex: number) => boolean;
  canConfirmPlan: (block: ChatMessageBlock) => boolean;
}

// ==================== blockRenderId ====================

// 注意：ephemeral map 由调用方（composable 实例）持有，避免多实例共享
export function blockRenderId(block: ChatMessageBlock, ephemeralMap?: WeakMap<ChatMessageBlock, string>, ephemeralSeqRef?: { value: number }): string {
  const stableRenderId = stableRenderIdFromBlock(block);
  if (stableRenderId) return block.isExtraTextBlock ? `${stableRenderId}::extra` : stableRenderId;
  const rawId = String(block.id || "").trim();
  if (rawId) return rawId;
  const sourceMessageId = String(block.sourceMessageId || "").trim();
  if (sourceMessageId) {
    return block.isExtraTextBlock ? `${sourceMessageId}::extra` : sourceMessageId;
  }
  const createdAt = String(block.createdAt || "").trim();
  const speakerAgentId = String(block.speakerAgentId || "").trim();
  const role = String(block.role || "").trim();
  const textPreview = String(block.text || "").trim().slice(0, 64);
  if (createdAt || speakerAgentId || role || textPreview) {
    return ["block-stable", role || "no-role", speakerAgentId || "no-speaker", createdAt || "no-time", block.isExtraTextBlock ? "extra" : "base", textPreview || "no-text"].join(":");
  }
  if (!ephemeralMap || !ephemeralSeqRef) return `block-ephemeral-unknown`;
  const cached = ephemeralMap.get(block);
  if (cached) return cached;
  ephemeralSeqRef.value += 1;
  const nextId = `block-ephemeral-${ephemeralSeqRef.value}`;
  ephemeralMap.set(block, nextId);
  return nextId;
}

// ==================== 高度估算 ====================

export function estimateChatRenderItemHeight(item: ChatRenderItem): number {
  if (item.kind === "compaction" || item.kind === "plan_started" || item.kind === "time_divider") return 44;
  return estimateMessageBlockHeight(item.block, isRightAlignedMessage(item.block)) + 8;
}

export function virtualItemSizeDependencies(item: ChatRenderItem): unknown[] {
  if (item.kind === "compaction" || item.kind === "plan_started" || item.kind === "time_divider") return [item.id, item.kind];
  return [item.id, ...blockSizeDependencies(item.block)];
}

// ==================== composable ====================

export function useChatVirtualList(options: UseChatVirtualListOptions) {
  const {
    messageBlocks, markdownIsDark, playingAudioId, userAlias, userAvatarUrl,
    personaNameMap, personaAvatarUrlMap, chatting, conversationBusy, frozen,
    messageSelectionModeEnabled, selectedMessageRenderIdSet,
    isBubbleBackgroundHidden, canToggleBubbleBackground, canRegenerateBlock, canConfirmPlan,
  } = options;

  // 实例级 ephemeral map，避免多 ChatView 实例共享
  const ephemeralMap = new WeakMap<ChatMessageBlock, string>();
  const ephemeralSeq = { value: 0 };
  const rid = (block: ChatMessageBlock) => blockRenderId(block, ephemeralMap, ephemeralSeq);

  const chatRenderItems = computed<ChatRenderItem[]>(() => {
    const items: ChatRenderItem[] = [];
    let previousMessageBlock: ChatMessageBlock | null = null;
    let previousMessageTimeMs = 0;

    const blockTimeMs = (block: ChatMessageBlock): number => {
      const raw = String(block.createdAt || "").trim();
      if (!raw) return 0;
      const timestamp = new Date(raw).getTime();
      return Number.isFinite(timestamp) ? timestamp : 0;
    };

    const maybePushTimeDivider = (block: ChatMessageBlock, renderId: string) => {
      const currentTimeMs = blockTimeMs(block);
      if (previousMessageTimeMs > 0 && currentTimeMs > 0 && currentTimeMs - previousMessageTimeMs >= TIME_DIVIDER_GAP_MS) {
        const stableRenderId = stableRenderIdFromBlock(block);
        const dividerId = stableRenderId
          ? `time-divider-${renderId}`
          : `time-divider-${renderId}-${String(block.createdAt || "").trim()}`;
        previousMessageBlock = null;
        items.push({
          kind: "time_divider",
          id: dividerId,
          createdAt: String(block.createdAt || "").trim(),
        });
      }
      if (currentTimeMs > 0) {
        previousMessageTimeMs = currentTimeMs;
      }
    };

    messageBlocks.value.forEach((block, blockIndex) => {
      const renderId = rid(block);
      if (block.dividerKind === "plan_started") {
        previousMessageBlock = null;
        previousMessageTimeMs = 0;
        items.push({ kind: "plan_started", id: `plan-started-${renderId}`, renderId, block, blockIndex });
        return;
      }
      if (isCompactionBlock(block)) {
        previousMessageBlock = null;
        previousMessageTimeMs = 0;
        items.push({ kind: "compaction", id: `compaction-${renderId}`, renderId, block, blockIndex });
        return;
      }
      maybePushTimeDivider(block, renderId);
      const compactWithPrevious = isCompactUserContinuation(block, previousMessageBlock);
      items.push({ kind: "message", id: `message-${renderId}`, renderId, block, blockIndex, compactWithPrevious });
      previousMessageBlock = block;
    });
    return items;
  });

  function messageMemoKey(block: ChatMessageBlock, renderId: string, blockIndex: number, compactWithPrevious = false) {
    const selected = selectedMessageRenderIdSet.value.has(renderId);
    const canRegenerate = canRegenerateBlock(block, blockIndex);
    const canConfirm = canConfirmPlan(block);
    const requiresInteractionState = canRegenerate || canConfirm;
    return [
      block, markdownIsDark.value, playingAudioId.value,
      ...blockSizeDependencies(block),
      userAlias.value, userAvatarUrl.value, personaNameMap.value, personaAvatarUrlMap.value,
      conversationBusy.value, messageSelectionModeEnabled.value, selected,
      canRegenerate, canConfirm,
      isBubbleBackgroundHidden(block), canToggleBubbleBackground(block),
      compactWithPrevious,
      requiresInteractionState ? chatting.value : false,
      requiresInteractionState ? conversationBusy.value : false,
      requiresInteractionState ? frozen.value : false,
    ];
  }

  return { chatRenderItems, messageMemoKey };
}
