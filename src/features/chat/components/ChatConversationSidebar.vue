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
              v-for="section in displayedConversationSections"
              :key="section.key"
              :ref="(el) => setConversationSectionElement(section.key, el)"
              :title="section.title"
              :count="section.totalItemCount"
              :model-value="isConversationSectionCollapsed(section.key)"
              :draggable="isConversationSectionDraggable(section)"
              :drop-indicator="conversationSectionDragIndicator(section)"
              @update:model-value="toggleConversationSection(section.key)"
              @collapse-all="collapseAllConversationSections"
              @after-enter="scheduleConversationListScrollbarUpdate"
              @after-leave="scheduleConversationListScrollbarUpdate"
              @dragstart="handleConversationSectionDragStart(section, $event)"
              @dragover="handleConversationSectionDragOver(section, $event)"
              @drop="handleConversationSectionDrop(section, $event)"
              @dragend="handleConversationSectionDragEnd"
            >
            <template #actions>
              <button
                v-if="section.workspaceRootPath"
                type="button"
                class="btn btn-ghost btn-xs ml-auto h-6 min-h-6 w-6 min-w-6 shrink-0 p-0 text-base-content opacity-0 transition-opacity group-hover/section:opacity-100"
                :title="t('chat.newConversation')"
                @click.stop="createConversationInSection(section)"
                @dblclick.stop
              >
                <SquarePen class="h-3.5 w-3.5" />
              </button>
            </template>
            <div
              v-for="item in section.visibleItems"
              :key="item.conversationId"
              class="group relative mx-1"
              @contextmenu.prevent="handleCardContextMenu(item, $event)"
              @pointerdown="handleCardPointerDown(item, $event)"
              @pointerup="handleCardPointerUp(item)"
              @pointerleave="handleCardPointerLeave"
            >
                  <div
                    class="block rounded-lg px-2 text-left transition-colors hover:bg-base-100/70"
                    :class="[
                      item.conversationId === activeConversationId ? 'bg-base-300 hover:bg-base-300' : 'bg-transparent',
                      isConversationVisuallyOccupied(item) ? 'opacity-60' : '',
                      isCurrentConversation(item) ? 'cursor-default' : 'cursor-pointer',
                    ]"
                    :role="isCurrentConversation(item) ? undefined : 'button'"
                    :tabindex="isCurrentConversation(item) ? undefined : 0"
                    :title="conversationItemTitle(item)"
                    @click="handleConversationCardClick(item)"
                    @keydown.enter.prevent="handleConversationCardClick(item)"
                    @keydown.space.prevent="handleConversationCardClick(item)"
                  >
                    <div class="flex items-center gap-2 py-1">
                    <div class="shrink-0">
                      <div class="indicator">
                        <span
                          v-if="conversationIndicatorTone(item)"
                          class="indicator-item indicator-top indicator-end z-10 h-2.5 w-2.5 translate-x-0.5 -translate-y-0.5 rounded-full"
                          :class="conversationIndicatorClass(conversationIndicatorTone(item))"
                          aria-hidden="true"
                        ></span>
                        <div class="avatar relative overflow-visible">
                          <div class="flex h-10 w-10 items-center justify-center rounded-full bg-neutral text-neutral-content">
                            <img
                              v-if="sideListDisplaySpeakerAvatarUrl(item)"
                              :src="sideListDisplaySpeakerAvatarUrl(item)"
                              :alt="sideListDisplaySpeakerLabel(item)"
                              class="w-10 h-10 rounded-full object-cover"
                            />
                            <span v-else class="text-sm font-bold">{{ sideListDisplaySpeakerInitial(item) }}</span>
                          </div>
                          <span
                            v-if="isRecentConversationSection(section.key)"
                            class="absolute bottom-0 left-1/2 z-20 inline-block max-w-10 -translate-x-1/2 translate-y-1/3 cursor-pointer truncate rounded-full bg-neutral px-1.5 py-[1px] text-micro font-normal leading-3 text-neutral-content shadow-sm transition-colors hover:bg-primary hover:text-primary-content"
                            :title="t('chat.revealConversationSection')"
                            role="button"
                            tabindex="0"
                            @click.stop="revealConversationSection(item)"
                            @keydown.enter.prevent="revealConversationSection(item)"
                            @keydown.space.prevent="revealConversationSection(item)"
                          >
                            {{ conversationSourceBadgeLabel(item) }}
                          </span>
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
                          <span class="conversation-time-label text-xs text-base-content/60">
                            {{ formatConversationTime(item.updatedAt) }}
                          </span>
                          <FloatingConversationMenu
                            :ref="(el) => { if (el) menuRefs[String(item.conversationId || '').trim()] = el }"
                            v-if="shouldShowConversationMenu(item) && !isEditingTitle(item)"
                            :title="t('common.more')"
                          >
                            <template #default="{ close }">
                              <li v-if="!item.isSystemNotificationConversation">
                                <button
                                  type="button"
                                  :disabled="!canToggleConversationPin(item)"
                                  @click.stop="close(); toggleConversationPin(item)"
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
                                  @click.stop="close(); startConversationTitleEdit(item)"
                                >
                                  <PencilLine class="h-4 w-4" />
                                  <span>{{ t("common.rename") }}</span>
                                </button>
                              </li>
                              <li>
                                <button
                                  type="button"
                                  :disabled="!canExportConversation(item)"
                                  @click.stop="close(); requestConversationExport(item)"
                                >
                                  <Upload class="h-4 w-4" />
                                  <span>{{ t("chat.exportConversation") }}</span>
                                </button>
                              </li>
                              <li v-if="!item.isSystemNotificationConversation">
                                <button
                                  type="button"
                                  :disabled="!canArchiveConversation(item)"
                                  @click.stop="close(); emit('archiveConversation', String(item.conversationId || '').trim())"
                                >
                                  <Archive class="h-4 w-4" />
                                  <span>{{ t('common.archive') }}</span>
                                </button>
                              </li>
                              <li v-if="!item.isSystemNotificationConversation">
                                <button
                                  type="button"
                                  :disabled="!canDeleteConversation(item)"
                                  class="text-error"
                                  @click.stop="close(); emit('deleteConversation', String(item.conversationId || '').trim())"
                                >
                                  <Trash2 class="h-4 w-4" />
                                  <span>{{ t('common.delete') }}</span>
                                </button>
                              </li>
                            </template>
                          </FloatingConversationMenu>
                        </div>
                      </div>

                      <div class="mt-1 flex items-center justify-between gap-2 text-xs">
                        <span class="min-w-0 truncate opacity-60">
                          {{ latestPreviewLine(item) }}
                        </span>
                        <div class="flex shrink-0 items-center gap-2">
                          <span v-if="conversationBusy(item)" class="loading loading-spinner loading-xs text-primary" :title="conversationStatusText(item)"></span>
                          <span v-else-if="conversationPipelineStatus(item) === 'error'" class="badge badge-error badge-xs">{{ t("common.failed") }}</span>
                          <span v-else-if="conversationStatusText(item)" class="text-xs text-base-content/60">{{ conversationStatusText(item) }}</span>
                          <span
                            v-if="unreadCountBadge(item)"
                            class="inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-error px-1.5 text-xs font-medium text-error-content"
                          >
                            {{ unreadCountBadge(item) }}
                          </span>
                        </div>
                      </div>
                    </div>
                  </div>
                  </div>

                </div>
            <template v-for="item in section.visibleItems" :key="`followers-${item.conversationId}`">
              <button
                v-for="simpleItem in (section.simpleFollowers[String(item.conversationId || '').trim()] || [])"
                :key="`simple-${simpleItem.conversationId}`"
                type="button"
                class="mx-1 flex w-[calc(100%-0.5rem)] items-center rounded-lg py-1 pl-2 pr-2 text-left text-sm transition-colors hover:bg-base-100/70"
                :class="String(simpleItem.conversationId || '').trim() === String(props.activeConversationId || '').trim() ? 'bg-base-300/60' : 'bg-transparent'"
                :title="conversationDisplayTitle(simpleItem)"
                @click="handleConversationCardClick(simpleItem)"
              >
                <span class="relative w-10 shrink-0 self-stretch" aria-hidden="true">
                  <span
                    class="absolute right-0 top-1 bottom-1 w-1 rounded-full transition-colors"
                    :class="simpleItemIndicatorClass(simpleItem)"
                  ></span>
                </span>
                <span class="min-w-0 truncate pl-2 font-medium">{{ conversationDisplayTitle(simpleItem) }}</span>
                <span class="ml-auto shrink-0 tabular-nums text-xs text-base-content/45">{{ formatConversationTime(simpleItem.updatedAt) }}</span>
              </button>
            </template>
            <div v-if="section.hiddenItemCount > 0" class="px-3 pb-2 pt-1">
              <button
                type="button"
                class="btn btn-ghost btn-xs h-7 min-h-7 w-full justify-center text-base-content/65 hover:text-base-content"
                @click.stop="loadMoreConversationsInSection(section.key)"
              >
                {{ t("chat.loadMore") }}
              </button>
            </div>
            </CollapsibleGroup>
            <div
              v-if="displayedConversationSections.length === 0"
              class="px-3 py-4 text-center text-sm text-base-content/60"
            >
              {{ t("chat.conversationSearchEmpty") }}
            </div>
          </template>
        </div>
      </Transition>
    </ChatConversationFloatingScroll>
    <div class="shrink-0 border-t border-base-300 bg-base-100/95 px-2 py-2 backdrop-blur">
      <div class="flex items-center justify-between gap-2">
        <div class="avatar">
          <div class="flex h-9 w-9 items-center justify-center rounded-full bg-neutral text-neutral-content">
            <img
              v-if="props.userAvatarUrl"
              :src="props.userAvatarUrl"
              :alt="props.userAlias || t('chat.userAvatarAlt')"
              class="h-9 w-9 rounded-full object-cover"
            />
            <span v-else class="text-sm font-bold">{{ userAvatarInitial }}</span>
          </div>
        </div>
        <button
          type="button"
          class="btn btn-ghost btn-xs min-h-8 rounded-xl px-3 text-sm font-bold"
          @click="openBatchArchiveCard"
        >
          <span>{{ t("chat.batchArchive.entryAction") }}</span>
          <ChevronRight class="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
    <dialog class="modal" :class="{ 'modal-open': batchArchiveCardOpen }">
      <div class="modal-box flex h-[min(88vh,44rem)] w-[min(94vw,64rem)] max-w-none flex-col overflow-hidden p-0">
        <div class="flex shrink-0 items-center justify-between gap-3 border-b border-base-300 px-5 py-4">
          <h3 class="text-base font-semibold">{{ t("chat.batchArchive.title") }}</h3>
        </div>
        <div class="flex min-h-0 flex-1 flex-col bg-base-200/35 px-5 py-4">
          <section class="shrink-0">
            <div class="text-sm font-semibold">{{ t("chat.batchArchive.conditionTitle") }}</div>
            <div class="mt-3 grid gap-4 md:grid-cols-[minmax(0,1fr)_minmax(0,1.8fr)]">
              <div class="space-y-2">
                <label class="block text-sm font-medium" for="batch-archive-days">
                  {{ t("chat.batchArchive.daysLabel") }}
                </label>
                <div class="flex items-center gap-2">
                  <input
                    id="batch-archive-days"
                    v-model.number="batchArchiveDays"
                    type="number"
                    min="1"
                    step="1"
                    class="input input-bordered w-28"
                  />
                  <span class="text-sm text-base-content/70">{{ t("chat.batchArchive.daysSuffix") }}</span>
                </div>
              </div>
              <div class="space-y-2">
                <label class="block text-sm font-medium" for="batch-archive-model">
                  {{ t("chat.batchArchive.modelLabel") }}
                </label>
                <ApiConfigTreeSelect
                  id="batch-archive-model"
                  v-model="batchArchiveSelectedModelId"
                  :api-configs="batchArchiveApiConfigs"
                />
              </div>
            </div>
            <label class="mt-4 flex items-start gap-3 px-1 py-1">
              <input
                v-model="batchArchiveKeepOnePerWorkspace"
                type="checkbox"
                class="checkbox checkbox-sm mt-0.5"
              />
              <div class="min-w-0 space-y-1 text-sm">
                <div class="font-medium leading-5">{{ t("chat.batchArchive.keepOnePerWorkspace") }}</div>
                <div class="text-xs leading-5 text-base-content/65">{{ t("chat.batchArchive.keepOnePerWorkspaceHint") }}</div>
              </div>
            </label>
          </section>

          <section class="mt-5 flex min-h-0 flex-1 flex-col">
            <div class="flex items-center justify-between gap-3">
              <div class="text-sm font-semibold">
                {{ t("chat.batchArchive.previewTitle", { count: batchArchiveCandidateConversations.length }) }}
              </div>
            </div>
            <div v-if="batchArchiveCandidateConversations.length === 0" class="mt-3 px-1 py-4 text-sm text-base-content/60">
              {{ t("chat.batchArchive.empty") }}
            </div>
            <div v-else class="mt-3 min-h-0 flex-1 space-y-2 overflow-auto pr-1">
              <article
                v-for="item in batchArchiveCandidateConversations"
                :key="item.conversationId"
                class="flex items-center gap-3 rounded-lg border border-base-300 bg-base-100 px-3 py-2"
              >
                <input
                  type="checkbox"
                  class="checkbox checkbox-sm shrink-0"
                  :checked="isBatchArchiveConversationSelected(item)"
                  @change="toggleBatchArchiveConversation(item, ($event.target as HTMLInputElement).checked)"
                />
                <Archive class="h-4 w-4 shrink-0 text-base-content/55" />
                <div class="min-w-0 flex-1">
                  <div class="truncate text-sm font-medium">{{ conversationDisplayTitle(item) }}</div>
                </div>
                <div class="badge badge-ghost max-w-28 shrink-0 truncate">
                  {{ conversationWorkspaceLabel(item) }}
                </div>
                <div class="w-24 shrink-0 text-right text-xs text-base-content/65">
                  {{ t("chat.batchArchive.olderThanDays", { count: conversationAgeDays(item) }) }}
                </div>
                <div class="w-12 shrink-0 text-right text-xs text-base-content/60">
                  {{ formatConversationTime(item.updatedAt) }}
                </div>
              </article>
            </div>
          </section>
        </div>
        <div class="flex shrink-0 items-center justify-between gap-3 border-t border-base-300 px-5 py-4">
          <div class="flex items-center gap-2">
            <button type="button" class="btn btn-ghost btn-sm" :disabled="batchArchiveCandidateConversations.length === 0" @click="selectAllBatchArchiveCandidates">
              {{ t("chat.batchArchive.selectAll") }}
            </button>
            <button type="button" class="btn btn-ghost btn-sm" :disabled="batchArchiveSelectedConversationIds.size === 0" @click="clearBatchArchiveSelection">
              {{ t("chat.batchArchive.selectNone") }}
            </button>
            <span class="text-xs text-base-content/60">
              {{ t("chat.batchArchive.selectedCount", { count: batchArchiveSelectedConversationIds.size }) }}
            </span>
          </div>
          <div class="flex items-center gap-2">
            <button type="button" class="btn btn-sm" @click="closeBatchArchiveCard">
              {{ t("common.cancel") }}
            </button>
            <button
              type="button"
              class="btn btn-primary btn-sm"
              :disabled="batchArchiveStartDisabled"
              @click="submitBatchArchive"
            >
              <span v-if="batchArchiveSubmitting" class="loading loading-spinner loading-xs"></span>
              {{ t("chat.batchArchive.startArchive") }}
            </button>
          </div>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click.prevent="closeBatchArchiveCard">close</button>
      </form>
    </dialog>
  </aside>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch, watchEffect } from "vue";
