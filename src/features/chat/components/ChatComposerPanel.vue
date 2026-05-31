<template>
  <div>
    <ChatQueuePreview
      v-if="!sidebarMode"
      :queue-events="visibleQueueEvents"
      :session-state="sessionState"
      @recall-to-input="handleRecallToInput"
      @mark-guided="markGuided"
    />

    <div
      v-if="linkOpenErrorText"
      class="alert alert-warning mb-2 py-2 px-3 text-sm whitespace-pre-wrap break-all max-h-24 overflow-auto"
    >
      <span>{{ linkOpenErrorText }}</span>
    </div>
    <ChatSelectionActionPanel
      v-if="selectionModeEnabled"
      :sidebar-mode="sidebarMode"
      :selected-message-count="selectedMessageCount"
      :active-conversation-id="activeConversationId"
      :unarchived-conversation-items="unarchivedConversationItems"
      :create-conversation-department-options="createConversationDepartmentOptions"
      :delegate-department-ids="delegateDepartmentIds"
      @exit-selection-mode="emit('exitSelectionMode')"
      @selection-action-branch="emit('selectionActionBranch')"
      @selection-action-forward="emit('selectionActionForward', $event)"
      @selection-action-delegate="emit('selectionActionDelegate', $event)"
      @selection-action-copy="emit('selectionActionCopy')"
      @selection-action-share="emit('selectionActionShare', $event)"
    />
    <template v-else>
    <div v-if="clipboardImages.length > 0 || queuedAttachmentNotices.length > 0" class="mb-2 flex flex-wrap gap-1">
      <div v-for="(img, idx) in clipboardImages" :key="`${img.mime}-${idx}`" class="badge badge-ghost gap-1 py-3">
        <ImageIcon v-if="isImageMime(img.mime)" class="h-3.5 w-3.5" />
        <FileText v-else-if="isPdfMime(img.mime)" class="h-3.5 w-3.5" />
        <ImageIcon v-else class="h-3.5 w-3.5" />
        <span class="text-[11px]">{{ isPdfMime(img.mime) ? `PDF ${idx + 1}` : t("chat.image", { index: idx + 1 }) }}</span>
        <button class="btn btn-ghost btn-sm btn-square" :disabled="chatting || frozen" @click="emit('removeClipboardImage', idx)">
          <X class="h-3 w-3" />
        </button>
      </div>
      <div
        v-for="(file, idx) in queuedAttachmentNotices"
        :key="file.id"
        class="badge badge-ghost gap-1 py-3"
      >
        <FileText class="h-3.5 w-3.5" />
        <span class="text-[11px]">{{ file.fileName }}</span>
        <button class="btn btn-ghost btn-sm btn-square" :disabled="chatting || frozen" @click="emit('removeQueuedAttachmentNotice', idx)">
          <X class="h-3 w-3" />
        </button>
      </div>
    </div>
    <div v-if="transcribing" class="mb-1 text-[11px] opacity-80 flex items-center gap-1">
      <span class="loading loading-spinner loading-sm"></span>
      <span>{{ t("chat.transcribing") }}</span>
    </div>
    <div v-if="selectedMentions.length > 0" class="mb-2 flex flex-wrap gap-1">
      <span
        v-for="item in selectedMentions"
        :key="`${item.agentId}:${item.departmentId}`"
        class="badge gap-1 bg-base-300 px-3 py-3 text-sm text-base-content border-transparent"
      >
        <span class="max-w-40 truncate leading-none">@{{ mentionDisplayLabel(item) }}</span>
        <button
          type="button"
          class="ml-0.5 inline-flex h-5 w-5 items-center justify-center rounded-full text-base-content transition hover:bg-error hover:text-error-content"
          :disabled="chatting || frozen"
          @click.stop="removeSelectedMention(item)"
        >
          <X class="h-3 w-3" />
        </button>
      </span>
    </div>
    <div v-if="selectedInstructionPrompts.length > 0" class="mb-2 flex flex-wrap gap-1">
      <div
        v-for="item in selectedInstructionPrompts"
        :key="item.id"
        class="badge badge-outline gap-1 py-3"
      >
        <Layers2 class="h-3.5 w-3.5" />
        <span class="max-w-48 truncate text-[11px]" :title="item.prompt">{{ item.prompt }}</span>
        <button class="btn btn-ghost btn-sm btn-square" :disabled="chatting || frozen" @click="removeSelectedInstructionPreset(item.id)">
          <X class="h-3 w-3" />
        </button>
      </div>
    </div>
    <div v-if="attachedIdeContextReferences.length > 0 || mergedIdeContextGroups.length > 0" class="mb-2 flex flex-col gap-2">
      <div v-for="group in mergedIdeContextGroups" :key="group.workspacePath" class="flex flex-col gap-1">
        <div v-if="showIdeWorkspaceGroupLabel" class="px-1 text-[11px] opacity-60">{{ group.workspaceName }}</div>
        <div class="flex flex-wrap gap-1">
          <button
            v-for="item in group.references"
            :key="item.id"
            type="button"
            class="gap-1 py-3 max-w-full"
            :class="isIdeContextAttached(item.id) ? 'badge badge-primary' : 'badge badge-ghost'"
            :disabled="chatting || frozen"
            :title="ideContextReferenceTitle(item)"
            @mousedown.prevent
            @click="toggleIdeContextReference(item)"
          >
            <Minus v-if="isIdeContextAttached(item.id)" class="h-3.5 w-3.5" />
            <Plus v-else class="h-3.5 w-3.5" />
            <span class="max-w-72 truncate text-[11px]">{{ item.displayLabel }}</span>
          </button>
        </div>
      </div>
    </div>
    <div ref="composerRootRef" class="flex flex-col">
      <div v-if="instructionPanelOpen" class="flex flex-wrap content-start gap-2 max-h-48 overflow-y-auto">
        <button
          v-for="(item, index) in normalizedInstructionPresets"
          :key="item.id"
          type="button"
          class="btn btn-sm min-h-0 max-w-full justify-start normal-case px-3"
          :class="instructionFocusIndex === index ? 'btn-primary' : 'btn-ghost'"
          :title="item.prompt"
          @click="applyInstructionPreset(item)"
        >
          <span class="block max-w-64 truncate text-left text-sm sm:max-w-80">{{ item.prompt }}</span>
        </button>
        <div v-if="normalizedInstructionPresets.length === 0" class="w-full px-2 py-3 text-sm opacity-60">
          {{ t("chat.noInstructionPresets") }}
        </div>
      </div>
      <div class="relative">
        <textarea
          ref="chatInputRef"
          v-model="localChatInput"
          class="w-full textarea resize-none overflow-y-auto chat-input-no-focus min-h-8"
          rows="1"
          :disabled="frozen"
          :placeholder="chatInputPlaceholder"
          @input="handleChatInputInput"
          @keydown="handleChatInputKeydown"
        ></textarea>
        <FloatingScrollbar v-if="chatInputRef" :target="chatInputRef" />
      </div>
      <Teleport to="body">
        <div
          v-if="mentionPanelOpen"
          class="fixed z-1200"
          :style="mentionPanelStyle"
        >
          <div class="dropdown-content mt-2 w-max max-w-[min(80vw,20rem)] overflow-hidden rounded-box border border-base-300 bg-base-100 p-1 shadow-xl">
            <ul class="flex flex-col gap-1">
              <li
                v-for="(item, index) in filteredMentionOptions"
                :key="`${item.agentId}:${item.departmentId}`"
              >
                <button
                  type="button"
                  class="flex min-h-0 w-full items-start gap-2 rounded-xl px-2 py-1.5 text-left text-base-content transition-colors"
                  :class="[
                    mentionFocusIndex === index ? 'bg-base-200' : '',
                    item.mentionable ? 'hover:bg-base-200/80' : 'opacity-65',
                  ]"
                  :disabled="!item.mentionable"
                  @click="applyMention(item)"
                >
                  <div class="indicator shrink-0">
                    <span
                      v-if="isMentionSelected(item)"
                      class="indicator-item inline-flex h-4 w-4 items-center justify-center rounded-full bg-primary text-[9px] font-bold text-primary-content"
                    >
                      @
                    </span>
                    <div class="avatar">
                      <div class="w-7 rounded-full">
                        <img
                          v-if="item.avatarUrl"
                          :src="item.avatarUrl"
                          :alt="item.agentName"
                          class="w-7 h-7 rounded-full object-cover"
                        />
                        <div v-else class="bg-neutral text-neutral-content w-7 h-7 rounded-full flex items-center justify-center text-[10px]">
                          {{ avatarInitial(item.agentName) }}
                        </div>
                      </div>
                    </div>
                  </div>
                  <div class="min-w-0 flex-1 pr-0.5">
                    <div class="truncate text-sm leading-5">@{{ mentionDisplayLabel(item) }}</div>
                    <div
                      v-if="!item.mentionable && item.unavailableReason"
                      class="truncate text-[11px] leading-4 text-base-content/60"
                    >
                      {{ item.unavailableReason }}
                    </div>
                  </div>
                </button>
              </li>
            </ul>
            <div v-if="filteredMentionOptions.length === 0" class="px-2.5 py-2 text-sm opacity-60">
              {{ t("chat.noMentionCandidates") }}
            </div>
          </div>
        </div>
      </Teleport>
      <div class="mt-2 flex items-center justify-between gap-2">
        <div class="flex items-center gap-2">
          <button
            class="btn btn-sm btn-circle shrink-0"
            :class="supervisionActive ? 'btn-primary' : 'btn-ghost'"
            :disabled="frozen || supervisionDisabled"
            :title="supervisionTitle || t('chat.supervision.buttonTitle')"
            @click="emit('openSupervisionTask')"
          >
            <Target class="h-3.5 w-3.5" />
          </button>
          <button
            v-if="!sidebarMode"
            class="btn btn-sm btn-circle btn-ghost shrink-0"
            :disabled="chatting || frozen"
            :title="t('chat.command')"
            @click="toggleInstructionPanel"
          >
            <Layers2 class="h-3.5 w-3.5" />
          </button>
          <button
            v-if="!sidebarMode"
            class="btn btn-sm btn-circle btn-ghost shrink-0"
            :disabled="chatting || frozen"
            :title="t('chat.attach')"
            @click="emit('pickAttachments')"
          >
            <Paperclip class="h-3.5 w-3.5" />
          </button>
          <button
            v-if="!sidebarMode"
            class="btn btn-sm btn-circle shrink-0"
            :class="recording ? 'btn-error' : 'btn-ghost'"
            :disabled="!canRecord || chatting || frozen"
            :title="recording ? t('chat.recording', { seconds: Math.max(1, Math.round(recordingMs / 1000)) }) : t('chat.holdRecord', { hotkey: recordHotkey })"
            @mousedown.prevent="emit('startRecording')"
            @mouseup.prevent="emit('stopRecording')"
            @mouseleave.prevent="recording && emit('stopRecording')"
            @touchstart.prevent="emit('startRecording')"
            @touchend.prevent="emit('stopRecording')"
          >
            <Mic class="h-3.5 w-3.5" />
          </button>
          <div v-if="normalizedChatModelOptions.length > 0" ref="modelDropdownRef" class="relative">
            <button
              type="button"
              :class="compactModelButton
                ? 'btn btn-sm btn-square h-8 min-h-8 w-8 shrink-0 border-0 shadow-none bg-base-100 text-base-content hover:bg-base-200'
                : 'btn btn-sm h-8 min-h-8 w-44 max-w-44 justify-between border-0 shadow-none bg-base-100 text-base-content hover:bg-base-200'"
              :disabled="chatting || frozen || normalizedChatModelOptions.length === 0"
              :title="selectedModelName"
              @click="modelDropdownOpen = !modelDropdownOpen"
            >
              <template v-if="compactModelButton">
                <Bot class="h-3.5 w-3.5 shrink-0" />
              </template>
              <template v-else>
                <span class="truncate">{{ selectedModelName }}</span>
                <ChevronDown class="h-3 w-3 shrink-0 opacity-50 rotate-180" :class="{ 'rotate-0': modelDropdownOpen }" />
              </template>
            </button>
            <ul
              v-if="modelDropdownOpen"
              class="absolute bottom-full left-0 z-9999 mb-2 w-80 max-h-[80vh] overflow-y-auto rounded-box border border-base-300 bg-base-100 p-2 shadow-xl"
            >
              <li v-for="item in normalizedChatModelOptions" :key="item.id" class="list-none">
                <button
                  type="button"
                  class="flex w-full items-center rounded-lg px-3 py-2 text-left text-sm hover:bg-base-200 transition-colors truncate"
                  :class="{ 'bg-primary/10': item.id === selectedChatModelId }"
                  @click="selectChatModel(item.id)"
                >
                  {{ item.name }}
                </button>
              </li>
            </ul>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <span
            v-if="planModeEnabled"
            class="badge badge-sm badge-info shrink-0 select-none"
            :title="`Shift+Tab ${t('chat.plan.mode')}`"
          >
            {{ t("chat.plan.mode") }}
          </span>
          <button
            class="btn btn-sm btn-circle shrink-0"
            :class="showStopAction ? 'btn-error' : 'btn-success'"
            :disabled="frozen || busy || (showStopAction && !!stopChatDisabled)"
            :title="showStopAction ? `${t('chat.stop')} / ${t('chat.stopReplying')}` : t('chat.send')"
            @click="showStopAction ? emit('stopChat') : handleSendChat()"
          >
            <Square v-if="showStopAction" class="h-3.5 w-3.5 fill-current" />
            <Send v-else class="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
    </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { Bot, ChevronDown, FileText, History, Image as ImageIcon, Layers2, Menu, Mic, Minus, Paperclip, Plus, Send, Settings, Square, Target, X } from "@lucide/vue";
