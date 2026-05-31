<template>
  <div class="flex h-[80vh] w-80 max-h-[calc(100vh-1rem)] max-w-[calc(100vw-1rem)] flex-col rounded-box border border-base-300 bg-base-100 shadow-xl">
    <ChatConversationListHeader
      v-model:search-query="conversationSearchQuery"
      v-model:active-tab="activeConversationTab"
      :search-placeholder="t('chat.conversationSearchPlaceholder')"
      :local-label="t('chat.localConversationTab')"
      :contact-label="t('chat.contactConversationTab')"
    />
    <ChatConversationFloatingScroll class="flex-1 min-h-0">
      <section
        v-for="section in filteredConversationSections"
        :key="section.key"
        class="last:mb-0"
      >
        <div class="divider m-0 h-auto min-h-0 py-1 text-[11px] font-semibold uppercase tracking-wide text-base-content/45 before:bg-base-300 after:bg-base-300">
          {{ section.title }}
        </div>
        <div>
          <div
            v-for="(item, itemIndex) in section.items"
            :key="item.conversationId"
            class="group relative"
          >
            <div
              class="block w-full rounded-none text-left transition-colors"
              :class="[
                item.conversationId === props.activeConversationId ? 'bg-base-300' : 'bg-base-100 hover:bg-base-200',
                isConversationItemDisabled(item) ? 'cursor-not-allowed opacity-60' : 'cursor-pointer',
              ]"
              :role="isCurrentConversation(item) || isConversationItemDisabled(item) ? undefined : 'button'"
              :tabindex="isCurrentConversation(item) || isConversationItemDisabled(item) ? undefined : 0"
              :title="conversationItemTitle(item)"
              @click="handleConversationCardClick(item)"
              @keydown.enter.prevent="handleConversationCardClick(item)"
              @keydown.space.prevent="handleConversationCardClick(item)"
            >
              <div class="flex items-center gap-2 p-2">
              <div class="shrink-0">
                <div class="indicator">
                  <span
                    v-if="conversationIndicatorTone(item)"
                    class="indicator-item indicator-top indicator-end z-10 h-2.5 w-2.5 translate-x-0.5 -translate-y-0.5 rounded-full"
                    :class="conversationIndicatorClass(conversationIndicatorTone(item))"
                    aria-hidden="true"
                  ></span>
                  <div class="avatar">
                    <div class="w-10 h-10 rounded-full bg-error text-error-content">
                      <img
                        v-if="lastSpeakerAvatarUrl(item)"
                        :src="lastSpeakerAvatarUrl(item)"
                        :alt="lastSpeakerLabel(item)"
                        class="w-10 h-10 rounded-full object-cover"
                      />
                      <span v-else class="flex h-10 w-10 items-center justify-center rounded-full text-sm font-bold">
                        {{ lastSpeakerInitial(item) }}
                      </span>
                    </div>
                  </div>
                </div>
              </div>

              <div class="flex-1 min-w-0">
                <div class="flex items-start justify-between gap-1.5">
                  <div class="flex min-w-0 items-center gap-1.5">
                    <input
                      v-if="isEditingTitle(item)"
                      :ref="setRenameInputRef"
                      v-model="editingTitleDraft"
                      type="text"
                      class="input input-bordered input-sm h-8 min-h-0 w-full max-w-full text-sm font-medium"
                      @click.stop
                      @mousedown.stop
                      @keydown.enter.prevent="commitConversationTitleEdit(item)"
                      @keydown.esc.prevent="cancelConversationTitleEdit()"
                      @blur="handleConversationTitleBlur(item)"
                    />
                    <div v-else class="min-w-0 truncate text-sm font-medium">
                      {{ conversationDisplayTitle(item) }}
                    </div>
                  </div>
                  <div class="flex shrink-0 items-center gap-1">
                    <span class="text-[11px] text-base-content/60">
                      {{ formatConversationTime(item.updatedAt) }}
                    </span>
                    <div
                      v-if="shouldShowConversationMenu(item) && !isEditingTitle(item)"
                      class="dropdown dropdown-end"
                      :class="conversationMenuPlacementClass(itemIndex, section.items.length)"
                    >
                      <button
                        type="button"
                        tabindex="0"
                        class="btn btn-ghost btn-xs h-6 min-h-6 w-6 min-w-6 p-0 text-base-content/55 opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 group-hover:pointer-events-auto group-focus-within:opacity-100 group-focus-within:pointer-events-auto hover:text-base-content"
                        :title="t('common.more')"
                        @click.stop
                        @mousedown.stop
                      >
                        <Ellipsis class="h-3.5 w-3.5" />
                      </button>
                      <ul
                        tabindex="0"
                        class="menu dropdown-content z-[60] mt-2 w-40 rounded-box border border-base-300 bg-base-100 p-1 shadow-xl"
                        @click.stop
                        @mousedown.stop
                      >
                        <li v-if="!item.isMainConversation">
                          <button
                            type="button"
                            :disabled="!canToggleConversationPin(item)"
                            @click.stop="toggleConversationPin(item)"
                          >
                            <PinOff v-if="item.isPinned" class="h-4 w-4" />
                            <Pin v-else class="h-4 w-4" />
                            <span>{{ pinConversationTitle(item) }}</span>
                          </button>
                        </li>
                        <li>
                          <button
                            type="button"
                            :disabled="!canRenameConversation(item)"
                            @click.stop="startConversationTitleEdit(item)"
                          >
                            <PencilLine class="h-4 w-4" />
                            <span>{{ t("common.rename") }}</span>
                          </button>
                        </li>
                        <li>
                          <button
                            type="button"
                            :disabled="!canExportConversation(item)"
                            @click.stop="requestConversationExport(item)"
                          >
                            <Download class="h-4 w-4" />
                            <span>{{ t("chat.exportConversation") }}</span>
                          </button>
                        </li>
                        <li v-if="!item.isMainConversation">
                          <button
                            type="button"
                            :disabled="!canArchiveConversation(item)"
                            @click.stop="requestConversationArchive(item)"
                          >
                            <Archive class="h-4 w-4" />
                            <span>{{ t("common.archive") }}</span>
                          </button>
                        </li>
                        <li v-if="!item.isMainConversation">
                          <button
                            type="button"
                            :disabled="!canDeleteConversation(item)"
                            class="text-error"
                            @click.stop="requestConversationDelete(item)"
                          >
                            <Trash2 class="h-4 w-4" />
                            <span>{{ t("common.delete") }}</span>
                          </button>
                        </li>
                      </ul>
                    </div>
                  </div>
                </div>

                <div class="mt-1 flex items-center justify-between gap-2 text-xs">
                  <span class="min-w-0 truncate opacity-60">
                    {{ conversationStatusText(item) || latestPreviewLine(item) }}
                  </span>
                  <div class="flex shrink-0 items-center gap-2">
                    <span v-if="conversationPipelineStatus(item) === 'busy' || conversationRuntimeBusy(item)" class="loading loading-spinner loading-xs text-primary" :title="conversationStatusText(item)"></span>
                    <span v-else-if="conversationPipelineStatus(item) === 'error'" class="badge badge-error badge-xs">{{ t("common.failed") }}</span>
                    <span v-else-if="conversationStatusText(item)" class="text-[11px] text-base-content/60">{{ conversationStatusText(item) }}</span>
                    <span
                      v-if="unreadCountBadge(item)"
                      class="inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-error px-1.5 text-[11px] font-medium text-error-content"
                    >
                      {{ unreadCountBadge(item) }}
                    </span>
                  </div>
                </div>
              </div>
            </div>
            </div>

          </div>
        </div>
      </section>
      <div
        v-if="filteredConversationSections.length === 0"
        class="px-3 py-4 text-center text-sm text-base-content/60"
      >
        {{ t("chat.conversationSearchEmpty") }}
      </div>
    </ChatConversationFloatingScroll>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watchEffect } from "vue";