import { useI18n } from "vue-i18n";
import { Archive, ChevronRight, PencilLine, Pin, PinOff, Search, SquarePen, Trash2, Upload } from "@lucide/vue";
import CollapsibleGroup from "./CollapsibleGroup.vue";
import FloatingConversationMenu from "./FloatingConversationMenu.vue";
import type { ApiConfigItem, ChatConversationOverviewItem, ConversationPreviewMessage } from "../../../types/app";
import { stripToolcallMarkers } from "../../../utils/chat-message-semantics";
import type { TaskEntry } from "../../config/views/config-tabs/task-editor";
import { invokeTauri } from "../../../services/tauri-api";
import { usePipelineStatus } from "../../shell/composables/use-pipeline-status";
import ApiConfigTreeSelect from "../../config/components/ApiConfigTreeSelect.vue";
import { formatConversationListTime } from "../utils/conversation-time";
import {
  aggregateConversationItems,
  conversationLastUsedMs,
} from "../utils/conversation-aggregation";
import {
  applyConversationSectionOrder,
  buildRecentConversationSection,
  buildRemoteConversationSections,
  buildWorkspaceConversationSections,
  RECENT_CONVERSATION_SECTION_KEY,
  workspaceNameFromPath,
  type ConversationSectionOrderState,
  type ConversationSection,
} from "../utils/conversation-sections";
import { resolveConversationDisplayTitle } from "../utils/conversation-title";
import ChatConversationFloatingScroll from "./ChatConversationFloatingScroll.vue";
import ChatTaskSidebarPanel from "./ChatTaskSidebarPanel.vue";