import type { ApiConfigItem, ChatConversationOverviewItem, ChatMentionEntry, ChatMentionTarget, IdeContextReferenceItem, IdeContextWorkspaceGroup, PromptCommandPreset } from "../../../types/app";
import { invokeTauri } from "../../../services/tauri-api";
import ChatQueuePreview from "./ChatQueuePreview.vue";
import ChatSelectionActionPanel from "./ChatSelectionActionPanel.vue";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";
import { useChatQueue } from "../composables/use-chat-queue";

type BinaryAttachment = { mime: string; bytesBase64: string };
type QueuedAttachmentNotice = { id: string; fileName: string; relativePath: string; mime: string };
type ConversationDepartmentOption = {
  id: string;
  name: string;
  ownerAgentId?: string;
  ownerName: string;
  providerName?: string;
  modelName?: string;
};
type MentionOptionView = {
  agentId: string;
  agentName: string;
  departmentId: string;
  departmentName: string;
  avatarUrl?: string;
  mentionable: boolean;
  unavailableReason?: string;
};

const props = defineProps<{
  selectionModeEnabled: boolean;
  selectedMessageCount: number;
  chatInput: string;
  instructionPresets: PromptCommandPreset[];
  mentionEntries: ChatMentionEntry[];
  selectedMentions: ChatMentionTarget[];
  chatInputPlaceholder: string;
  clipboardImages: BinaryAttachment[];
  queuedAttachmentNotices: QueuedAttachmentNotice[];
  linkOpenErrorText: string;
  chatErrorText: string;
  transcribing: boolean;
  canRecord: boolean;
  recording: boolean;
  recordingMs: number;
  recordHotkey: string;
  selectedChatModelId: string;
  chatModelOptions: ApiConfigItem[];
  workspaceAccess?: "read_only" | "approval" | "full_access" | "";
  planModeEnabled: boolean;
  chatting: boolean;
  frontendRoundPhase?: "idle" | "queued" | "waiting" | "streaming";
  busy: boolean;
  stopChatDisabled?: boolean;
  frozen: boolean;
  supervisionActive: boolean;
  supervisionTitle: string;
  supervisionDisabled?: boolean;
  showSideConversationList: boolean;
  activeConversationId: string;
  unarchivedConversationItems: ChatConversationOverviewItem[];
  userAlias: string;
  userAvatarUrl: string;
  personaNameMap: Record<string, string>;
  personaAvatarUrlMap: Record<string, string>;
  createConversationDepartmentOptions: ConversationDepartmentOption[];
  delegateDepartmentIds: string[];
  defaultCreateConversationDepartmentId: string;
  ideContextGroups: IdeContextWorkspaceGroup[];
  attachedIdeContextReferences: IdeContextReferenceItem[];
  sidebarMode?: boolean;
  trimTip?: string;
  chatUsagePercent?: number;
}>();

