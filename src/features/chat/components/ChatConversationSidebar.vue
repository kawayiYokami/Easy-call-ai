<template>
  <aside class="conversation-time-container flex h-full w-full shrink-0 flex-col border-r border-base-300 bg-base-200">
    <div class="flex items-center gap-2 p-2 pb-0">
      <div role="tablist" class="tabs tabs-border min-w-0 shrink-0">
        <button
          type="button"
          role="tab"
          class="tab h-8 px-3 transition-[color,border-color,background-color] duration-200 ease-out"
          :class="activeConversationTab === 'local' ? 'tab-active font-semibold' : ''"
          @click="requestConversationTabChange('local')"
        >
          {{ t('chat.localConversationTab') }}
        </button>
        <button
          type="button"
          role="tab"
          class="tab h-8 px-3 transition-[color,border-color,background-color] duration-200 ease-out"
          :class="activeConversationTab === 'contact' ? 'tab-active font-semibold' : ''"
          @click="requestConversationTabChange('contact')"
        >
          {{ t('chat.contactConversationTab') }}
        </button>
        <button
          type="button"
          role="tab"
          class="tab h-8 px-3 transition-[color,border-color,background-color] duration-200 ease-out"
          :class="activeConversationTab === 'task' ? 'tab-active font-semibold' : ''"
          @click="requestConversationTabChange('task')"
        >
          {{ t('chat.taskConversationTab') }}
        </button>
      </div>
      <button
        type="button"
        class="btn btn-ghost btn-xs h-7 min-h-7 w-7 min-w-7 p-0 ml-auto"
        :class="showSearch ? 'text-primary' : 'text-base-content/55'"
        :title="searchPlaceholder"
        @click="showSearch = !showSearch"
      >
        <Search class="h-4 w-4" />
      </button>
    </div>
    <div v-if="showSearch" class="shrink-0 px-2 pt-1 pb-1">
      <label class="input input-bordered input-sm flex h-8 min-w-0 items-center gap-2 bg-base-100">
        <Search class="h-3.5 w-3.5 opacity-60" />
        <input
          ref="searchInputRef"
          v-model="conversationSearchQuery"
          type="text"
          class="w-full bg-transparent outline-none"
          :placeholder="searchPlaceholder"
        />
      </label>
    </div>
    <ChatConversationFloatingScroll ref="conversationFloatingScrollRef" class="flex-1 min-h-0">
      <Transition :name="conversationTabTransitionName" mode="out-in" @after-enter="handleConversationTabTransitionSettled">
        <div :key="activeConversationTab" class="conversation-tab-panel">
          <ChatTaskSidebarPanel
            v-if="activeConversationTab === 'task'"
            :conversation-items="items"
            :search-query="conversationSearchQuery"
            @edit-task="requestTaskEdit"
            @layout-change="scheduleConversationListScrollbarUpdate"
          />
          <template v-else>
            <CollapsibleGroup
              v-for="section in filteredConversationSections"
              :key="section.key"
              :title="section.title"
              :count="section.items.length"
              :model-value="isConversationSectionCollapsed(section.key)"
              @update:model-value="toggleConversationSection(section.key)"
              @after-enter="scheduleConversationListScrollbarUpdate"
              @after-leave="scheduleConversationListScrollbarUpdate"
            >
            <template #actions>
              <button
                v-if="section.workspaceRootPath"
                type="button"
                class="btn btn-ghost btn-xs ml-auto h-6 min-h-6 w-6 min-w-6 shrink-0 p-0 text-base-content opacity-0 transition-opacity group-hover/section:opacity-100"
                :title="t('chat.newConversation')"
                @click.stop="createConversationInSection(section)"
              >
                <SquarePen class="h-3.5 w-3.5" />
              </button>
            </template>
            <div
              v-for="(item, itemIndex) in section.items"
              :key="item.conversationId"
              class="group relative mx-1"
            >
                  <div
                    class="block rounded-lg px-2 text-left transition-colors hover:bg-base-100/70"
                    :class="[
                      item.conversationId === activeConversationId ? 'bg-base-300 hover:bg-base-300' : 'bg-transparent',
                      isConversationDisabled(item) ? 'cursor-not-allowed opacity-60' : 'cursor-pointer',
                    ]"
                    :role="isCurrentConversation(item) || isConversationDisabled(item) ? undefined : 'button'"
                    :tabindex="isCurrentConversation(item) || isConversationDisabled(item) ? undefined : 0"
                    :title="conversationItemTitle(item)"
                    @click="handleConversationCardClick(item)"
                    @keydown.enter.prevent="handleConversationCardClick(item)"
                    @keydown.space.prevent="handleConversationCardClick(item)"
                  >
                    <div class="flex items-center gap-2 py-2">
                    <div class="shrink-0">
                      <div class="indicator">
                        <span
                          v-if="conversationIndicatorTone(item)"
                          class="indicator-item indicator-top indicator-end z-10 h-2.5 w-2.5 translate-x-0.5 -translate-y-0.5 rounded-full"
                          :class="conversationIndicatorClass(conversationIndicatorTone(item))"
                          aria-hidden="true"
                        ></span>
                        <div class="avatar">
                          <div class="flex h-10 w-10 items-center justify-center rounded-full bg-neutral text-neutral-content">
                            <img
                              v-if="sideListLastSpeakerAvatarUrl(item)"
                              :src="sideListLastSpeakerAvatarUrl(item)"
                              :alt="sideListLastSpeakerLabel(item)"
                              class="w-10 h-10 rounded-full object-cover"
                            />
                            <span v-else class="text-sm font-bold">{{ sideListLastSpeakerInitial(item) }}</span>
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
                          <button
                            v-else-if="canRenameConversation(item)"
                            type="button"
                            class="min-w-0 truncate rounded px-0.5 text-left text-sm font-medium hover:bg-base-300/70"
                            @click.stop="startConversationTitleEdit(item)"
                          >
                            {{ conversationDisplayTitle(item) }}
                          </button>
                          <div v-else class="min-w-0 truncate text-sm font-medium">
                            {{ conversationDisplayTitle(item) }}
                          </div>
                        </div>
                        <div class="flex shrink-0 items-center gap-1">
                          <span class="conversation-time-label text-[11px] text-base-content/60">
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
                              class="menu dropdown-content z-60 mt-2 w-40 rounded-box border border-base-300 bg-base-100 p-1 shadow-xl"
                              @click.stop
                              @mousedown.stop
                            >
                              <li v-if="!item.isSystemNotificationConversation">
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
                              <li v-if="!item.isSystemNotificationConversation">
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
                                  <Upload class="h-4 w-4" />
                                  <span>{{ t("chat.exportConversation") }}</span>
                                </button>
                              </li>
                              <li v-if="!item.isSystemNotificationConversation">
                                <button
                                  type="button"
                                  :disabled="!canArchiveConversation(item)"
                                  class="text-error"
                                  @click.stop="requestConversationArchive(item)"
                                >
                                  <Trash2 class="h-4 w-4" />
                                  <span>{{ t("common.archive") }}</span>
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
            </CollapsibleGroup>
            <div
              v-if="filteredConversationSections.length === 0"
              class="px-3 py-4 text-center text-sm text-base-content/60"
            >
              {{ t("chat.conversationSearchEmpty") }}
            </div>
          </template>
        </div>
      </Transition>
    </ChatConversationFloatingScroll>
  </aside>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch, watchEffect } from "vue";