type ConversationSidebarTab = "local" | "contact" | "task";
type DisplayConversationSection = ConversationSection & {
  visibleItems: ChatConversationOverviewItem[];
  /** full 会话 id → 聚合其后的简单条目（同人格旧会话，按更新时间倒序） */
  simpleFollowers: Record<string, ChatConversationOverviewItem[]>;
  hiddenItemCount: number;
  totalItemCount: number;
};
type BatchArchiveConversationsOutput = {
  success: boolean;
  acceptedConversationIds: string[];
  skipped: Array<{ conversationId: string; reason: string }>;
  activeConversationId?: string;
};

const CONVERSATION_SECTION_UNUSED_DAYS = 7;
const CONVERSATION_SECTION_MIN_VISIBLE = 5;
const CONVERSATION_SECTION_LOAD_MORE_STEP = 10;
const CONVERSATION_SECTION_RESET_DELAY_MS = 30_000;

const props = defineProps<{
  items: ChatConversationOverviewItem[];
  activeConversationId: string;
  userAlias: string;
  userAvatarUrl: string;
  personaNameMap: Record<string, string>;
  personaAvatarUrlMap: Record<string, string>;
  activeTab: ConversationSidebarTab;
  chatModelOptions: ApiConfigItem[];
  toolReviewApiConfigId?: string;
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
  (e: "batchArchiveCompleted", payload: { archivedConversationIds: string[]; activeConversationId?: string }): void;
}>();