import { useI18n } from "vue-i18n";
import { Archive, Download, Ellipsis, PencilLine, Pin, PinOff, Trash2 } from "@lucide/vue";
import type { ChatConversationOverviewItem, ConversationPreviewMessage } from "../../../types/app";
import { usePipelineStatus } from "../../shell/composables/use-pipeline-status";
import { formatConversationListTime } from "../utils/conversation-time";
import { resolveConversationDisplayTitle } from "../utils/conversation-title";
import ChatConversationFloatingScroll from "./ChatConversationFloatingScroll.vue";
import ChatConversationListHeader from "./ChatConversationListHeader.vue";

const CHAT_CONVERSATION_LIST_TAB_STORAGE_KEY = "easy_call.chat_conversation_list_tab.v1";

const props = defineProps<{
  items: ChatConversationOverviewItem[];
  activeConversationId: string;
  userAlias: string;
  personaNameMap: Record<string, string>;
  personaAvatarUrlMap: Record<string, string>;
  userAvatarUrl: string;
}>();

const emit = defineEmits<{
  (e: "selectConversation", payload: { conversationId: string; kind?: "local_unarchived" | "remote_im_contact"; remoteContactId?: string }): void;
  (e: "renameConversation", payload: { conversationId: string; title: string }): void;
  (e: "togglePinConversation", conversationId: string): void;
  (e: "archiveConversation", conversationId: string): void;
  (e: "exportConversation", conversationId: string): void;
  (e: "deleteConversation", conversationId: string): void;
}>();