const emit = defineEmits<{
  (e: "exitSelectionMode"): void;
  (e: "selectionActionBranch"): void;
  (e: "selectionActionForward", targetConversationId: string): void;
  (e: "selectionActionDelegate", payload: { departmentId: string; presetId: string; background: string; question: string; focus: string }): void;
  (e: "selectionActionCopy"): void;
  (e: "selectionActionShare", format: "html" | "png"): void;
  (e: "update:chatInput", value: string): void;
  (e: "update:selectedInstructionPrompts", value: PromptCommandPreset[]): void;
  (e: "addMention", value: ChatMentionTarget): void;
  (e: "removeMention", value: string | { agentId: string; departmentId?: string }): void;
  (e: "removeClipboardImage", index: number): void;
  (e: "removeQueuedAttachmentNotice", index: number): void;
  (e: "startRecording"): void;
  (e: "stopRecording"): void;
  (e: "pickAttachments"): void;
  (e: "update:selectedChatModelId", value: string): void;
  (e: "update:workspaceAccess", value: "read_only" | "approval" | "full_access"): void;
  (e: "update:planModeEnabled", value: boolean): void;
  (e: "attachIdeContextReference", value: IdeContextReferenceItem): void;
  (e: "removeIdeContextReference", value: string): void;
  (e: "sendChat"): void;
  (e: "stopChat"): void;
  (e: "openSupervisionTask"): void;
  (e: "open-conversation-list"): void;
  (e: "open-settings"): void;
  (e: "trim-conversation"): void;
}>();

