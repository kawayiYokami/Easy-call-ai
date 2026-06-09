import type { ChatConversationOverviewItem } from "../../../types/app";

type ConversationTitleLike = Pick<
  ChatConversationOverviewItem,
  "kind" | "title" | "summaryTitle" | "remoteContactDisplayName" | "updatedAt" | "lastMessageAt" | "isSystemNotificationConversation"
>;

type ResolveConversationDisplayTitleOptions = {
  locale?: string;
  untitledLabel: string;
};

function normalizedTitlePart(value?: string): string {
  return String(value || "").trim();
}

export const SYSTEM_NOTIFICATION_DISPLAY_TITLE = "P-ai系统";

export function formatConversationFallbackTitle(value?: string, locale?: string): string {
  const rawValue = normalizedTitlePart(value);
  if (!rawValue) return "";
  const date = new Date(rawValue);
  if (Number.isNaN(date.getTime())) return rawValue;
  const now = new Date();
  const sameYear = date.getFullYear() === now.getFullYear();
  const dateText = date.toLocaleDateString(locale || undefined, sameYear
    ? {
      month: "2-digit",
      day: "2-digit",
    }
    : {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
    });
  const timeText = date.toLocaleTimeString(locale || undefined, {
    hour: "2-digit",
    minute: "2-digit",
  });
  return `${dateText} ${timeText}`.trim();
}

export function resolveConversationDisplayTitle(
  item: ConversationTitleLike,
  options: ResolveConversationDisplayTitleOptions,
): string {
  if (item.isSystemNotificationConversation) {
    return SYSTEM_NOTIFICATION_DISPLAY_TITLE;
  }
  if (item.kind === "remote_im_contact") {
    return normalizedTitlePart(item.remoteContactDisplayName)
      || normalizedTitlePart(item.title)
      || options.untitledLabel;
  }
  return normalizedTitlePart(item.title)
    || normalizedTitlePart(item.summaryTitle)
    || formatConversationFallbackTitle(item.lastMessageAt || item.updatedAt, options.locale)
    || options.untitledLabel;
}