const { t, locale } = useI18n();
const { conversationStatusById } = usePipelineStatus({
  activeConversationId: computed(() => String(props.activeConversationId || "").trim()),
});
const renameInputRef = ref<HTMLInputElement | null>(null);
const editingConversationId = ref("");
const editingTitleDraft = ref("");
const conversationSearchQuery = ref("");
const activeConversationTab = ref<"local" | "contact">(readStoredConversationTab());

function readStoredConversationTab(): "local" | "contact" {
  if (typeof window === "undefined") return "local";
  const stored = String(window.localStorage.getItem(CHAT_CONVERSATION_LIST_TAB_STORAGE_KEY) || "").trim();
  return stored === "contact" ? "contact" : "local";
}

watchEffect(() => {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(CHAT_CONVERSATION_LIST_TAB_STORAGE_KEY, activeConversationTab.value);
});

const conversationPreviewCache = computed(() => new Map(
  props.items.map((item) => [String(item.conversationId || "").trim(), Array.isArray(item.previewMessages) ? item.previewMessages : []]),
));
const conversationSections = computed(() => {
  const visibleItems = props.items.filter((item) => {
    const kind = String(item.kind || "local_unarchived").trim();
    return activeConversationTab.value === "contact"
      ? kind === "remote_im_contact"
      : kind !== "remote_im_contact";
  });
  const pinned = visibleItems.filter((item) => !!item.isPinned || !!item.isMainConversation);
  const others = visibleItems.filter((item) => !item.isPinned && !item.isMainConversation);
  return [
    pinned.length > 0
      ? {
        key: "pinned",
        title: t("chat.pinnedConversations"),
        items: pinned,
      }
      : null,
    others.length > 0
      ? {
        key: "others",
        title: t("chat.otherConversations"),
        items: others,
      }
      : null,
  ].filter(Boolean) as Array<{ key: string; title: string; items: ChatConversationOverviewItem[] }>;
});

const normalizedConversationSearchQuery = computed(() =>
  String(conversationSearchQuery.value || "").trim().toLocaleLowerCase(),
);

const filteredConversationSections = computed(() => {
  const query = normalizedConversationSearchQuery.value;
  if (!query) return conversationSections.value;
  return conversationSections.value
    .map((section) => ({
      ...section,
      items: section.items.filter((item) => conversationMatchesSearch(item, query)),
    }))
    .filter((section) => section.items.length > 0);
});

watchEffect(() => {
  const editingId = String(editingConversationId.value || "").trim();
  if (!editingId) return;
  const item = props.items.find((entry) => String(entry.conversationId || "").trim() === editingId);
  if (!item || !canRenameConversation(item)) {
    resetConversationTitleEdit();
  }
});

function resetConversationTitleEdit() {
  editingConversationId.value = "";
  editingTitleDraft.value = "";
}

function setRenameInputRef(element: Element | { $el?: Element | null } | null) {
  renameInputRef.value = element instanceof HTMLInputElement ? element : null;
}

function isConversationItemDisabled(item: ChatConversationOverviewItem): boolean {
  return item.runtimeState === "organizing_context"
    || item.runtimeState === "archiving"
    || item.runtimeState === "compacting"
    || !!item.detachedWindowOpen;
}

function isLocalConversation(item: ChatConversationOverviewItem): boolean {
  return item.kind !== "remote_im_contact";
}

function isCurrentConversation(item: ChatConversationOverviewItem): boolean {
  return String(item.conversationId || "").trim() === String(props.activeConversationId || "").trim();
}

function shouldShowConversationMenu(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item) && !isConversationItemDisabled(item);
}

function canRenameConversation(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item) && !isConversationItemDisabled(item);
}

function isEditingTitle(item: ChatConversationOverviewItem): boolean {
  return String(item.conversationId || "").trim() === String(editingConversationId.value || "").trim();
}