import { useI18n } from "vue-i18n";
import { Ellipsis, PencilLine, Pin, PinOff, Search, SquarePen, Trash2, Upload } from "@lucide/vue";
import CollapsibleGroup from "./CollapsibleGroup.vue";
import type { ChatConversationOverviewItem, ConversationPreviewMessage } from "../../../types/app";
import type { TaskEntry } from "../../config/views/config-tabs/task-editor";
import { usePipelineStatus } from "../../shell/composables/use-pipeline-status";
import { formatConversationListTime } from "../utils/conversation-time";
import {
  buildRemoteConversationSections,
  buildWorkspaceConversationSections,
  type ConversationSection,
} from "../utils/conversation-sections";
import { resolveConversationDisplayTitle } from "../utils/conversation-title";
import ChatConversationFloatingScroll from "./ChatConversationFloatingScroll.vue";
import ChatTaskSidebarPanel from "./ChatTaskSidebarPanel.vue";

type ConversationSidebarTab = "local" | "contact" | "task";


const props = defineProps<{
  items: ChatConversationOverviewItem[];
  activeConversationId: string;
  userAlias: string;
  userAvatarUrl: string;
  personaNameMap: Record<string, string>;
  personaAvatarUrlMap: Record<string, string>;
  activeTab: ConversationSidebarTab;
}>();

