import type { ChatConversationOverviewItem } from "../../../types/app";
import { defaultWorkspaceNameFromPath } from "../../../utils/shell-workspaces";

export type ConversationSection = {
  key: string;
  title: string;
  items: ChatConversationOverviewItem[];
  workspaceRootPath?: string;
};

export type ConversationSectionOrderState = {
  local: string[];
  contact: string[];
};

export type ConversationSidebarTab = "local" | "contact" | "task";

export type ConversationSectionTitles = {
  recent: string;
  pinned: string;
  other: string;
  defaultWorkspace: string;
};

export function buildConversationSections(
  items: ChatConversationOverviewItem[],
  options: {
    tab: ConversationSidebarTab;
    titles: ConversationSectionTitles;
    locale?: string | string[];
  },
): ConversationSection[] {
  const { tab, titles, locale } = options;
  const visibleItems = items.filter((item) => {
    const kind = String(item.kind || "local_unarchived").trim();
    return tab === "contact"
      ? kind === "remote_im_contact"
      : kind !== "remote_im_contact";
  });
  const pinned = visibleItems.filter((item) => !!item.isPinned || !!item.isSystemNotificationConversation);
  const others = visibleItems.filter((item) => !item.isPinned && !item.isSystemNotificationConversation);
  const recentSection = buildRecentConversationSection(visibleItems, titles.recent);
  const sections: ConversationSection[] = [];
  if (pinned.length > 0) {
    sections.push({
      key: "pinned",
      title: titles.pinned,
      items: pinned,
    });
  }
  if (recentSection) {
    sections.push(recentSection);
  }
  if (tab === "contact") {
    return [
      ...sections,
      ...buildRemoteConversationSections(others, {
        fallbackTitle: titles.other,
        locale,
      }),
    ];
  }
  return [
    ...sections,
    ...buildWorkspaceConversationSections(others, {
      defaultWorkspaceTitle: titles.defaultWorkspace,
      locale,
    }),
  ];
}

export const RECENT_CONVERSATION_SECTION_KEY = "recent";
const RECENT_CONVERSATION_LIMIT = 5;
const RECENT_TIME_EXTRA_WINDOW_MS = 60 * 60 * 1000;

type BuildWorkspaceConversationSectionsOptions = {
  defaultWorkspaceTitle: string;
  locale?: string | string[];
};

type BuildRemoteConversationSectionsOptions = {
  fallbackTitle: string;
  locale?: string | string[];
};

function normalizeWorkspaceSectionPath(path: string): string {
  return String(path || "").trim().replace(/\\/g, "/").replace(/\/+$/, "").toLocaleLowerCase();
}

function compareWorkspaceSectionText(left: string, right: string, locale?: string | string[]): number {
  return left.localeCompare(right, locale, {
    numeric: true,
    sensitivity: "base",
  });
}

function conversationRecencyMs(item: ChatConversationOverviewItem): number {
  const raw = String(item.lastMessageAt || item.updatedAt || "").trim();
  if (!raw) return 0;
  const time = Date.parse(raw);
  return Number.isFinite(time) ? time : 0;
}

function shouldIncludeRecentTimeExtra(item: ChatConversationOverviewItem, nowMs: number): boolean {
  const recencyMs = conversationRecencyMs(item);
  return recencyMs > 0 && nowMs - recencyMs <= RECENT_TIME_EXTRA_WINDOW_MS;
}

function shouldIncludeRecentUnreadExtra(item: ChatConversationOverviewItem): boolean {
  return Number(item.unreadCount || 0) > 0;
}

export function workspaceNameFromPath(path: string): string {
  return defaultWorkspaceNameFromPath(path);
}

export function applyConversationSectionOrder(
  sections: ConversationSection[],
  savedOrder: string[],
): { sections: ConversationSection[]; nextOrder: string[]; changed: boolean } {
  const normalizedSavedOrder = Array.isArray(savedOrder)
    ? savedOrder.map((item) => String(item || "").trim()).filter(Boolean)
    : [];
  const sectionByKey = new Map(sections.map((section) => [section.key, section] as const));
  const orderedSections: ConversationSection[] = [];
  const nextOrder: string[] = [];

  for (const key of normalizedSavedOrder) {
    const section = sectionByKey.get(key);
    if (!section) continue;
    orderedSections.push(section);
    nextOrder.push(key);
    sectionByKey.delete(key);
  }

  for (const section of sections) {
    if (!sectionByKey.has(section.key)) continue;
    orderedSections.push(section);
    nextOrder.push(section.key);
    sectionByKey.delete(section.key);
  }

  const changed = nextOrder.length !== normalizedSavedOrder.length
    || nextOrder.some((key, index) => key !== normalizedSavedOrder[index]);

  return {
    sections: orderedSections,
    nextOrder,
    changed,
  };
}