function conversationItemTitle(item: ChatConversationOverviewItem): string {
  if (item.kind === "remote_im_contact") {
    return String(item.remoteContactDisplayName || item.title || "").trim();
  }
  if (item.detachedWindowOpen) {
    return t("chat.detachedWindowOpen");
  }
  if (item.runtimeState === "archiving") {
    return runtimeStateText("archiving");
  }
  if (item.runtimeState === "compacting") {
    return runtimeStateText("compacting");
  }
  if (item.runtimeState === "organizing_context") {
    return t("chat.organizingContextDisabled");
  }
  return item.workspaceLabel || t("chat.defaultWorkspace");
}

function handleConversationCardClick(item: ChatConversationOverviewItem) {
  if (isCurrentConversation(item) || isConversationItemDisabled(item)) return;
  emit("selectConversation", {
    conversationId: String(item.conversationId || "").trim(),
    kind: item.kind,
    remoteContactId: String(item.remoteContactId || "").trim() || undefined,
  });
}

function canToggleConversationPin(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item) && !item.isMainConversation && !isConversationItemDisabled(item);
}

function canArchiveConversation(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item) && !item.isMainConversation && !isConversationItemDisabled(item);
}

function canExportConversation(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item) && !isConversationItemDisabled(item);
}

function canDeleteConversation(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item) && !item.isMainConversation && !isConversationItemDisabled(item);
}

function pinConversationTitle(item: ChatConversationOverviewItem): string {
  if (item.isMainConversation) return t("chat.mainConversationPinned");
  return item.isPinned ? t("chat.unpinConversation") : t("chat.pinConversation");
}

function toggleConversationPin(item: ChatConversationOverviewItem) {
  if (!canToggleConversationPin(item)) return;
  emit("togglePinConversation", String(item.conversationId || "").trim());
}

function requestConversationArchive(item: ChatConversationOverviewItem) {
  if (!canArchiveConversation(item)) return;
  emit("archiveConversation", String(item.conversationId || "").trim());
}

function requestConversationExport(item: ChatConversationOverviewItem) {
  if (!canExportConversation(item)) return;
  emit("exportConversation", String(item.conversationId || "").trim());
}

function requestConversationDelete(item: ChatConversationOverviewItem) {
  if (!canDeleteConversation(item)) return;
  emit("deleteConversation", String(item.conversationId || "").trim());
}

function conversationMenuPlacementClass(itemIndex: number, total: number): string {
  if (total <= 0) return "dropdown-bottom";
  return itemIndex < Math.ceil(total / 2) ? "dropdown-bottom" : "dropdown-top";
}

function conversationDisplayTitle(item: ChatConversationOverviewItem): string {
  return resolveConversationDisplayTitle(item, {
    locale: locale.value,
    untitledLabel: t("chat.untitledConversation"),
  });
}

async function startConversationTitleEdit(item: ChatConversationOverviewItem) {
  if (!canRenameConversation(item)) return;
  editingConversationId.value = String(item.conversationId || "").trim();
  editingTitleDraft.value = String(item.title || "").trim();
  await nextTick();
  renameInputRef.value?.focus();
  renameInputRef.value?.select();
}

function cancelConversationTitleEdit() {
  resetConversationTitleEdit();
}

function commitConversationTitleEdit(item: ChatConversationOverviewItem) {
  if (!isEditingTitle(item)) return;
  const conversationId = String(item.conversationId || "").trim();
  const currentTitle = String(item.title || "").trim();
  const nextTitle = String(editingTitleDraft.value || "").trim();
  if (!conversationId || nextTitle === currentTitle) {
    resetConversationTitleEdit();
    return;
  }
  resetConversationTitleEdit();
  emit("renameConversation", {
    conversationId,
    title: nextTitle,
  });
}

function handleConversationTitleBlur(item: ChatConversationOverviewItem) {
  commitConversationTitleEdit(item);
}

function unreadCountBadge(item: ChatConversationOverviewItem): string {
  if (String(item.conversationId || "").trim() === String(props.activeConversationId || "").trim()) {
    return "";
  }
  const unreadCount = Math.max(0, Number(item.unreadCount || 0));
  if (unreadCount <= 0) return "";
  return unreadCount > 99 ? "99+" : String(unreadCount);
}

function conversationPipelineStatus(item: ChatConversationOverviewItem) {
  return conversationStatusById.value[String(item.conversationId || "").trim()] || "";
}

function conversationRuntimeBusy(item: ChatConversationOverviewItem): boolean {
  return item.runtimeState === "assistant_streaming"
    || item.runtimeState === "organizing_context"
    || item.runtimeState === "archiving"
    || item.runtimeState === "compacting";
}