const { t } = useI18n();
const sidebarMode = computed(() => !!props.sidebarMode);

const menuOpen = ref(false);
const menuTriggerRef = ref<HTMLButtonElement | null>(null);
const menuWrapperRef = ref<HTMLDivElement | null>(null);

function closeMenu() {
  menuOpen.value = false;
}

function handleOpenHistory() {
  closeMenu();
  emit('open-conversation-list');
}

function handleOpenConfig() {
  closeMenu();
  emit('open-settings');
}

function onMenuOutsideClick(event: MouseEvent) {
  if (!menuOpen.value) return;
  const target = event.target as Node | null;
  if (menuWrapperRef.value && menuWrapperRef.value.contains(target)) return;
  closeMenu();
}

onMounted(() => { document.addEventListener('pointerdown', onMenuOutsideClick); });
onBeforeUnmount(() => { document.removeEventListener('pointerdown', onMenuOutsideClick); });

const { queueEvents, sessionState, recallQueueEvent, markGuided } = useChatQueue({
  enabled: computed(() => !sidebarMode.value),
});

const visibleQueueEvents = computed(() => {
  const activeConversationId = String(props.activeConversationId || "").trim();
  if (!activeConversationId) return [];
  return queueEvents.value.filter(
    (event) => String(event.conversationId || "").trim() === activeConversationId,
  );
});

const localChatInput = computed({
  get: () => props.chatInput,
  set: (value: string) => emit("update:chatInput", value),
});
const CHAT_INPUT_HISTORY_STORAGE_KEY = "easy_call.chat_input_history.v1";
const CHAT_INPUT_HISTORY_LIMIT = 100;
const composerRootRef = ref<HTMLDivElement | null>(null);
const chatInputRef = ref<HTMLTextAreaElement | null>(null);
const composerWidth = ref(0);
const chatInputHistory = ref<string[]>([]);
const chatInputHistoryCursor = ref(-1);
const chatInputHistoryDraft = ref("");
const chatInputHistoryApplying = ref(false);
const resizeInputRaf = ref(0);
const instructionPanelOpen = ref(false);
const instructionFocusIndex = ref(0);
const selectedInstructionPrompts = ref<PromptCommandPreset[]>([]);
const mentionPanelOpen = ref(false);
const mentionQuery = ref("");
const mentionFocusIndex = ref(0);
const mentionRange = ref<{ start: number; end: number } | null>(null);
const mentionPanelStyle = ref<Record<string, string>>({
  left: "0px",
  top: "0px",
  transform: "translateY(calc(-100% - 8px))",
});