const { t, locale } = useI18n();
const SYSTEM_PERSONA_ID = "system-persona";
const renameInputRef = ref<HTMLInputElement | null>(null);
const editingConversationId = ref("");
const editingTitleDraft = ref("");
const conversationSearchQuery = ref("");
const menuRefs = ref<Record<string, any>>({});
const showSearch = ref(false);
const searchInputRef = ref<HTMLInputElement | null>(null);
const batchArchiveCardOpen = ref(false);
const batchArchiveDays = ref(30);
const batchArchiveKeepOnePerWorkspace = ref(true);
const batchArchiveSelectedModelId = ref("");
const batchArchiveSelectedConversationIds = ref<Set<string>>(new Set());
const batchArchiveSubmitting = ref(false);
const conversationFloatingScrollRef = ref<InstanceType<typeof ChatConversationFloatingScroll> | null>(null);
const collapsedConversationSectionKeys = ref<Record<string, boolean>>({});
const conversationSectionOrders = ref<ConversationSectionOrderState>({ local: [], contact: [] });
const conversationSectionLoadMoreCounts = ref<Record<string, number>>({});
const conversationSectionResetTimers = new Map<ConversationSidebarTab, ReturnType<typeof setTimeout>>();
const draggingConversationSectionKey = ref("");
const dragOverConversationSectionKey = ref("");
const dragOverConversationSectionPlacement = ref<"before" | "after">("before");
const savingConversationSectionOrder = ref(false);
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
  const recentSection = buildRecentConversationSection(visibleItems, t("chat.recentConversations"));
  const sections: ConversationSection[] = [];
  if (pinned.length > 0) {
    sections.push({
      key: "pinned",
      title: t("chat.pinnedConversations"),
      items: pinned,
    });
  }
  if (recentSection) {
    sections.push(recentSection);
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

const orderedConversationSections = computed<ConversationSection[]>(() => {
  const tab = activeConversationTab.value === "contact" ? "contact" : "local";
  const result = applyConversationSectionOrder(conversationSections.value, conversationSectionOrders.value[tab]);
  if (result.changed && !savingConversationSectionOrder.value) {
    conversationSectionOrders.value = {
      ...conversationSectionOrders.value,
      [tab]: result.nextOrder,
    };
    void persistConversationSectionOrder(tab, result.nextOrder);
  }
  return result.sections;
});

const normalizedConversationSearchQuery = computed(() =>
  String(conversationSearchQuery.value || "").trim().toLocaleLowerCase(),
);

const searchPlaceholder = computed(() =>
  activeConversationTab.value === "task"
    ? t("chat.taskSidebar.searchPlaceholder")
    : t("chat.conversationSearchPlaceholder"),
);

const userAvatarInitial = computed(() => {
  const text = String(props.userAlias || t("chat.userAvatarAlt")).trim();
  return text.charAt(0).toUpperCase() || "U";
});

const batchArchiveApiConfigs = computed(() => props.chatModelOptions.filter((item) => item.enableText));
const batchArchiveModelOptions = computed<Array<{ id: string }>>(() =>
  batchArchiveApiConfigs.value
    .map((item) => ({ id: String(item.id || "").trim() }))
    .filter((item) => !!item.id),
);

const batchArchiveCandidateConversations = computed(() => {
  const thresholdDays = Math.max(1, Math.round(Number(batchArchiveDays.value || 0)));
  const now = Date.now();
  const oldLocalConversations = props.items
    .filter((item) => {
      if (!isLocalConversation(item) || item.isSystemNotificationConversation) return false;
      const updatedAt = Date.parse(String(item.updatedAt || item.lastMessageAt || "").trim());
      if (!Number.isFinite(updatedAt)) return false;
      const ageDays = Math.floor((now - updatedAt) / 86_400_000);
      return ageDays >= thresholdDays;
    });
  if (!batchArchiveKeepOnePerWorkspace.value) {
    return sortBatchArchiveCandidates(oldLocalConversations);
  }
  const preservedConversationIds = new Set<string>();
  const newestByWorkspace = new Map<string, ChatConversationOverviewItem>();
  for (const item of oldLocalConversations) {
    const workspaceKey = conversationWorkspaceKey(item);
    const current = newestByWorkspace.get(workspaceKey);
    if (!current || conversationTimeValue(item) > conversationTimeValue(current)) {
      newestByWorkspace.set(workspaceKey, item);
    }
  }
  for (const item of newestByWorkspace.values()) {
    preservedConversationIds.add(String(item.conversationId || "").trim());
  }
  return sortBatchArchiveCandidates(oldLocalConversations.filter((item) =>
    !preservedConversationIds.has(String(item.conversationId || "").trim()),
  ));
});

const batchArchiveSelectedCandidateIds = computed(() => {
  const selectedIds = batchArchiveSelectedConversationIds.value;
  return batchArchiveCandidateConversations.value
    .map(batchArchiveConversationId)
    .filter((id) => !!id && selectedIds.has(id));
});

const batchArchiveStartDisabled = computed(() =>
  batchArchiveSubmitting.value
  || batchArchiveSelectedCandidateIds.value.length === 0
  || !String(batchArchiveSelectedModelId.value || "").trim(),
);

watch(
  () => [props.toolReviewApiConfigId, batchArchiveModelOptions.value.map((item) => item.id).join("|")] as const,
  () => {
    const quickModelId = String(props.toolReviewApiConfigId || "").trim();
    const optionIds = batchArchiveModelOptions.value.map((item) => item.id);
    if (quickModelId && optionIds.includes(quickModelId)) {
      batchArchiveSelectedModelId.value = quickModelId;
      return;
    }
    if (!optionIds.includes(batchArchiveSelectedModelId.value)) {
      batchArchiveSelectedModelId.value = optionIds[0] || "";
    }
  },
  { immediate: true },
);

watch(
  () => batchArchiveCandidateConversations.value.map((item) => String(item.conversationId || "").trim()).filter(Boolean),
  (candidateIds) => {
    const candidateSet = new Set(candidateIds);
    const next = new Set<string>();
    for (const id of batchArchiveSelectedConversationIds.value) {
      if (candidateSet.has(id)) next.add(id);
    }
    batchArchiveSelectedConversationIds.value = next;
  },
  { immediate: true },
);

const filteredConversationSections = computed(() => {
  const query = normalizedConversationSearchQuery.value;
  if (!query) return orderedConversationSections.value;
  return orderedConversationSections.value
    .map((section) => ({
      ...section,
      items: section.items.filter((item) => conversationMatchesSearch(item, query)),
    }))
    .filter((section) => section.items.length > 0);
});

const displayedConversationSections = computed<DisplayConversationSection[]>(() =>
  filteredConversationSections.value.map((section) => buildDisplayedConversationSection(section)),
);

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
    if (previousValue && previousValue !== nextValue) {
      scheduleConversationSectionReset(previousValue);
    }
    clearConversationSectionResetTimer(nextValue);
    if (!previousValue || nextValue === previousValue) return;
    conversationTabTransitionName.value = conversationTabOrder(nextValue) > conversationTabOrder(previousValue)
      ? "conversation-tab-slide-left"
      : "conversation-tab-slide-right";
    if (nextValue === "task") {
      resetConversationTitleEdit();
    }
  },
);