function conversationStatusText(item: ChatConversationOverviewItem): string {
  if (item.runtimeState && item.runtimeState !== "idle") return runtimeStateText(item.runtimeState);
  const pipelineStatus = conversationPipelineStatus(item);
  if (pipelineStatus === "busy") return t("chat.runtimeStreaming");
  if (pipelineStatus === "error") return t("common.failed");
  return "";
}

function conversationIndicatorTone(item: ChatConversationOverviewItem): "error" | "info" | "success" | "" {
  if (isCurrentConversation(item)) return "";
  const pipelineStatus = conversationPipelineStatus(item);
  if (pipelineStatus === "error") return "error";
  if (pipelineStatus === "busy") return "info";
  if (pipelineStatus === "success") return "success";
  return "";
}

function conversationIndicatorClass(tone: "error" | "info" | "success" | ""): string {
  if (tone === "error") return "bg-error";
  if (tone === "info") return "bg-warning";
  if (tone === "success") return "bg-success";
  return "";
}

function normalizedPreviewMessages(item: ChatConversationOverviewItem): ConversationPreviewMessage[] {
  return conversationPreviewCache.value.get(String(item.conversationId || "").trim()) || [];
}

function conversationMatchesSearch(item: ChatConversationOverviewItem, query: string): boolean {
  if (!query) return true;
  const title = conversationDisplayTitle(item).toLocaleLowerCase();
  if (title.includes(query)) return true;
  const previewTextBlock = normalizedPreviewMessages(item)
    .slice(-2)
    .map((preview) => previewText(preview).toLocaleLowerCase())
    .join("\n");
  return previewTextBlock.includes(query);
}

function runtimeStateText(runtimeState?: ChatConversationOverviewItem["runtimeState"]): string {
  if (runtimeState === "assistant_streaming") return t("chat.runtimeStreaming");
  if (runtimeState === "organizing_context") return t("chat.runtimeOrganizing");
  if (runtimeState === "archiving") return "归档中";
  if (runtimeState === "compacting") return "压缩中";
  return t("chat.runtimeIdle");
}

function speakerLabel(preview: ConversationPreviewMessage): string {
  if (preview.role === "tool") return t("archives.roleTool");
  const speakerId = String(preview.speakerAgentId || "").trim();
  if (!speakerId || speakerId === "user-persona") {
    return props.userAlias || t("archives.roleUser");
  }
  return props.personaNameMap?.[speakerId] || speakerId;
}

function previewText(preview: ConversationPreviewMessage): string {
  const text = String(preview.textPreview || "").trim();
  if (text) return text;
  if (preview.hasPdf) return t("chat.previewPdf");
  if (preview.hasImage) return t("chat.previewImage");
  if (preview.hasAudio) return t("chat.previewAudio");
  if (preview.hasAttachment) return t("chat.previewAttachment");
  return t("chat.conversationNoPreview");
}

function latestPreviewLine(item: ChatConversationOverviewItem): string {
  const previews = normalizedPreviewMessages(item);
  const latestPreview = previews[previews.length - 1];
  if (!latestPreview) return t("chat.conversationNoPreview");
  return `${speakerLabel(latestPreview)}: ${previewText(latestPreview)}`;
}

function formatConversationTime(value?: string): string {
  return formatConversationListTime(value, locale.value);
}

function lastSpeakerInitial(item: ChatConversationOverviewItem): string {
  const previews = normalizedPreviewMessages(item);
  if (previews.length === 0) return "?";

  const lastPreview = previews[previews.length - 1];
  const label = speakerLabel(lastPreview);
  return label.charAt(0).toUpperCase();
}

function lastSpeakerLabel(item: ChatConversationOverviewItem): string {
  const previews = normalizedPreviewMessages(item);
  if (previews.length === 0) return "";

  const lastPreview = previews[previews.length - 1];
  return speakerLabel(lastPreview);
}

function lastSpeakerAvatarUrl(item: ChatConversationOverviewItem): string {
  const previews = normalizedPreviewMessages(item);
  if (previews.length === 0) return "";

  const lastPreview = previews[previews.length - 1];
  const speakerId = String(lastPreview.speakerAgentId || "").trim();

  if (!speakerId || speakerId === "user-persona") {
    return props.userAvatarUrl || "";
  }

  return props.personaAvatarUrlMap?.[speakerId] || "";
}
</script>