const normalizedInstructionPresets = computed(() =>
  (Array.isArray(props.instructionPresets) ? props.instructionPresets : [])
    .map((item) => ({
      id: String(item?.id || "").trim(),
      name: String(item?.prompt || item?.name || "").trim(),
      prompt: String(item?.prompt || item?.name || "").trim(),
    }))
    .filter((item) => !!item.id && !!item.prompt),
);
const normalizedChatModelOptions = computed(() =>
  (Array.isArray(props.chatModelOptions) ? props.chatModelOptions : [])
    .map((item) => ({
      id: String(item?.id || "").trim(),
      name: String(item?.name || "").trim(),
    }))
    .filter((item) => !!item.id && !!item.name),
);
const selectedModelName = computed(() => {
  const found = normalizedChatModelOptions.value.find((item) => item.id === props.selectedChatModelId);
  return found?.name || props.selectedChatModelId;
});
const compactModelButton = computed(() => composerWidth.value > 0 && composerWidth.value < 420);
const showIdeWorkspaceGroupLabel = computed(() => false);
const attachedIdeContextReferenceIds = computed(() => new Set((props.attachedIdeContextReferences || []).map((item) => item.id)));
const mergedIdeContextGroups = computed<IdeContextWorkspaceGroup[]>(() => {
  const referencesByIdentity = new Map<string, IdeContextReferenceItem>();
  const attachedMap = new Map((props.attachedIdeContextReferences || []).map((item) => [item.id, item]));
  const attachedReferences = Array.isArray(props.attachedIdeContextReferences) ? props.attachedIdeContextReferences : [];
  for (const group of props.ideContextGroups || []) {
    for (const item of group.references || []) {
      const identity = ideContextReferenceIdentityKey(item);
      if (!identity) continue;
      if (attachedReferences.some((attached) => ideContextSameRange(attached, item))) continue;
      referencesByIdentity.set(identity, item);
    }
  }
  for (const item of props.attachedIdeContextReferences || []) {
    const identity = ideContextReferenceIdentityKey(item);
    if (!identity) continue;
    referencesByIdentity.set(identity, item);
  }
  const references = Array.from(referencesByIdentity.values()).sort((left, right) => {
    const leftAttached = attachedMap.has(left.id) ? 1 : 0;
    const rightAttached = attachedMap.has(right.id) ? 1 : 0;
    if (leftAttached !== rightAttached) return rightAttached - leftAttached;
    return String(left.displayLabel || "").localeCompare(String(right.displayLabel || ""));
  });
  return references.length > 0 ? [{ workspacePath: "", workspaceName: "", references }] : [];
});

function ideContextReferencePathKey(item: IdeContextReferenceItem): string {
  return String(item.filePath || item.relativePath || item.displayLabel || item.id || "").trim().replace(/\\/g, "/").toLowerCase();
}

function ideContextReferenceIdentityKey(item: IdeContextReferenceItem): string {
  const path = ideContextReferencePathKey(item);
  if (!path) return "";
  return [
    path,
    Number(item.startLine || 0),
    Number(item.endLine || 0),
  ].join(":");
}

function ideContextSameRange(left: IdeContextReferenceItem, right: IdeContextReferenceItem): boolean {
  return ideContextReferencePathKey(left) === ideContextReferencePathKey(right)
    && Number(left.startLine || 0) === Number(right.startLine || 0)
    && Number(left.endLine || 0) === Number(right.endLine || 0);
}

function isIdeContextAttached(referenceId: string): boolean {
  return attachedIdeContextReferenceIds.value.has(referenceId);
}

function toggleIdeContextReference(item: IdeContextReferenceItem) {
  if (isIdeContextAttached(item.id)) {
    emit("removeIdeContextReference", item.id);
  } else {
    emit("attachIdeContextReference", item);
  }
  void nextTick(() => focusInput({ preventScroll: true }));
}

function ideContextReferenceTitle(item: IdeContextReferenceItem): string {
  const relativePath = String(item.relativePath || "").trim();
  const startLine = Number(item.startLine || 0);
  const endLine = Number(item.endLine || 0);
  if (!relativePath) return String(item.displayLabel || "").trim();
  if (startLine > 0 && endLine > startLine) {
    return `${relativePath}:${startLine}-${endLine}`;
  }
  if (startLine > 0) {
    return `${relativePath}:${startLine}`;
  }
  return relativePath;
}

const showStopAction = computed(() =>
  props.chatting || ["queued", "waiting", "streaming"].includes(String(props.frontendRoundPhase || "idle")),
);
const selectedMentions = computed(() =>
  (Array.isArray(props.selectedMentions) ? props.selectedMentions : [])
    .map((item) => ({
      agentId: String(item?.agentId || "").trim(),
      agentName: String(item?.agentName || "").trim(),
      departmentId: String(item?.departmentId || "").trim(),
      departmentName: String(item?.departmentName || "").trim(),
      avatarUrl: String(item?.avatarUrl || "").trim() || undefined,
    }))
    .filter((item) => !!item.agentId && !!item.departmentId && !!item.agentName),
);
const filteredMentionOptions = computed<MentionOptionView[]>(() => {
  return (Array.isArray(props.mentionEntries) ? props.mentionEntries : [])
    .map((item) => ({
      agentId: String(item?.agentId || "").trim(),
      agentName: String(item?.agentName || "").trim(),
      departmentId: String(item?.departmentId || "").trim(),
      departmentName: String(item?.departmentName || "").trim(),
      avatarUrl: String(item?.avatarUrl || "").trim() || undefined,
      mentionable: !!item?.mentionable,
      unavailableReason: String(item?.unavailableReason || "").trim() || undefined,
    }))
    .filter((item) => !!item.agentId && !!item.agentName && !!item.mentionable);
});

const planModeToggleAllowed = computed(() => !props.chatting && !props.frozen);

