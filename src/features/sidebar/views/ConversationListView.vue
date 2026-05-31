<template>
  <div class="sidebar-scrollbar h-full overflow-y-auto bg-base-200">
    <button
      v-for="item in items"
      :key="item.conversationId"
      type="button"
      class="group flex w-full items-center gap-2 border-b border-base-300 p-2 text-left transition-colors"
      :class="itemClass(item)"
      :disabled="isConversationDisabled(item)"
      @click="handleSelect(item)"
    >
      <div class="shrink-0">
        <div class="indicator">
          <span
            v-if="indicatorTone(item)"
            class="indicator-item indicator-top indicator-end z-10 h-2.5 w-2.5 translate-x-0.5 -translate-y-0.5 rounded-full"
            :class="indicatorTone(item) === 'busy' ? 'bg-primary' : 'bg-error'"
            aria-hidden="true"
          ></span>
          <div class="avatar">
            <div class="flex h-10 w-10 items-center justify-center rounded-full bg-neutral text-neutral-content">
              <img
                v-if="lastSpeakerAvatarUrl(item)"
                :src="lastSpeakerAvatarUrl(item)"
                :alt="lastSpeakerLabel(item)"
                class="h-10 w-10 rounded-full object-cover"
              />
              <span v-else class="text-sm font-bold">{{ conversationInitial(item) }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="min-w-0 flex-1">
        <div class="flex items-start justify-between gap-1.5">
          <div class="min-w-0 truncate text-sm font-medium">{{ displayTitle(item) }}</div>
          <span class="shrink-0 text-[11px] text-base-content/60">{{ formatDate(item.lastMessageAt || item.updatedAt) }}</span>
        </div>
        <div class="mt-1 flex items-center justify-between gap-2 text-xs">
          <span class="min-w-0 truncate opacity-60">{{ previewLine(item) }}</span>
          <div class="flex shrink-0 items-center gap-2">
            <span v-if="runtimeStateText(item)" class="text-[11px] text-base-content/60">{{ runtimeStateText(item) }}</span>
            <span
              v-if="unreadCount(item)"
              class="inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-error px-1.5 text-[11px] font-medium text-error-content"
            >
              {{ unreadCount(item) }}
            </span>
          </div>
        </div>
      </div>
    </button>
    <div v-if="items.length === 0" class="px-3 py-6 text-center text-sm opacity-70">暂无会话</div>
  </div>
</template>

<script setup lang="ts">
type ConversationListItem = {
  conversationId: string;
  title: string;
  summaryTitle?: string;
  updatedAt: string;
  lastMessageAt?: string;
  departmentName?: string;
  runtimeState?: string;
  detachedWindowOpen?: boolean;
  detachedWindowLabel?: string;
  unreadCount?: number;
  previewMessages?: Array<{
    role: string;
    speakerAgentId?: string;
    textPreview?: string;
    hasImage?: boolean;
    hasPdf?: boolean;
    hasAudio?: boolean;
    hasAttachment?: boolean;
  }>;
};
type ConversationPreview = NonNullable<ConversationListItem["previewMessages"]>[number];

const props = defineProps<{
  items: ConversationListItem[];
  activeConversationId: string;
  persona: {
    userAlias?: string;
    userAvatarUrl?: string;
    personaNameMap?: Record<string, string>;
    personaAvatarUrlMap?: Record<string, string>;
  };
}>();

const emit = defineEmits<{
  select: [conversationId: string];
}>();

function formatDate(value: string) {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString(undefined, { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}

function displayTitle(item: ConversationListItem): string {
  return String(item.title || item.summaryTitle || "").trim() || "未命名会话";
}

function conversationInitial(item: ConversationListItem): string {
  const text = lastSpeakerLabel(item) || displayTitle(item) || item.departmentName || "?";
  return text.trim()[0]?.toUpperCase() || "?";
}

function lastPreviewMessage(item: ConversationListItem): ConversationPreview | null {
  const messages = Array.isArray(item.previewMessages) ? item.previewMessages : [];
  return [...messages].reverse().find((message) =>
    String(message.textPreview || "").trim()
    || message.hasImage
    || message.hasPdf
    || message.hasAudio
    || message.hasAttachment
  ) || null;
}

function lastSpeakerLabel(item: ConversationListItem): string {
  const message = lastPreviewMessage(item);
  const speakerId = String(message?.speakerAgentId || "").trim();
  if (!speakerId || speakerId === "user-persona" || message?.role === "user") {
    return String(props.persona.userAlias || "我").trim() || "我";
  }
  return String(props.persona.personaNameMap?.[speakerId] || speakerId).trim();
}

function lastSpeakerAvatarUrl(item: ConversationListItem): string {
  const message = lastPreviewMessage(item);
  const speakerId = String(message?.speakerAgentId || "").trim();
  if (!speakerId || speakerId === "user-persona" || message?.role === "user") {
    return String(props.persona.userAvatarUrl || "").trim();
  }
  return String(props.persona.personaAvatarUrlMap?.[speakerId] || "").trim();
}

function previewLine(item: ConversationListItem): string {
  const latest = lastPreviewMessage(item);
  if (latest) {
    const text = String(latest.textPreview || "").trim();
    if (text) return text;
    if (latest.hasPdf) return "[PDF]";
    if (latest.hasImage) return "[图片]";
    if (latest.hasAudio) return "[音频]";
    if (latest.hasAttachment) return "[附件]";
  }
  return String(item.departmentName || "").trim() || "暂无消息";
}

function unreadCount(item: ConversationListItem): string {
  const count = Number(item.unreadCount || 0);
  if (!Number.isFinite(count) || count <= 0) return "";
  return count > 99 ? "99+" : String(count);
}

function indicatorTone(item: ConversationListItem): "busy" | "error" | "" {
  if (item.conversationId === props.activeConversationId) return "";
  const state = String(item.runtimeState || "").trim();
  if (state === "assistant_streaming" || state === "organizing_context") return "busy";
  return "";
}

function isConversationDisabled(item: ConversationListItem): boolean {
  if (item.conversationId === props.activeConversationId) return false;
  return String(item.runtimeState || "").trim() === "organizing_context" || !!item.detachedWindowOpen;
}

function itemClass(item: ConversationListItem): string {
  if (item.conversationId === props.activeConversationId) return "bg-base-300 hover:bg-base-300";
  if (isConversationDisabled(item)) return "cursor-not-allowed bg-base-100 opacity-60";
  return "bg-base-100 hover:bg-base-200";
}

function runtimeStateText(item: ConversationListItem): string {
  if (item.conversationId === props.activeConversationId) return "";
  if (item.detachedWindowOpen) return "已打开";
  const state = String(item.runtimeState || "").trim();
  if (state === "assistant_streaming") return "回复中";
  if (state === "organizing_context") return "整理中";
  return "";
}

function handleSelect(item: ConversationListItem) {
  if (isConversationDisabled(item)) return;
  emit("select", item.conversationId);
}
</script>