onMounted(() => {
  void loadConversationSectionOrders();
});

onBeforeUnmount(() => {
  clearConversationSectionResetTimer("local");
  clearConversationSectionResetTimer("contact");
  clearConversationSectionResetTimer("task");
});

async function loadConversationSectionOrders() {
  try {
    const result = await invokeTauri<ConversationSectionOrderState>("conversation.sectionOrders.get");
    conversationSectionOrders.value = {
      local: Array.isArray(result?.local) ? result.local.map((item) => String(item || "").trim()).filter(Boolean) : [],
      contact: Array.isArray(result?.contact) ? result.contact.map((item) => String(item || "").trim()).filter(Boolean) : [],
    };
  } catch (error) {
    console.warn("[会话分组排序] 读取失败", { error });
  }
}

async function persistConversationSectionOrder(tab: "local" | "contact", orderedKeys: string[]) {
  savingConversationSectionOrder.value = true;
  try {
    const result = await invokeTauri<{ orderedKeys?: string[] }>("conversation.sectionOrders.save", {
      input: {
        tab,
        orderedKeys,
      },
    });
    conversationSectionOrders.value = {
      ...conversationSectionOrders.value,
      [tab]: Array.isArray(result?.orderedKeys)
        ? result.orderedKeys.map((item) => String(item || "").trim()).filter(Boolean)
        : orderedKeys,
    };
  } catch (error) {
    console.warn("[会话分组排序] 保存失败", { tab, error });
  } finally {
    savingConversationSectionOrder.value = false;
  }
}

function isConversationSectionDraggable(section: ConversationSection): boolean {
  if (normalizedConversationSearchQuery.value) return false;
  return section.key !== "pinned" && section.key !== RECENT_CONVERSATION_SECTION_KEY;
}

function handleConversationSectionDragStart(section: ConversationSection, event: DragEvent) {
  handleConversationSectionDragEnd();
  if (!isConversationSectionDraggable(section)) {
    event.preventDefault();
    event.stopPropagation();
    return;
  }
  draggingConversationSectionKey.value = section.key;
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", section.key);
    const currentTarget = event.currentTarget;
    if (currentTarget instanceof HTMLElement) {
      event.dataTransfer.setDragImage(currentTarget, 16, 16);
    }
  }
}

function handleConversationSectionDragOver(section: ConversationSection, event: DragEvent) {
  if (!draggingConversationSectionKey.value || !isConversationSectionDraggable(section)) return;
  event.preventDefault();
  const currentTarget = event.currentTarget;
  if (currentTarget instanceof HTMLElement) {
    const rect = currentTarget.getBoundingClientRect();
    const offsetY = event.clientY - rect.top;
    dragOverConversationSectionPlacement.value = offsetY >= rect.height / 2 ? "after" : "before";
    dragOverConversationSectionKey.value = section.key;
  }
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = "move";
  }
}

function handleConversationSectionDragEnd() {
  draggingConversationSectionKey.value = "";
  dragOverConversationSectionKey.value = "";
  dragOverConversationSectionPlacement.value = "before";
}