function loadChatInputHistory() {
  try {
    const raw = window.localStorage.getItem(CHAT_INPUT_HISTORY_STORAGE_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return;
    const normalized: string[] = [];
    const seen = new Set<string>();
    for (const item of parsed) {
      const text = String(item || "").trim();
      if (!text || seen.has(text)) continue;
      seen.add(text);
      normalized.push(text);
      if (normalized.length >= CHAT_INPUT_HISTORY_LIMIT) break;
    }
    chatInputHistory.value = normalized;
  } catch {
    chatInputHistory.value = [];
  }
}

function saveChatInputHistory() {
  try {
    window.localStorage.setItem(CHAT_INPUT_HISTORY_STORAGE_KEY, JSON.stringify(chatInputHistory.value));
  } catch {
    // ignore persistence failures
  }
}

function pushChatInputHistory(rawText: string) {
  const text = String(rawText || "").trim();
  if (!text) return;
  chatInputHistory.value = [text, ...chatInputHistory.value.filter((item) => item !== text)].slice(0, CHAT_INPUT_HISTORY_LIMIT);
  saveChatInputHistory();
  chatInputHistoryCursor.value = -1;
  chatInputHistoryDraft.value = "";
}

function emitSelectedInstructionPrompts() {
  emit("update:selectedInstructionPrompts", selectedInstructionPrompts.value);
}

function openInstructionPanel() {
  instructionPanelOpen.value = true;
  if (instructionFocusIndex.value >= normalizedInstructionPresets.value.length) {
    instructionFocusIndex.value = Math.max(0, normalizedInstructionPresets.value.length - 1);
  }
}

function closeInstructionPanel() {
  instructionPanelOpen.value = false;
}

function closeMentionPanel() {
  mentionPanelOpen.value = false;
  mentionQuery.value = "";
  mentionFocusIndex.value = 0;
  mentionRange.value = null;
}

function refreshMentionPanelPosition() {
  const el = chatInputRef.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  mentionPanelStyle.value = {
    left: `${Math.round(rect.left)}px`,
    top: `${Math.round(rect.top)}px`,
    transform: "translateY(calc(-100% - 8px))",
  };
}

function toggleInstructionPanel() {
  if (instructionPanelOpen.value) {
    closeInstructionPanel();
    return;
  }
  openInstructionPanel();
}

function applyInstructionPreset(item: PromptCommandPreset | undefined) {
  if (!item) return;
  if (!selectedInstructionPrompts.value.some((entry) => entry.id === item.id)) {
    selectedInstructionPrompts.value = [...selectedInstructionPrompts.value, item];
    emitSelectedInstructionPrompts();
  }
  closeInstructionPanel();
}

function selectInstructionPresetByIndex(index: number) {
  const list = normalizedInstructionPresets.value;
  if (list.length === 0) return;
  const nextIndex = Math.max(0, Math.min(list.length - 1, index));
  instructionFocusIndex.value = nextIndex;
  applyInstructionPreset(list[nextIndex]);
}

function moveInstructionFocus(delta: number) {
  const list = normalizedInstructionPresets.value;
  if (list.length === 0) return;
  const next = instructionFocusIndex.value + delta;
  instructionFocusIndex.value = Math.max(0, Math.min(list.length - 1, next));
}

function removeSelectedInstructionPreset(id: string) {
  selectedInstructionPrompts.value = selectedInstructionPrompts.value.filter((item) => item.id !== id);
  emitSelectedInstructionPrompts();
}

function clearSelectedInstructionPrompts() {
  if (selectedInstructionPrompts.value.length === 0) return;
  selectedInstructionPrompts.value = [];
  emitSelectedInstructionPrompts();
}

function removeSelectedMention(item: ChatMentionTarget | undefined) {
  if (!item) return;
  emit("removeMention", {
    agentId: String(item.agentId || "").trim(),
    departmentId: String(item.departmentId || "").trim() || undefined,
  });
  closeMentionPanel();
}

function applyMention(item: MentionOptionView | undefined) {
  if (!item || !item.mentionable || !mentionRange.value) return;
  const current = String(localChatInput.value || "");
  const before = current.slice(0, mentionRange.value.start);
  const after = current.slice(mentionRange.value.end);
  const nextValue = `${before}${after}`;
  localChatInput.value = nextValue;
  if (selectedMentions.value.some((entry) =>
    String(entry.agentId || "").trim() === String(item.agentId || "").trim()
    && String(entry.departmentId || "").trim() === String(item.departmentId || "").trim()
  )) {
    emit("removeMention", {
      agentId: String(item.agentId || "").trim(),
      departmentId: String(item.departmentId || "").trim() || undefined,
    });
  } else {
    emit("addMention", {
      agentId: String(item.agentId || "").trim(),
      agentName: String(item.agentName || "").trim(),
      departmentId: String(item.departmentId || "").trim(),
      departmentName: String(item.departmentName || "").trim(),
      avatarUrl: String(item.avatarUrl || "").trim() || undefined,
    });
  }
  closeMentionPanel();
  nextTick(() => {
    const el = chatInputRef.value;
    if (!el) return;
    const cursor = Math.min(before.length, nextValue.length);
    el.focus();
    el.setSelectionRange(cursor, cursor);
    scheduleResizeChatInput();
  });
}

function selectMentionByIndex(index: number) {
  const list = filteredMentionOptions.value;
  if (list.length === 0) return;
  const nextIndex = Math.max(0, Math.min(list.length - 1, index));
  mentionFocusIndex.value = nextIndex;
  applyMention(list[nextIndex]);
}

function moveMentionFocus(delta: number) {
  const list = filteredMentionOptions.value;
  if (list.length === 0) return;
  const next = mentionFocusIndex.value + delta;
  mentionFocusIndex.value = Math.max(0, Math.min(list.length - 1, next));
}

function updateMentionState() {
  const el = chatInputRef.value;
  if (!el || el.selectionStart !== el.selectionEnd) {
    closeMentionPanel();
    return;
  }
  const value = String(localChatInput.value || "");
  const cursor = el.selectionStart ?? value.length;
  const beforeCursor = value.slice(0, cursor);
  const match = beforeCursor.match(/(?:^|\s)@$/);
  if (!match) {
    closeMentionPanel();
    return;
  }
  mentionQuery.value = "";
  const queryStart = cursor - 1;
  mentionRange.value = { start: queryStart, end: cursor };
  refreshMentionPanelPosition();
  mentionPanelOpen.value = true;
  if (mentionFocusIndex.value >= filteredMentionOptions.value.length) {
    mentionFocusIndex.value = 0;
  }
}

const modelDropdownOpen = ref(false);
const modelDropdownRef = ref<HTMLElement | null>(null);
let composerWidthObserver: ResizeObserver | null = null;

function refreshComposerWidth() {
  const el = composerRootRef.value;
  composerWidth.value = el ? Math.round(el.getBoundingClientRect().width) : 0;
}

function handleModelDropdownClickOutside(event: MouseEvent) {
  if (
    modelDropdownRef.value &&
    !modelDropdownRef.value.contains(event.target as Node)
  ) {
    modelDropdownOpen.value = false;
  }
}

watch(modelDropdownOpen, (open) => {
  if (open) {
    nextTick(() => {
      document.addEventListener("click", handleModelDropdownClickOutside);
    });
  } else {
    document.removeEventListener("click", handleModelDropdownClickOutside);
  }
});

watch(compactModelButton, (compact) => {
  if (compact) {
    modelDropdownOpen.value = false;
  }
});

function selectChatModel(id: string) {
  if (!id || id === props.selectedChatModelId) return;
  modelDropdownOpen.value = false;
  emit("update:selectedChatModelId", id);
}

function togglePlanMode() {
  if (!planModeToggleAllowed.value) return;
  emit("update:planModeEnabled", !props.planModeEnabled);
}

function resizeChatInput() {
  const el = chatInputRef.value;
  if (!el) return;
  const minHeight = 48;
  const maxHeight = 160;
  el.style.height = "auto";
  const nextHeight = Math.max(Math.min(el.scrollHeight, maxHeight), minHeight);
  el.style.height = `${nextHeight}px`;
  el.style.overflowY = "auto";
}

function handleChatInputInput() {
  scheduleResizeChatInput();
  updateMentionState();
}

function scheduleResizeChatInput() {
  if (resizeInputRaf.value) cancelAnimationFrame(resizeInputRaf.value);
  resizeInputRaf.value = requestAnimationFrame(() => {
    resizeChatInput();
    resizeInputRaf.value = 0;
  });
}

function applyChatInputHistoryValue(value: string) {
  chatInputHistoryApplying.value = true;
  localChatInput.value = value;
  nextTick(() => {
    chatInputHistoryApplying.value = false;
    scheduleResizeChatInput();
    const el = chatInputRef.value;
    if (!el) return;
    const cursor = value.length;
    el.setSelectionRange(cursor, cursor);
  });
}

function canNavigateHistory(el: HTMLTextAreaElement, direction: "up" | "down"): boolean {
  if (el.selectionStart !== el.selectionEnd) return false;
  if (direction === "up") return el.selectionStart === 0;
  return el.selectionStart === el.value.length;
}

function navigateChatInputHistory(direction: "up" | "down"): boolean {
  const list = chatInputHistory.value;
  if (list.length === 0) return false;
  if (direction === "up") {
    if (chatInputHistoryCursor.value === -1) {
      chatInputHistoryDraft.value = localChatInput.value;
      chatInputHistoryCursor.value = 0;
      applyChatInputHistoryValue(list[0]);
      return true;
    }
    if (chatInputHistoryCursor.value < list.length - 1) {
      chatInputHistoryCursor.value += 1;
      applyChatInputHistoryValue(list[chatInputHistoryCursor.value]);
      return true;
    }
    return false;
  }
  if (chatInputHistoryCursor.value === -1) return false;
  if (chatInputHistoryCursor.value === 0) {
    chatInputHistoryCursor.value = -1;
    const draft = chatInputHistoryDraft.value;
    chatInputHistoryDraft.value = "";
    applyChatInputHistoryValue(draft);
    return true;
  }
  chatInputHistoryCursor.value -= 1;
  applyChatInputHistoryValue(list[chatInputHistoryCursor.value]);
  return true;
}

function recordSentTextIfNeeded(rawText: string) {
  const text = String(rawText || "").trim();
  if (!text) return;
  setTimeout(() => {
    if (String(props.chatInput || "").trim()) return;
    pushChatInputHistory(text);
  }, 0);
}

function handleSendChat() {
  const plainText = String(localChatInput.value || "").trim();
  emit("sendChat");
  recordSentTextIfNeeded(plainText);
  clearSelectedInstructionPrompts();
  closeInstructionPanel();
  closeMentionPanel();
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (event.defaultPrevented || event.isComposing || event.repeat) return;
  if (event.key !== "Tab" || !event.shiftKey || event.ctrlKey || event.altKey || event.metaKey) return;
  if (!planModeToggleAllowed.value) return;
  const activeElement = document.activeElement;
  const textareaFocused = !!chatInputRef.value && activeElement === chatInputRef.value;
  const composerFocused = !!composerRootRef.value && activeElement === composerRootRef.value;
  if (!textareaFocused && !composerFocused) return;
  event.preventDefault();
  togglePlanMode();
}

function handleChatInputKeydown(event: KeyboardEvent) {
  if (event.isComposing) return;
  if (mentionPanelOpen.value) {
    if (event.key === "Escape") {
      event.preventDefault();
      closeMentionPanel();
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      moveMentionFocus(-1);
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      moveMentionFocus(1);
      return;
    }
    if (event.key === "Enter" && !event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey) {
      event.preventDefault();
      selectMentionByIndex(mentionFocusIndex.value);
      return;
    }
  }
  if (event.key === "Tab" && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey) {
    event.preventDefault();
    toggleInstructionPanel();
    return;
  }
  if (instructionPanelOpen.value) {
    if (event.key === "Escape") {
      event.preventDefault();
      closeInstructionPanel();
      return;
    }
    if (event.key === "ArrowUp" || event.key === "ArrowLeft") {
      event.preventDefault();
      moveInstructionFocus(-1);
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowRight") {
      event.preventDefault();
      moveInstructionFocus(1);
      return;
    }
    if (event.key === "Enter" && !event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey) {
      event.preventDefault();
      selectInstructionPresetByIndex(instructionFocusIndex.value);
      return;
    }
  }
  if (event.key === "Enter" && !event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey) {
    if (props.frozen || props.busy) return;
    event.preventDefault();
    handleSendChat();
    return;
  }
  if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
  if (event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return;
  const el = chatInputRef.value;
  if (!el) return;
  const direction = event.key === "ArrowUp" ? "up" : "down";
  if (!canNavigateHistory(el, direction)) return;
  if (navigateChatInputHistory(direction)) {
    event.preventDefault();
  }
}

function isImageMime(mime: string): boolean {
  return (mime || "").trim().toLowerCase().startsWith("image/");
}

function isPdfMime(mime: string): boolean {
  return (mime || "").trim().toLowerCase() === "application/pdf";
}

function avatarInitial(name: string): string {
  const text = String(name || "").trim();
  if (!text) return "?";
  return text[0].toUpperCase();
}

function mentionDisplayLabel(target: Pick<ChatMentionTarget, "agentName" | "departmentName">): string {
  const agentName = String(target?.agentName || "").trim();
  const departmentName = String(target?.departmentName || "").trim();
  if (!departmentName) return agentName;
  return `${agentName} / ${departmentName}`;
}

function isMentionSelected(target: Pick<ChatMentionTarget, "agentId" | "departmentId"> | undefined): boolean {
  const agentId = String(target?.agentId || "").trim();
  const departmentId = String(target?.departmentId || "").trim();
  if (!agentId || !departmentId) return false;
  return selectedMentions.value.some((item) =>
    String(item.agentId || "").trim() === agentId
    && String(item.departmentId || "").trim() === departmentId
  );
}

async function handleRecallToInput(event: {
  source?: string;
  messagePreview?: string;
  messageText?: string;
  id?: string;
  queueMode?: "normal" | "guided";
}) {
  if (event.source === "user" && event.queueMode !== "guided") {
    if (event.id) {
      const result = await recallQueueEvent(event.id);
      if (result.removed) {
        localChatInput.value = result.messageText || event.messageText || event.messagePreview || "";
      }
    }
  }
}

function focusInput(options?: FocusOptions) {
  chatInputRef.value?.focus(options);
}

defineExpose({
  focusInput,
});

onMounted(() => {
  loadChatInputHistory();
  window.addEventListener("keydown", handleWindowKeydown);
  window.addEventListener("resize", refreshMentionPanelPosition);
  window.addEventListener("scroll", refreshMentionPanelPosition, true);
  refreshComposerWidth();
  if (typeof ResizeObserver !== "undefined" && composerRootRef.value) {
    composerWidthObserver = new ResizeObserver(() => refreshComposerWidth());
    composerWidthObserver.observe(composerRootRef.value);
  }
  nextTick(() => {
    resizeChatInput();
    refreshComposerWidth();
    refreshMentionPanelPosition();
  });
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", handleWindowKeydown);
  window.removeEventListener("resize", refreshMentionPanelPosition);
  window.removeEventListener("scroll", refreshMentionPanelPosition, true);
  if (resizeInputRaf.value) {
    cancelAnimationFrame(resizeInputRaf.value);
    resizeInputRaf.value = 0;
  }
  composerWidthObserver?.disconnect();
  composerWidthObserver = null;
});

watch(
  () => props.chatInput,
  (nextValue, prevValue) => {
    if (!chatInputHistoryApplying.value && nextValue !== prevValue && chatInputHistoryCursor.value !== -1) {
      chatInputHistoryCursor.value = -1;
      chatInputHistoryDraft.value = "";
    }
    nextTick(() => scheduleResizeChatInput());
    nextTick(() => {
      refreshMentionPanelPosition();
      updateMentionState();
    });
  },
);

watch(
  () => props.chatting,
  (isChatting, wasChatting) => {
    if (wasChatting && !isChatting && !props.frozen) {
      nextTick(() => focusInput({ preventScroll: true }));
    }
  },
);

watch(
  () => props.activeConversationId,
  () => {
    closeInstructionPanel();
    closeMentionPanel();
    clearSelectedInstructionPrompts();
    nextTick(() => scheduleResizeChatInput());
  },
);

watch(
  () => normalizedInstructionPresets.value,
  (list) => {
    if (list.length === 0) {
      instructionFocusIndex.value = 0;
      selectedInstructionPrompts.value = [];
      emitSelectedInstructionPrompts();
      instructionPanelOpen.value = false;
      return;
    }
    if (instructionFocusIndex.value >= list.length) {
      instructionFocusIndex.value = list.length - 1;
    }
    selectedInstructionPrompts.value = selectedInstructionPrompts.value.filter((item) =>
      list.some((entry) => entry.id === item.id),
    );
    emitSelectedInstructionPrompts();
  },
  { deep: true },
);

watch(
  () => props.selectedMentions.map((item) => `${item.agentId}:${item.departmentId}`).join("|"),
  () => {
    closeMentionPanel();
  },
);
</script>

<style scoped>
.chat-input-no-focus::-webkit-scrollbar {
  display: none;
}
.chat-input-no-focus {
  scrollbar-width: none;
}
</style>