const emit = defineEmits<{
  (e: "select", payload: { conversationId: string; kind?: "local_unarchived" | "remote_im_contact"; remoteContactId?: string }): void;
  (e: "rename", payload: { conversationId: string; title: string }): void;
  (e: "togglePinConversation", conversationId: string): void;
  (e: "archiveConversation", conversationId: string): void;
  (e: "exportConversation", conversationId: string): void;
  (e: "deleteConversation", conversationId: string): void;
  (e: "update:activeTab", value: ConversationSidebarTab): void;
  (e: "editTask", task: TaskEntry): void;
}>();

const { t, locale } = useI18n();
const SYSTEM_PERSONA_ID = "system-persona";
const renameInputRef = ref<HTMLInputElement | null>(null);
const editingConversationId = ref("");
const editingTitleDraft = ref("");
const conversationSearchQuery = ref("");
const showSearch = ref(false);
const searchInputRef = ref<HTMLInputElement | null>(null);
const conversationFloatingScrollRef = ref<InstanceType<typeof ChatConversationFloatingScroll> | null>(null);
const collapsedConversationSectionKeys = ref<Record<string, boolean>>({});
const conversationTabTransitionName = ref("conversation-tab-slide-left");
const activeConversationTab = computed({
  get: (): ConversationSidebarTab => {
    if (props.activeTab === "contact" || props.activeTab === "task") return props.activeTab;
    return "local";
  },
  set: (value: ConversationSidebarTab) => emit("update:activeTab", value),
});
const { conversationStatusById, markConversationRead } = usePipelineStatus({
  activeConversationId: computed(() => String(props.activeConversationId || "").trim()),
});

const conversationPreviewCache = computed(() => new Map(
  props.items.map((item) => [String(item.conversationId || "").trim(), Array.isArray(item.previewMessages) ? item.previewMessages : []]),
));

const conversationSections = computed<ConversationSection[]>(() => {
  const visibleItems = props.items.filter((item) => {
    const kind = String(item.kind || "local_unarchived").trim();
    return activeConversationTab.value === "contact"
      ? kind === "remote_im_contact"
      : kind !== "remote_im_contact";
  });
  const pinned = visibleItems.filter((item) => !!item.isPinned || !!item.isSystemNotificationConversation);
  const others = visibleItems.filter((item) => !item.isPinned && !item.isSystemNotificationConversation);
  const sections: ConversationSection[] = [];
  if (pinned.length > 0) {
    sections.push({
      key: "pinned",
      title: t("chat.pinnedConversations"),
      items: pinned,
    });
  }
  if (activeConversationTab.value === "contact") {
    return [
      ...sections,
      ...buildRemoteConversationSections(others, {
        fallbackTitle: t("chat.otherConversations"),
        locale: locale.value,
      }),
    ];
  }
  return [
    ...sections,
    ...buildWorkspaceConversationSections(others, {
      defaultWorkspaceTitle: t("chat.defaultWorkspace"),
      locale: locale.value,
    }),
  ];
});

const normalizedConversationSearchQuery = computed(() =>
  String(conversationSearchQuery.value || "").trim().toLocaleLowerCase(),
);