function handleConversationSectionDrop(section: ConversationSection, event: DragEvent) {
  const draggingKey = String(draggingConversationSectionKey.value || "").trim();
  if (!draggingKey || draggingKey === section.key || !isConversationSectionDraggable(section)) {
    handleConversationSectionDragEnd();
    return;
  }
  event.preventDefault();
  const tab = activeConversationTab.value === "contact" ? "contact" : "local";
  const draggableKeys = orderedConversationSections.value
    .filter((item) => isConversationSectionDraggable(item))
    .map((item) => item.key);
  const fromIndex = draggableKeys.indexOf(draggingKey);
  const rawToIndex = draggableKeys.indexOf(section.key);
  const dropPlacement = dragOverConversationSectionKey.value === section.key
    ? dragOverConversationSectionPlacement.value
    : "before";
  const toIndex = dropPlacement === "after" ? rawToIndex + 1 : rawToIndex;
  if (fromIndex < 0 || toIndex < 0) {
    handleConversationSectionDragEnd();
    return;
  }
  const nextDraggableKeys = [...draggableKeys];
  const [moved] = nextDraggableKeys.splice(fromIndex, 1);
  const adjustedToIndex = fromIndex < toIndex ? toIndex - 1 : toIndex;
  nextDraggableKeys.splice(adjustedToIndex, 0, moved);
  const fixedPrefix = orderedConversationSections.value
    .filter((item) => !isConversationSectionDraggable(item))
    .map((item) => item.key);
  const nextOrder = [...fixedPrefix, ...nextDraggableKeys];
  conversationSectionOrders.value = {
    ...conversationSectionOrders.value,
    [tab]: nextOrder,
  };
  handleConversationSectionDragEnd();
  void persistConversationSectionOrder(tab, nextOrder);
}

function conversationSectionDragIndicator(section: ConversationSection): "before" | "after" | null {
  if (dragOverConversationSectionKey.value !== section.key) return null;
  if (!draggingConversationSectionKey.value) return null;
  return dragOverConversationSectionPlacement.value;
}

function defaultVisibleConversationCount(section: ConversationSection): number {
  const items = Array.isArray(section.items) ? section.items : [];
  if (items.length <= CONVERSATION_SECTION_MIN_VISIBLE) return items.length;
  if (normalizedConversationSearchQuery.value) return items.length;
  if (section.key === "pinned" || section.key === RECENT_CONVERSATION_SECTION_KEY) return items.length;
  const thresholdMs = Date.now() - CONVERSATION_SECTION_UNUSED_DAYS * 24 * 60 * 60 * 1000;
  const recentCount = items.reduce((count, item) => (
    conversationLastUsedMs(item) >= thresholdMs ? count + 1 : count
  ), 0);
  const activeConversationId = String(props.activeConversationId || "").trim();
  const activeIndex = activeConversationId
    ? items.findIndex((item) => String(item.conversationId || "").trim() === activeConversationId)
    : -1;
  return Math.min(
    items.length,
    Math.max(CONVERSATION_SECTION_MIN_VISIBLE, recentCount, activeIndex >= 0 ? activeIndex + 1 : 0),
  );
}

function buildDisplayedConversationSection(section: ConversationSection): DisplayConversationSection {
  const items = Array.isArray(section.items) ? section.items : [];
  const baseVisibleCount = defaultVisibleConversationCount(section);
  const extraVisibleCount = Math.max(0, Number(conversationSectionLoadMoreCounts.value[section.key] || 0));
  const visibleCount = Math.min(items.length, baseVisibleCount + extraVisibleCount);
  const { reorderedItems, simpleFollowers } = aggregateConversationItems(items.slice(0, visibleCount), {
    searchActive: !!normalizedConversationSearchQuery.value,
  });
  return {
    ...section,
    visibleItems: reorderedItems,
    simpleFollowers,
    hiddenItemCount: Math.max(0, items.length - visibleCount),
    totalItemCount: items.length,
  };
}

function loadMoreConversationsInSection(sectionKey: string) {
  const key = String(sectionKey || "").trim();
  if (!key) return;
  conversationSectionLoadMoreCounts.value = {
    ...conversationSectionLoadMoreCounts.value,
    [key]: Math.max(0, Number(conversationSectionLoadMoreCounts.value[key] || 0)) + CONVERSATION_SECTION_LOAD_MORE_STEP,
  };
  scheduleConversationListScrollbarUpdate();
}

function sectionTabFromKey(sectionKey: string): ConversationSidebarTab {
  return sectionKey.startsWith("channel:") ? "contact" : "local";
}

function clearConversationSectionResetTimer(tab: ConversationSidebarTab) {
  const timer = conversationSectionResetTimers.get(tab);
  if (!timer) return;
  clearTimeout(timer);
  conversationSectionResetTimers.delete(tab);
}

function resetConversationSectionLoadMore(tab: ConversationSidebarTab) {
  if (tab === "task") return;
  conversationSectionLoadMoreCounts.value = Object.fromEntries(
    Object.entries(conversationSectionLoadMoreCounts.value)
      .filter(([key, value]) => sectionTabFromKey(key) !== tab || !(Number(value) > 0)),
  );
  scheduleConversationListScrollbarUpdate();
}

function scheduleConversationSectionReset(tab: ConversationSidebarTab) {
  if (tab === "task") return;
  clearConversationSectionResetTimer(tab);
  conversationSectionResetTimers.set(tab, setTimeout(() => {
    conversationSectionResetTimers.delete(tab);
    if (activeConversationTab.value === tab) return;
    resetConversationSectionLoadMore(tab);
  }, CONVERSATION_SECTION_RESET_DELAY_MS));
}

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
  if (normalizedConversationSearchQuery.value) return false;
  return collapsedConversationSectionKeys.value[key] ?? key !== RECENT_CONVERSATION_SECTION_KEY;
}