function resolveWorkspaceSectionTitle(
  currentTitle: string,
  nextTitle: string,
  workspaceRootPath: string,
): string {
  const current = String(currentTitle || "").trim();
  const next = String(nextTitle || "").trim();
  if (!current) return next;
  if (!next) return current;
  if (current === next) return current;

  const fallback = workspaceNameFromPath(workspaceRootPath);
  const currentIsFallback = !!fallback && current.localeCompare(fallback, undefined, { sensitivity: "accent" }) === 0;
  const nextIsFallback = !!fallback && next.localeCompare(fallback, undefined, { sensitivity: "accent" }) === 0;

  if (currentIsFallback && !nextIsFallback) return next;
  if (nextIsFallback && !currentIsFallback) return current;

  return current.length >= next.length ? current : next;
}

function resolveRemoteConversationSectionMeta(
  item: ChatConversationOverviewItem,
  fallbackTitle: string,
): { channelId: string; channelName: string; hasChannel: boolean; title: string; key: string } {
  const channelId = String(item.channelId || "").trim();
  let channelName = String(item.channelName || "").trim();
  if (!channelName) {
    const departmentName = String(item.departmentName || "").trim();
    const separatorIndex = departmentName.indexOf(" · ");
    if (separatorIndex > 0) {
      channelName = departmentName.slice(0, separatorIndex).trim();
    }
  }
  const hasChannel = !!(channelId || channelName);
  const title = channelName || channelId || fallbackTitle;
  return {
    channelId,
    channelName,
    hasChannel,
    title,
    key: `channel:${channelId || channelName || "__fallback__"}`,
  };
}

export function buildWorkspaceConversationSections(
  items: ChatConversationOverviewItem[],
  options: BuildWorkspaceConversationSectionsOptions,
): ConversationSection[] {
  const sections: ConversationSection[] = [];
  const byWorkspace = new Map<string, ConversationSection>();
  for (const item of items) {
    const path = String(item.workspaceRootPath || "").trim();
    const normalizedPath = normalizeWorkspaceSectionPath(path);
    const title = String(item.workspaceLabel || "").trim()
      || workspaceNameFromPath(path)
      || options.defaultWorkspaceTitle;
    const key = `workspace:${normalizedPath || "__default__"}`;
    const existing = byWorkspace.get(key);
    if (existing) {
      existing.title = resolveWorkspaceSectionTitle(existing.title, title, path || existing.workspaceRootPath || "");
      existing.items.push(item);
      continue;
    }
    const section = {
      key,
      title,
      workspaceRootPath: path || undefined,
      items: [item],
    };
    byWorkspace.set(key, section);
    sections.push(section);
  }
  return sections.sort((left, right) => {
    const leftPath = normalizeWorkspaceSectionPath(left.workspaceRootPath || "");
    const rightPath = normalizeWorkspaceSectionPath(right.workspaceRootPath || "");
    if (!!leftPath !== !!rightPath) {
      return leftPath ? -1 : 1;
    }
    return compareWorkspaceSectionText(leftPath || left.title, rightPath || right.title, options.locale)
      || compareWorkspaceSectionText(left.title, right.title, options.locale)
      || compareWorkspaceSectionText(left.key, right.key, options.locale);
  });
}

export function buildRemoteConversationSections(
  items: ChatConversationOverviewItem[],
  options: BuildRemoteConversationSectionsOptions,
): ConversationSection[] {
  const byChannel = new Map<string, {
    section: ConversationSection;
    hasChannel: boolean;
    sortTitle: string;
    sortKey: string;
  }>();
  for (const item of items) {
    const { channelId, channelName, hasChannel, title, key } = resolveRemoteConversationSectionMeta(
      item,
      options.fallbackTitle,
    );
    const existing = byChannel.get(key);
    if (existing) {
      existing.section.items.push(item);
      continue;
    }
    byChannel.set(key, {
      section: {
        key,
        title,
        items: [item],
      },
      hasChannel,
      sortTitle: title,
      sortKey: channelId || channelName || title,
    });
  }
  return Array.from(byChannel.values())
    .sort((left, right) => {
      if (left.hasChannel !== right.hasChannel) {
        return left.hasChannel ? -1 : 1;
      }
      return compareWorkspaceSectionText(left.sortTitle, right.sortTitle, options.locale)
        || compareWorkspaceSectionText(left.sortKey, right.sortKey, options.locale);
    })
    .map((entry) => entry.section);
}

export function buildRecentConversationSection(
  items: ChatConversationOverviewItem[],
  title: string,
): ConversationSection | null {
  const sortedItems = [...items]
    .sort((left, right) => conversationRecencyMs(right) - conversationRecencyMs(left));
  const recentItemsById = new Map<string, ChatConversationOverviewItem>();
  const addItem = (item: ChatConversationOverviewItem) => {
    const id = String(item.conversationId || "").trim();
    if (id) {
      recentItemsById.set(id, item);
    }
  };

  sortedItems.slice(0, RECENT_CONVERSATION_LIMIT).forEach(addItem);
  const nowMs = Date.now();
  sortedItems
    .filter((item) => shouldIncludeRecentTimeExtra(item, nowMs) || shouldIncludeRecentUnreadExtra(item))
    .forEach(addItem);

  const recentItems = Array.from(recentItemsById.values())
    .sort((left, right) => conversationRecencyMs(right) - conversationRecencyMs(left));
  if (recentItems.length === 0) return null;
  return {
    key: RECENT_CONVERSATION_SECTION_KEY,
    title,
    items: recentItems,
  };
}
