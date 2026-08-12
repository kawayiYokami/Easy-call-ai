import type { ChatConversationOverviewItem, ConversationPreviewMessage } from "../../../types/app";

// ==================== 会话项展示判定（纯函数，供 ChatConversationItem 与 Sidebar 共用） ====================

export type ConversationItemLevel = "full" | "sim" | "mini";

/** 简单条目摘要的默认展开窗口：7 天内有更新的会话 */
export const SIMPLE_ITEM_RECENT_WINDOW_MS = 7 * 24 * 60 * 60 * 1000;

/** 未读或 7 天内有更新 → sim（摘要常显）；否则 → mini（摘要折叠、hover 展开） */
export function simpleConversationItemLevel(
  item: ChatConversationOverviewItem,
  now: number = Date.now(),
): "sim" | "mini" {
  return hasUnreadOrRecentActivity(item, now) ? "sim" : "mini";
}

export function hasUnreadOrRecentActivity(
  item: ChatConversationOverviewItem,
  now: number = Date.now(),
): boolean {
  if (Number(item.unreadCount || 0) > 0) return true;
  const raw = String(item.lastMessageAt || item.updatedAt || "").trim();
  if (!raw) return false;
  const time = Date.parse(raw);
  if (!Number.isFinite(time)) return false;
  return now - time <= SIMPLE_ITEM_RECENT_WINDOW_MS;
}

/** 未读角标：当前会话不显示；0 不显示；超过 99 显示 99+ */
export function conversationUnreadBadge(
  item: ChatConversationOverviewItem,
  activeConversationId: string,
): string {
  if (String(item.conversationId || "").trim() === String(activeConversationId || "").trim()) {
    return "";
  }
  const unreadCount = Math.max(0, Number(item.unreadCount || 0));
  if (unreadCount <= 0) return "";
  return unreadCount > 99 ? "99+" : String(unreadCount);
}

/** 运行时是否忙碌（流式/整理上下文/归档/压缩） */
export function conversationRuntimeBusy(runtimeState?: ChatConversationOverviewItem["runtimeState"]): boolean {
  return runtimeState === "assistant_streaming"
    || runtimeState === "organizing_context"
    || runtimeState === "archiving"
    || runtimeState === "compacting";
}

export type ConversationIndicatorTone = "error" | "info" | "success" | "";

/** 完整项状态指示点：当前会话不显示；pipeline error/busy/success 映射为 error/info/success */
export function conversationStatusIndicatorTone(
  pipelineStatus: string,
  isActiveConversation: boolean,
): ConversationIndicatorTone {
  if (isActiveConversation) return "";
  if (pipelineStatus === "error") return "error";
  if (pipelineStatus === "busy") return "info";
  if (pipelineStatus === "success") return "success";
  return "";
}

export function conversationIndicatorClass(tone: ConversationIndicatorTone): string {
  if (tone === "error") return "bg-error";
  if (tone === "info") return "bg-warning";
  if (tone === "success") return "bg-success";
  return "";
}

/** 简单项左侧指示条：未读优先 error；tool/system 消息 warning；用户消息按 speaker 区分；其余 success */
export function conversationSimpleIndicatorClass(
  item: ChatConversationOverviewItem,
  unreadBadge: string,
  previews: ConversationPreviewMessage[],
): string {
  if (unreadBadge) return "bg-error";
  const last = previews[previews.length - 1];
  if (!last) return "bg-success";
  const role = last.role || "";
  const speakerId = String(last.speakerAgentId || "").trim();
  if (role === "tool" || role === "system") return "bg-warning";
  if (role === "user") {
    // 系统提醒/压缩摘要等系统消息的 role 也是 user，须用 agentId 区分用户与系统
    if (!speakerId || speakerId === "user-persona") return "bg-info";
    return "bg-warning";
  }
  return "bg-success";
}

/** 摘要行显隐：sim 常显两行；mini 折叠一行、hover 展开 */
export function conversationSummaryClass(level: ConversationItemLevel): string {
  if (level === "sim") return "max-h-8 opacity-60";
  return "max-h-0 opacity-0 group-hover:max-h-8 group-hover:opacity-60";
}