const searchPlaceholder = computed(() =>
  activeConversationTab.value === "task"
    ? t("chat.taskSidebar.searchPlaceholder")
    : t("chat.conversationSearchPlaceholder"),
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

watch(
  () => props.activeConversationId,
  (conversationId) => markConversationRead(conversationId),
  { immediate: true },
);

watch(
  () => activeConversationTab.value,
  (nextValue, previousValue) => {
    if (!previousValue || nextValue === previousValue) return;
    conversationTabTransitionName.value = conversationTabOrder(nextValue) > conversationTabOrder(previousValue)
      ? "conversation-tab-slide-left"
      : "conversation-tab-slide-right";
    if (nextValue === "task") {
      resetConversationTitleEdit();
    }
  },
);

watch(showSearch, async (visible) => {
  if (visible) {
    await nextTick();
    searchInputRef.value?.focus();
  } else {
    conversationSearchQuery.value = "";
  }
});

function resetConversationTitleEdit() {
  editingConversationId.value = "";
  editingTitleDraft.value = "";
}

function isConversationSectionCollapsed(key: string): boolean {
  return !!collapsedConversationSectionKeys.value[key];
}

function toggleConversationSection(key: string) {
  collapsedConversationSectionKeys.value = {
    ...collapsedConversationSectionKeys.value,
    [key]: !collapsedConversationSectionKeys.value[key],
  };
}

function conversationTabOrder(value: ConversationSidebarTab): number {
  if (value === "task") return 2;
  if (value === "contact") return 1;
  return 0;
}

function requestConversationTabChange(value: ConversationSidebarTab) {
  if (value === activeConversationTab.value) return;
  conversationTabTransitionName.value = conversationTabOrder(value) > conversationTabOrder(activeConversationTab.value)
    ? "conversation-tab-slide-left"
    : "conversation-tab-slide-right";
  emit("update:activeTab", value);
}

function requestTaskEdit(task: TaskEntry) {
  emit("editTask", task);
}

function scheduleConversationListScrollbarUpdate() {
  void nextTick(() => {
    requestAnimationFrame(() => conversationFloatingScrollRef.value?.updateThumb());
  });
}

function handleConversationTabTransitionSettled() {
  scheduleConversationListScrollbarUpdate();
}

function createConversationInSection(section: ConversationSection) {
  const path = String(section.workspaceRootPath || "").trim();
  if (!path) return;
  window.dispatchEvent(new CustomEvent("easy-call:open-create-conversation-dialog", {
    detail: {
      workspace: {
        id: `conversation-workspace-${path}`,
        name: section.title,
        path,
        level: "main",
        access: "approval",
        builtIn: false,
      },
    },
  }));
}

function setRenameInputRef(element: Element | { $el?: Element | null } | null) {
  renameInputRef.value = element instanceof HTMLInputElement ? element : null;
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

function isCurrentConversation(item: ChatConversationOverviewItem): boolean {
  return String(item.conversationId || "").trim() === String(props.activeConversationId || "").trim();
}

function conversationIndicatorTone(item: ChatConversationOverviewItem): "error" | "info" | "success" | "" {
  if (isCurrentConversation(item)) return "";
  const conversationId = String(item.conversationId || "").trim();
  if (!conversationId) return "";
  const pipelineStatus = conversationStatusById.value[conversationId];
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

function isConversationDisabled(item: ChatConversationOverviewItem): boolean {
  return item.runtimeState === "organizing_context"
    || item.runtimeState === "archiving"
    || item.runtimeState === "compacting"
    || !!item.detachedWindowOpen;
}

function isLocalConversation(item: ChatConversationOverviewItem): boolean {
  return item.kind !== "remote_im_contact";
}

function shouldShowConversationMenu(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item) && !isConversationDisabled(item);
}

function canRenameConversation(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item)
    && !item.isSystemNotificationConversation
    && !isConversationDisabled(item)
    && isCurrentConversation(item);
}

function isEditingTitle(item: ChatConversationOverviewItem): boolean {
  return String(item.conversationId || "").trim() === String(editingConversationId.value || "").trim();
}

function conversationDisplayTitle(item: ChatConversationOverviewItem): string {
  return resolveConversationDisplayTitle(item, {
    locale: locale.value,
    untitledLabel: t("chat.untitledConversation"),
  });
}

function conversationItemTitle(item: ChatConversationOverviewItem): string {
  if (item.detachedWindowOpen) return t("chat.detachedWindowOpen");
  if (item.runtimeState === "archiving") return runtimeStateText("archiving");
  if (item.runtimeState === "compacting") return runtimeStateText("compacting");
  if (isConversationDisabled(item)) return t("chat.organizingContextDisabled");
  return item.workspaceLabel || t("chat.defaultWorkspace");
}

function handleConversationCardClick(item: ChatConversationOverviewItem) {
  const conversationId = String(item.conversationId || "").trim();
  if (isCurrentConversation(item) || isConversationDisabled(item)) return;
  emit("select", {
    conversationId,
    kind: item.kind,
    remoteContactId: String(item.remoteContactId || "").trim() || undefined,
  });
}

function canToggleConversationPin(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item) && !item.isSystemNotificationConversation && !isConversationDisabled(item);
}