function toggleConversationSection(key: string) {
  collapsedConversationSectionKeys.value = {
    ...collapsedConversationSectionKeys.value,
    [key]: !isConversationSectionCollapsed(key),
  };
  scheduleConversationListScrollbarUpdate();
}

function expandConversationSection(key: string) {
  if (!key || !isConversationSectionCollapsed(key)) return;
  collapsedConversationSectionKeys.value = {
    ...collapsedConversationSectionKeys.value,
    [key]: false,
  };
  scheduleConversationListScrollbarUpdate();
}

const conversationSectionElements = new Map<string, HTMLElement>();

function setConversationSectionElement(key: string, element: unknown) {
  const root = element instanceof HTMLElement
    ? element
    : (element as { $el?: HTMLElement | null } | null)?.$el ?? null;
  if (root) conversationSectionElements.set(key, root);
  else conversationSectionElements.delete(key);
}

function revealConversationSection(item: ChatConversationOverviewItem) {
  const conversationId = String(item.conversationId || "").trim();
  if (!conversationId) return;
  const section = orderedConversationSections.value.find((entry) =>
    entry.key !== RECENT_CONVERSATION_SECTION_KEY
    && entry.items.some((candidate) => String(candidate.conversationId || "").trim() === conversationId),
  );
  if (!section) return;
  const wasCollapsed = isConversationSectionCollapsed(section.key);
  expandConversationSection(section.key);
  const element = conversationSectionElements.get(section.key);
  window.setTimeout(() => {
    if (element) conversationFloatingScrollRef.value?.scrollToElement(element);
  }, wasCollapsed ? 220 : 0);
}

function collapseAllConversationSections() {
  collapsedConversationSectionKeys.value = conversationSections.value.reduce((next, section) => {
    next[section.key] = true;
    return next;
  }, { ...collapsedConversationSectionKeys.value } as Record<string, boolean>);
  scheduleConversationListScrollbarUpdate();
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

function isConversationVisuallyOccupied(item: ChatConversationOverviewItem): boolean {
  void item;
  return false;
}

function isLocalConversation(item: ChatConversationOverviewItem): boolean {
  return item.kind !== "remote_im_contact";
}

function shouldShowConversationMenu(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item);
}

function canRenameConversation(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item)
    && !item.isSystemNotificationConversation
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
  return item.workspaceLabel || t("chat.defaultWorkspace");
}

function isRecentConversationSection(sectionKey: string): boolean {
  return sectionKey === RECENT_CONVERSATION_SECTION_KEY;
}

function conversationSourceBadgeLabel(item: ChatConversationOverviewItem): string {
  if (item.kind === "remote_im_contact") {
    return String(
      item.channelName
      || item.remoteContactDisplayName
      || item.departmentName
      || t("chat.otherConversations"),
    ).trim();
  }
  const workspacePath = String(item.workspaceRootPath || "").trim();
  return String(
    item.workspaceLabel
    || workspaceNameFromPath(workspacePath)
    || t("chat.defaultWorkspace"),
  ).trim();
}

function simpleItemIndicatorClass(item: ChatConversationOverviewItem): string {
  if (unreadCountBadge(item)) return "bg-error";
  const previews = normalizedPreviewMessages(item);
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

function handleConversationCardClick(item: ChatConversationOverviewItem) {
  const conversationId = String(item.conversationId || "").trim();
  if (isCurrentConversation(item)) return;
  emit("select", {
    conversationId,
    kind: item.kind,
    remoteContactId: String(item.remoteContactId || "").trim() || undefined,
  });
}

let longPressTimer: ReturnType<typeof setTimeout> | null = null;

function clearLongPressTimer() {
  if (longPressTimer !== null) {
    clearTimeout(longPressTimer);
    longPressTimer = null;
  }
}

function handleCardContextMenu(item: ChatConversationOverviewItem, event: MouseEvent) {
  const id = String(item.conversationId || "").trim();
  if (!id) return;
  clearLongPressTimer();
  menuRefs.value[id]?.openMenu(event.clientX, event.clientY);
}

function handleCardPointerDown(item: ChatConversationOverviewItem, event: PointerEvent) {
  if (event.pointerType !== "touch") return;
  const id = String(item.conversationId || "").trim();
  if (!id) return;
  clearLongPressTimer();
  const clientX = event.clientX;
  const clientY = event.clientY;
  longPressTimer = setTimeout(() => {
    menuRefs.value[id]?.openMenu(clientX, clientY);
  }, 500);
}

function handleCardPointerUp(_item: ChatConversationOverviewItem) {
  clearLongPressTimer();
}

function handleCardPointerLeave() {
  clearLongPressTimer();
}

function openBatchArchiveCard() {
  batchArchiveCardOpen.value = true;
  selectAllBatchArchiveCandidates();
}

function closeBatchArchiveCard() {
  if (batchArchiveSubmitting.value) return;
  batchArchiveCardOpen.value = false;
}

function batchArchiveConversationId(item: ChatConversationOverviewItem): string {
  return String(item.conversationId || "").trim();
}

function isBatchArchiveConversationSelected(item: ChatConversationOverviewItem): boolean {
  const id = batchArchiveConversationId(item);
  return !!id && batchArchiveSelectedConversationIds.value.has(id);
}

function toggleBatchArchiveConversation(item: ChatConversationOverviewItem, checked: boolean) {
  const id = batchArchiveConversationId(item);
  if (!id) return;
  const next = new Set(batchArchiveSelectedConversationIds.value);
  if (checked) {
    next.add(id);
  } else {
    next.delete(id);
  }
  batchArchiveSelectedConversationIds.value = next;
}

function selectAllBatchArchiveCandidates() {
  batchArchiveSelectedConversationIds.value = new Set(
    batchArchiveCandidateConversations.value
      .map(batchArchiveConversationId)
      .filter(Boolean),
  );
}

function clearBatchArchiveSelection() {
  batchArchiveSelectedConversationIds.value = new Set();
}

async function submitBatchArchive() {
  const conversationIds = batchArchiveSelectedCandidateIds.value;
  const reflectionApiConfigId = String(batchArchiveSelectedModelId.value || "").trim();
  if (conversationIds.length === 0 || !reflectionApiConfigId || batchArchiveSubmitting.value) return;
  batchArchiveSubmitting.value = true;
  try {
    const payload = { conversationIds, reflectionApiConfigId };
    const result = await invokeTauri<BatchArchiveConversationsOutput>("conversation.batchArchive", payload, 30_000);
    batchArchiveSelectedConversationIds.value = new Set();
    batchArchiveCardOpen.value = false;
    emit("batchArchiveCompleted", {
      archivedConversationIds: Array.isArray(result.acceptedConversationIds) ? result.acceptedConversationIds : [],
      activeConversationId: String(result.activeConversationId || "").trim() || undefined,
    });
    if (Array.isArray(result.skipped) && result.skipped.length > 0) {
      console.warn("[批量归档] 部分会话跳过", result.skipped);
    }
  } catch (error) {
    console.warn("[批量归档] 提交失败", error);
  } finally {
    batchArchiveSubmitting.value = false;
  }
}

function conversationAgeDays(item: ChatConversationOverviewItem): number {
  const updatedAt = conversationTimeValue(item);
  if (!Number.isFinite(updatedAt)) return 0;
  return Math.max(0, Math.floor((Date.now() - updatedAt) / 86_400_000));
}

function conversationTimeValue(item: ChatConversationOverviewItem): number {
  return Date.parse(String(item.updatedAt || item.lastMessageAt || "").trim()) || 0;
}

function sortBatchArchiveCandidates(items: ChatConversationOverviewItem[]): ChatConversationOverviewItem[] {
  return [...items].sort((left, right) => conversationTimeValue(left) - conversationTimeValue(right));
}

function conversationWorkspaceKey(item: ChatConversationOverviewItem): string {
  const rootPath = String(item.workspaceRootPath || "").trim();
  if (rootPath) return rootPath.toLocaleLowerCase();
  return `label:${conversationWorkspaceLabel(item).toLocaleLowerCase()}`;
}

function conversationWorkspaceLabel(item: ChatConversationOverviewItem): string {
  return String(item.workspaceLabel || workspaceNameFromPath(String(item.workspaceRootPath || "").trim()) || t("chat.defaultWorkspace")).trim();
}

function canToggleConversationPin(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item) && !item.isSystemNotificationConversation;
}

