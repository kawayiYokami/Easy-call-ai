import type { ChatConversationOverviewItem } from "../../../types/app";
import { defaultWorkspaceNameFromPath } from "../../../utils/shell-workspaces";

export type ConversationSection = {
  key: string;
  title: string;
  items: ChatConversationOverviewItem[];
  workspaceRootPath?: string;
};

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

export function workspaceNameFromPath(path: string): string {
  return defaultWorkspaceNameFromPath(path);
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
    const key = `workspace:${normalizedPath || title}`;
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