function canArchiveConversation(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item)
    && !item.isSystemNotificationConversation
    && Number(item.messageCount || 0) > 3
    && item.hasAssistantReply !== false
    && !isConversationDisabled(item);
}

function canExportConversation(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item) && !isConversationDisabled(item);
}

function pinConversationTitle(item: ChatConversationOverviewItem): string {
  if (item.isSystemNotificationConversation) return t("chat.mainConversationPinned");
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

function conversationMenuPlacementClass(itemIndex: number, total: number): string {
  if (total <= 0) return "dropdown-bottom";
  return itemIndex < Math.ceil(total / 2) ? "dropdown-bottom" : "dropdown-top";
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
  emit("rename", {
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
  return previewText(latestPreview);
}

function formatConversationTime(value?: string): string {
  return formatConversationListTime(value, locale.value);
}

function systemPersonaLabel(): string {
  return props.personaNameMap?.[SYSTEM_PERSONA_ID] || "P-ai系统";
}

function systemPersonaInitial(): string {
  return systemPersonaLabel().charAt(0).toUpperCase() || "P";
}

function systemPersonaAvatarUrl(): string {
  return props.personaAvatarUrlMap?.[SYSTEM_PERSONA_ID] || "";
}

function sideListLastSpeakerInitial(item: ChatConversationOverviewItem): string {
  if (item.isSystemNotificationConversation) return systemPersonaInitial();
  const previews = normalizedPreviewMessages(item);
  if (previews.length === 0) return "?";
  return speakerLabel(previews[previews.length - 1]).charAt(0).toUpperCase();
}

function sideListLastSpeakerLabel(item: ChatConversationOverviewItem): string {
  if (item.isSystemNotificationConversation) return systemPersonaLabel();
  const previews = normalizedPreviewMessages(item);
  if (previews.length === 0) return "";
  return speakerLabel(previews[previews.length - 1]);
}

function sideListLastSpeakerAvatarUrl(item: ChatConversationOverviewItem): string {
  if (item.isSystemNotificationConversation) return systemPersonaAvatarUrl();
  const previews = normalizedPreviewMessages(item);
  if (previews.length === 0) return "";
  const speakerId = String(previews[previews.length - 1].speakerAgentId || "").trim();
  if (!speakerId || speakerId === "user-persona") {
    return props.userAvatarUrl || "";
  }
  return props.personaAvatarUrlMap?.[speakerId] || "";
}
</script>

<style scoped>
.conversation-tab-panel {
  min-height: 100%;
}

.conversation-time-container {
  container-type: inline-size;
}

@container (max-width: 229px) {
  .conversation-time-label {
    display: none;
  }
}

.conversation-tab-slide-left-enter-active,
.conversation-tab-slide-left-leave-active,
.conversation-tab-slide-right-enter-active,
.conversation-tab-slide-right-leave-active {
  transition:
    opacity 120ms ease,
    transform 120ms cubic-bezier(0.22, 1, 0.36, 1);
}

.conversation-tab-slide-left-enter-from,
.conversation-tab-slide-right-leave-to {
  opacity: 0;
  transform: translateX(12px);
}

.conversation-tab-slide-left-leave-to,
.conversation-tab-slide-right-enter-from {
  opacity: 0;
  transform: translateX(-12px);
}
</style>