function canArchiveConversation(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item) && !item.isSystemNotificationConversation;
}

function canDeleteConversation(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item) && !item.isSystemNotificationConversation;
}

function canExportConversation(item: ChatConversationOverviewItem): boolean {
  return isLocalConversation(item);
}

function pinConversationTitle(item: ChatConversationOverviewItem): string {
  if (item.isSystemNotificationConversation) return t("chat.mainConversationPinned");
  return item.isPinned ? t("chat.unpinConversation") : t("chat.pinConversation");
}

function toggleConversationPin(item: ChatConversationOverviewItem) {
  if (!canToggleConversationPin(item)) return;
  emit("togglePinConversation", String(item.conversationId || "").trim());
}

function requestConversationExport(item: ChatConversationOverviewItem) {
  if (!canExportConversation(item)) return;
  emit("exportConversation", String(item.conversationId || "").trim());
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

function conversationBusy(item: ChatConversationOverviewItem): boolean {
  return conversationPipelineStatus(item) === "busy" || conversationRuntimeBusy(item);
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
  const text = stripToolcallMarkers(preview.textPreview || "");
  if (text) return text;
  if (preview.hasPdf) return t("chat.previewPdf");
  if (preview.hasImage) return t("chat.previewImage");
  if (preview.hasAudio) return t("chat.previewAudio");
  if (preview.hasAttachment) return t("chat.previewAttachment");
  return t("chat.conversationNoPreview");
}

function hasVisiblePreview(preview: ConversationPreviewMessage): boolean {
  return !!stripToolcallMarkers(preview.textPreview || "")
    || !!preview.hasPdf
    || !!preview.hasImage
    || !!preview.hasAudio
    || !!preview.hasAttachment;
}

function latestPreviewLine(item: ChatConversationOverviewItem): string {
  if (conversationBusy(item)) return t("chat.runtimeTyping");
  const previews = normalizedPreviewMessages(item);
  const latestPreview = [...previews].reverse().find(hasVisiblePreview);
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

function sideListConversationAssistantId(item: ChatConversationOverviewItem): string {
  return String(item.agentId || "").trim();
}

function sideListConversationAssistantLabel(item: ChatConversationOverviewItem): string {
  const agentId = sideListConversationAssistantId(item);
  if (!agentId) return sideListLastSpeakerLabel(item);
  return props.personaNameMap?.[agentId] || agentId;
}

function sideListConversationAssistantAvatarUrl(item: ChatConversationOverviewItem): string {
  const agentId = sideListConversationAssistantId(item);
  if (!agentId) return "";
  return props.personaAvatarUrlMap?.[agentId] || "";
}

function sideListDisplaySpeakerLabel(item: ChatConversationOverviewItem): string {
  if (!conversationBusy(item)) return sideListLastSpeakerLabel(item);
  return sideListConversationAssistantLabel(item);
}

function sideListDisplaySpeakerInitial(item: ChatConversationOverviewItem): string {
  return sideListDisplaySpeakerLabel(item).charAt(0).toUpperCase() || "?";
}

function sideListDisplaySpeakerAvatarUrl(item: ChatConversationOverviewItem): string {
  if (!conversationBusy(item)) return sideListLastSpeakerAvatarUrl(item);
  return sideListConversationAssistantAvatarUrl(item) || sideListLastSpeakerAvatarUrl(item);
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
