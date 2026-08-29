<template>
  <div>
    <ChatQueuePreview
      v-if="queueEnabled && !systemNotificationMode && !remoteContactMode"
      :queue-events="visibleQueueEvents"
      :session-state="sessionState"
      :user-persona-name="queueUserPersonaName"
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
      :show-conversation-actions="showConversationActions"
      :delegate-only="selectionDelegateOnly || systemNotificationMode || remoteContactMode"
      :selected-message-count="selectedMessageCount"
      :active-conversation-id="activeConversationId"
      :unarchived-conversation-items="unarchivedConversationItems"
      :remote-im-contact-conversations="remoteImContactConversations"
      :create-conversation-department-options="createConversationDepartmentOptions"
      :persona-avatar-url-map="personaAvatarUrlMap"
      :active-agent-id="activeAgentId"
      @exit-selection-mode="emit('exitSelectionMode')"
      @selection-action-branch="emit('selectionActionBranch')"
      @selection-action-forward="emit('selectionActionForward', $event)"
      @selection-action-delegate="emit('selectionActionDelegate', $event)"
      @selection-action-copy="emit('selectionActionCopy')"
      @selection-action-share="emit('selectionActionShare', $event)"
    />
    <template v-else>
    <div v-if="systemNotificationMode" class="flex flex-wrap items-center justify-center gap-2">
      <button
        type="button"
        class="btn btn-sm gap-2"
        :disabled="frozen || busy"
        @click="emit('openDelegateSelection')"
      >
        <ClipboardList class="h-3.5 w-3.5" />
        {{ t("chat.conversationMenu.startDelegate") }}
      </button>
      <button
        type="button"
        class="btn btn-sm gap-2"
        :disabled="frozen || busy"
        @click="emit('openTaskCreate')"
      >
        <CalendarPlus class="h-3.5 w-3.5" />
        {{ t("chat.newTask") }}
      </button>
      <button
        type="button"
        class="btn btn-sm gap-2"
        :disabled="frozen || busy"
        @click="openCreateConversationDialog"
      >
        <Plus class="h-3.5 w-3.5" />
        {{ t("chat.newConversation") }}
      </button>
    </div>
    <div v-else-if="remoteContactMode" class="flex flex-wrap items-center justify-center gap-2">
      <button
        type="button"
        class="btn btn-sm gap-2"
        :disabled="frozen || busy"
        @click="emit('openTaskCreate')"
      >
        <CalendarPlus class="h-3.5 w-3.5" />
        {{ t("chat.newTask") }}
      </button>
    </div>
    <template v-else>
    <div v-if="visibleQueuedAttachmentNotices.length > 0" class="mb-2 flex flex-wrap gap-1">
      <div
        v-for="(file, idx) in visibleQueuedAttachmentNotices"
        :key="file.id"
        class="badge badge-ghost gap-1 py-3"
      >
        <span v-if="file.pending" class="loading loading-spinner loading-xs"></span>
        <FileText v-else class="h-3.5 w-3.5" />
        <span class="text-xs">{{ file.fileName }}</span>
        <button
          v-if="!file.pending"
          class="btn btn-ghost btn-sm btn-square"
          @click="emit('removeQueuedAttachmentNotice', idx)"
        >
          <X class="h-3 w-3" />
        </button>
      </div>
    </div>
    <div v-if="transcribing" class="mb-1 text-xs opacity-80 flex items-center gap-1">
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
          @click.stop="removeSelectedMention(item)"
        >
          <X class="h-3 w-3" />
        </button>
      </span>
    </div>
    <div v-if="attachedIdeContextReferences.length > 0 || mergedIdeContextGroups.length > 0" class="mb-2 flex flex-col gap-2">
      <div v-for="group in mergedIdeContextGroups" :key="group.workspacePath" class="flex flex-col gap-1">
        <div v-if="showIdeWorkspaceGroupLabel" class="px-1 text-xs opacity-60">{{ group.workspaceName }}</div>
        <div class="flex flex-wrap gap-1">
          <button
            v-for="item in group.references"
            :key="item.id"
            type="button"
            class="gap-1 py-3 max-w-full"
            :class="isIdeContextAttached(item.id) ? 'badge badge-primary' : 'badge badge-ghost'"
            :title="ideContextReferenceTitle(item)"
            @mousedown.prevent
            @click="toggleIdeContextReference(item)"
          >
            <Minus v-if="isIdeContextAttached(item.id)" class="h-3.5 w-3.5 shrink-0" />
            <Plus v-else class="h-3.5 w-3.5 shrink-0" />
            <span class="flex min-w-0 max-w-72 items-center text-xs">
              <span class="min-w-0 truncate">{{ ideContextReferenceDisplayParts(item).fileName }}</span>
              <span
                v-if="ideContextReferenceDisplayParts(item).lineSuffix"
                class="shrink-0 whitespace-nowrap"
              >{{ ideContextReferenceDisplayParts(item).lineSuffix }}</span>
            </span>
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
      <div v-if="mobileViewport" class="flex flex-col gap-1.5">
        <div class="flex items-center gap-1.5">
          <button
            v-if="isMobileCompact && showConversationActions"
            type="button"
            class="btn btn-sm btn-circle btn-ghost shrink-0"
            :title="t('chat.attach')"
            @click="emit('pickAttachments')"
          >
            <Paperclip class="h-3.5 w-3.5" />
          </button>
          <div class="min-w-0 flex-1 p-1.5">
            <div v-if="clipboardImages.length > 0" class="ecall-chat-composer-image-previews mb-1.5">
              <div
                v-for="(img, idx) in clipboardImages"
                :key="`${img.mime}-${idx}`"
                class="ecall-chat-composer-image-preview"
              >
                <img
                  v-if="clipboardImagePreviewSrc(img)"
                  class="ecall-chat-composer-image-preview-media"
                  :src="clipboardImagePreviewSrc(img)"
                  :alt="t('chat.image', { index: idx + 1 })"
                  draggable="false"
                />
                <div v-else class="ecall-chat-composer-file-preview">
                  <FileText class="h-5 w-5" />
                  <span class="text-xs">{{ isPdfMime(img.mime) ? `PDF ${idx + 1}` : t("chat.image", { index: idx + 1 }) }}</span>
                </div>
                <button
                  type="button"
                  class="ecall-chat-composer-image-remove"
                  aria-label="删除图片"
                  @mousedown.prevent
                  @click.stop="removeClipboardImageAt(idx)"
                >
                  <X class="h-3 w-3" />
                </button>
              </div>
            </div>
            <textarea
              ref="chatInputRef"
              v-model="localChatInput"
              class="block w-full resize-none overflow-y-auto bg-transparent text-sm leading-6 outline-none chat-input-no-focus"
              rows="1"
              :placeholder="effectiveChatInputPlaceholder"
              @input="handleChatInputInput"
              @compositionstart="handleChatInputCompositionStart"
              @compositionend="handleChatInputCompositionEnd"
              @keydown="handleChatInputKeydown"
              @focus="handleChatInputFocus"
              @blur="handleChatInputBlur"
            ></textarea>
          </div>
          <button
            v-if="showStopAction"
            type="button"
            class="btn btn-sm btn-circle shrink-0 btn-error"
            :disabled="frozen || busy || !!stopChatDisabled"
            :title="`${t('chat.stop')} / ${t('chat.stopReplying')}`"
            @click="emit('stopChat')"
          >
            <Square class="h-3.5 w-3.5 fill-current" />
          </button>
          <button
            v-else-if="isMobileCompact && showConversationActions && canUseTransportSpeechRecording()"
            type="button"
            class="btn btn-sm btn-circle shrink-0"
            :class="recording ? 'btn-error' : 'btn-ghost'"
            :disabled="!canRecord"
            :title="recording ? t('chat.recording', { seconds: Math.max(1, Math.round(recordingMs / 1000)) }) : t('chat.holdRecord', { hotkey: recordHotkey })"
            @mousedown.prevent="emit('startRecording')"
            @mouseup.prevent="emit('stopRecording')"
            @mouseleave.prevent="recording && emit('stopRecording')"
            @touchstart.prevent="emit('startRecording')"
            @touchend.prevent="emit('stopRecording')"
          >
            <Mic class="h-3.5 w-3.5" />
          </button>
        </div>
        <div v-if="!isMobileCompact" class="flex items-center justify-between gap-2">
          <div class="flex min-w-0 items-center">
            <button
              v-if="showConversationActions"
              type="button"
              class="btn btn-sm btn-circle btn-ghost shrink-0"
              :title="t('chat.attach')"
              @click="emit('pickAttachments')"
            >
              <Paperclip class="h-3.5 w-3.5" />
            </button>
            <div
              :class="goalActive ? 'aura aura-rainbow aura-sm' : undefined"
              :style="goalActive ? { '--aura-radius': '9999px' } : undefined"
            >
              <button
                class="btn btn-sm btn-circle shrink-0"
                :class="goalActive ? 'btn-primary' : 'btn-ghost'"
                :disabled="frozen || goalDisabled"
                :title="goalTitle || t('chat.goal.buttonTitle')"
                @click="emit('openGoalTask')"
              >
                <Target class="h-3.5 w-3.5" />
              </button>
            </div>
            <ChatModelPicker
              variant="chip"
              :model-value="activeModelDisplayId"
              :api-configs="chatModelOptions"
              :theme="teleportTheme"
              @update:model-value="selectConversationPreferredModel"
            />
          </div>
          <div class="flex shrink-0 items-center gap-2">
            <button
              v-if="planModeEnabled"
              type="button"
              class="inline-flex h-8 min-h-8 shrink-0 select-none items-center rounded-full bg-info px-3 text-xs font-medium leading-none text-info-content"
              :title="`Shift+Tab ${t('chat.plan.mode')}`"
              @click="togglePlanMode()"
            >
              {{ t("chat.plan.mode") }}
            </button>
            <button
              v-else-if="planSuggestionVisible"
              type="button"
              class="inline-flex h-8 min-h-8 shrink-0 select-none items-center rounded-full bg-base-200 px-3 text-xs font-medium leading-none text-base-content"
              :title="`Shift+Tab ${t('chat.plan.mode')}`"
              @click="togglePlanMode()"
            >
              {{ t("chat.plan.mode") }}
            </button>
            <button
              v-if="showStopAction"
              class="btn btn-sm btn-circle shrink-0 btn-error"
              :disabled="frozen || busy || !!stopChatDisabled"
              :title="`${t('chat.stop')} / ${t('chat.stopReplying')}`"
              @click="emit('stopChat')"
            >
              <Square class="h-3.5 w-3.5 fill-current" />
            </button>
            <div v-else ref="sendModeMenuRef" class="relative flex shrink-0">
              <button
                type="button"
                class="btn btn-sm btn-circle shrink-0"
                :class="composerInputBlank ? 'bg-base-200' : 'btn-success'"
                :disabled="!composerInputBlank && (frozen || busy)"
                :title="composerInputBlank ? t('chat.sendModeMenu') : t('chat.send')"
                @click="composerInputBlank ? (sendModeMenuOpen = !sendModeMenuOpen) : handleSendChat()"
                @contextmenu.prevent="sendModeMenuOpen = !sendModeMenuOpen"
              >
                <ArrowUp class="h-3.5 w-3.5" />
              </button>
              <div
                v-if="sendModeMenuOpen"
                class="absolute bottom-full right-0 z-50 mb-1.5 min-w-52 overflow-hidden rounded-box border border-base-300 bg-base-100 text-base-content shadow-xl"
              >
                <div class="flex flex-col p-1">
                  <button
                    type="button"
                    class="flex min-h-8 w-full items-center justify-between gap-3 rounded-lg px-2.5 text-left text-sm transition-colors hover:bg-base-200"
                    @click="setSendMode('enter')"
                  >
                    <span>{{ t("chat.sendModeEnter") }}</span>
                    <Check v-if="sendMode === 'enter'" class="h-4 w-4 shrink-0 text-primary" />
                  </button>
                  <button
                    type="button"
                    class="flex min-h-8 w-full items-center justify-between gap-3 rounded-lg px-2.5 text-left text-sm transition-colors hover:bg-base-200"
                    @click="setSendMode('ctrl_enter')"
                  >
                    <span>{{ t("chat.sendModeCtrlEnter") }}</span>
                    <Check v-if="sendMode === 'ctrl_enter'" class="h-4 w-4 shrink-0 text-primary" />
                  </button>
                  <div class="px-2.5 pt-1 pb-0.5 text-xs opacity-50">{{ t("chat.sendModeAltS") }}</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
      <template v-else>
      <div class="relative">
        <div
          class="ecall-chat-composer-input-shell w-full"
          :class="{ 'ecall-chat-composer-input-shell-with-images': clipboardImages.length > 0 }"
        >
          <div v-if="clipboardImages.length > 0" class="ecall-chat-composer-image-previews">
            <div
              v-for="(img, idx) in clipboardImages"
              :key="`${img.mime}-${idx}`"
              class="ecall-chat-composer-image-preview"
            >
              <img
                v-if="clipboardImagePreviewSrc(img)"
                class="ecall-chat-composer-image-preview-media"
                :src="clipboardImagePreviewSrc(img)"
                :alt="t('chat.image', { index: idx + 1 })"
                draggable="false"
              />
              <div v-else class="ecall-chat-composer-file-preview">
                <FileText class="h-5 w-5" />
                <span class="text-xs">{{ isPdfMime(img.mime) ? `PDF ${idx + 1}` : t("chat.image", { index: idx + 1 }) }}</span>
              </div>
              <button
                type="button"
                class="ecall-chat-composer-image-remove"
                aria-label="删除图片"
                @mousedown.prevent
                @click.stop="removeClipboardImageAt(idx)"
              >
                <X class="h-3 w-3" />
              </button>
            </div>
          </div>
          <textarea
            ref="chatInputRef"
            v-model="localChatInput"
            class="ecall-chat-composer-input w-full resize-none overflow-y-auto chat-input-no-focus min-h-0"
            rows="1"
            :placeholder="effectiveChatInputPlaceholder"
            @input="handleChatInputInput"
            @compositionstart="handleChatInputCompositionStart"
            @compositionend="handleChatInputCompositionEnd"
            @keydown="handleChatInputKeydown"
            @focus="handleChatInputFocus"
            @blur="handleChatInputBlur"
          ></textarea>
        </div>
        <FloatingScrollbar v-if="chatInputRef" :target="chatInputRef" />
      </div>
      <div class="flex items-center justify-between">
        <div class="flex items-center">
          <div
            :class="goalActive ? 'aura aura-rainbow aura-sm' : undefined"
            :style="goalActive ? { '--aura-radius': '9999px' } : undefined"
          >
            <button
              class="btn btn-sm btn-circle shrink-0"
              :class="goalActive ? 'btn-primary' : 'btn-ghost'"
              :disabled="frozen || goalDisabled"
              :title="goalTitle || t('chat.goal.buttonTitle')"
              @click="emit('openGoalTask')"
            >
              <Target class="h-3.5 w-3.5" />
            </button>
          </div>
          <button
            v-if="showConversationActions"
            class="btn btn-sm btn-circle btn-ghost shrink-0"
            :title="t('chat.attach')"
            @click="emit('pickAttachments')"
          >
            <Paperclip class="h-3.5 w-3.5" />
          </button>
          <button
            v-if="showConversationActions && canUseTransportSpeechRecording()"
            class="btn btn-sm btn-circle shrink-0"
            :class="recording ? 'btn-error' : 'btn-ghost'"
            :disabled="!canRecord"
            :title="recording ? t('chat.recording', { seconds: Math.max(1, Math.round(recordingMs / 1000)) }) : t('chat.holdRecord', { hotkey: recordHotkey })"
            @mousedown.prevent="emit('startRecording')"
            @mouseup.prevent="emit('stopRecording')"
            @mouseleave.prevent="recording && emit('stopRecording')"
            @touchstart.prevent="emit('startRecording')"
            @touchend.prevent="emit('stopRecording')"
          >
            <Mic class="h-3.5 w-3.5" />
          </button>
          <ChatModelPicker
            variant="chip"
            :model-value="activeModelDisplayId"
            :api-configs="chatModelOptions"
            :theme="teleportTheme"
            @update:model-value="selectConversationPreferredModel"
          />
        </div>
        <div class="flex items-center gap-2">
          <button
            v-if="planModeEnabled"
            type="button"
            class="inline-flex h-8 min-h-8 shrink-0 select-none items-center rounded-full bg-info px-3 text-xs font-medium leading-none text-info-content"
            :title="`Shift+Tab ${t('chat.plan.mode')}`"
            @click="togglePlanMode()"
          >
            {{ t("chat.plan.mode") }}
          </button>
          <button
            v-else-if="planSuggestionVisible"
            type="button"
            class="inline-flex h-8 min-h-8 shrink-0 select-none items-center rounded-full bg-base-200 px-3 text-xs font-medium leading-none text-base-content"
            :title="`Shift+Tab ${t('chat.plan.mode')}`"
            @click="togglePlanMode()"
          >
            {{ t("chat.plan.mode") }}
          </button>
          <button
            v-if="showStopAction"
            class="btn btn-sm btn-circle shrink-0 btn-error"
            :disabled="frozen || busy || !!stopChatDisabled"
            :title="`${t('chat.stop')} / ${t('chat.stopReplying')}`"
            @click="emit('stopChat')"
          >
            <Square class="h-3.5 w-3.5 fill-current" />
          </button>
          <div v-else ref="sendModeMenuRef" class="relative flex shrink-0">
            <button
              class="btn btn-sm btn-circle shrink-0"
              :class="composerInputBlank ? 'bg-base-200' : 'btn-success'"
              :disabled="!composerInputBlank && (frozen || busy)"
              :title="composerInputBlank ? t('chat.sendModeMenu') : t('chat.send')"
              @click="composerInputBlank ? (sendModeMenuOpen = !sendModeMenuOpen) : handleSendChat()"
              @contextmenu.prevent="sendModeMenuOpen = !sendModeMenuOpen"
            >
              <ArrowUp class="h-3.5 w-3.5" />
            </button>
            <div
              v-if="sendModeMenuOpen"
              class="absolute bottom-full right-0 z-50 mb-1.5 min-w-52 overflow-hidden rounded-box border border-base-300 bg-base-100 text-base-content shadow-xl"
            >
              <div class="flex flex-col p-1">
                <button
                  type="button"
                  class="flex min-h-8 w-full items-center justify-between gap-3 rounded-lg px-2.5 text-left text-sm transition-colors hover:bg-base-200"
                  @click="setSendMode('enter')"
                >
                  <span>{{ t("chat.sendModeEnter") }}</span>
                  <Check v-if="sendMode === 'enter'" class="h-4 w-4 shrink-0 text-primary" />
                </button>
                <button
                  type="button"
                  class="flex min-h-8 w-full items-center justify-between gap-3 rounded-lg px-2.5 text-left text-sm transition-colors hover:bg-base-200"
                  @click="setSendMode('ctrl_enter')"
                >
                  <span>{{ t("chat.sendModeCtrlEnter") }}</span>
                  <Check v-if="sendMode === 'ctrl_enter'" class="h-4 w-4 shrink-0 text-primary" />
                </button>
                <div class="px-2.5 pt-1 pb-0.5 text-xs opacity-50">{{ t("chat.sendModeAltS") }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>
      </template>
      <Teleport to="body">
        <div
          v-if="mentionPanelOpen"
          class="fixed z-1200"
          :data-theme="teleportTheme"
          :style="mentionPanelStyle"
        >
          <div
            ref="mentionPanelScrollRef"
            class="dropdown-content max-h-[min(56vh,24rem)] w-max max-w-[min(80vw,20rem)] overflow-y-auto overscroll-contain rounded-box border border-base-300 bg-base-100 p-1 text-base-content shadow-xl"
          >
            <ul class="flex flex-col gap-1">
              <li
                v-for="(item, index) in filteredMentionOptions"
                :key="`${item.agentId}:${item.departmentId}`"
              >
                <button
                  type="button"
                  :data-mention-option-index="index"
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
                      class="indicator-item inline-flex h-4 w-4 items-center justify-center rounded-full bg-primary text-micro font-bold text-primary-content"
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
                        <div v-else class="bg-neutral text-neutral-content w-7 h-7 rounded-full flex items-center justify-center text-caption">
                          {{ avatarInitial(item.agentName) }}
                        </div>
                      </div>
                    </div>
                  </div>
                  <div class="min-w-0 flex-1 pr-0.5">
                    <div class="truncate text-sm leading-5">@{{ mentionDisplayLabel(item) }}</div>
                    <div
                      v-if="!item.mentionable && item.unavailableReason"
                      class="truncate text-xs leading-4 text-base-content/60"
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
    </div>
    </template>
    </template>
  </div>
</template>

<script setup lang="ts">
import { Teleport, computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { ArrowUp, CalendarPlus, Check, ClipboardList, FileText, History, Menu, Mic, Minus, Paperclip, Plus, Settings, Square, Target, X } from "@lucide/vue";
import type { ApiConfigItem, ChatConversationOverviewItem, ChatMentionEntry, ChatMentionTarget, ConversationForwardTarget, IdeContextReferenceItem, IdeContextWorkspaceGroup, PromptCommandPreset, RemoteImContactConversationOption } from "../../../types/app";
import ChatQueuePreview from "./ChatQueuePreview.vue";
import ChatSelectionActionPanel from "./ChatSelectionActionPanel.vue";
import ChatModelPicker from "../../config/components/ApiConfigPicker.vue";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";
import { useChatQueue } from "../composables/use-chat-queue";
import { chatInputEnterConfirmsComposition } from "../composables/chat-composer-ime";
import { clearChatComposerFocus, registerChatComposerFocus } from "../composables/chat-composer-focus";
import type { DepartmentPersonaOption } from "../../shared/department-persona-options";
import { ideContextReferenceDisplayParts } from "../utils/ide-context-reference-display";
import { mergeComposerIdeContextGroups } from "../utils/ide-context-reference-groups";
import { isMobileTouchViewport } from "../../shared/utils/mobile-viewport";
import { canUseTransportSpeechRecording } from "../../../services/tauri-api";

type BinaryAttachment = { mime: string; bytesBase64: string; previewDataUrl?: string };
type QueuedAttachmentNotice = { id: string; fileName: string; path: string; mime: string; pending?: boolean };
type ConversationDepartmentOption = DepartmentPersonaOption;
type MentionOptionView = {
  agentId: string;
  agentName: string;
  departmentId: string;
  departmentName: string;
  avatarUrl?: string;
  mentionable: boolean;
  hidden?: boolean;
  unavailableReason?: string;
};

const props = defineProps<{
  composerScope?: "main" | "side";
  selectionModeEnabled: boolean;
  selectionDelegateOnly?: boolean;
  selectedMessageCount: number;
  chatInput: string;
  instructionPresets: PromptCommandPreset[];
  mentionEntries: ChatMentionEntry[];
  selectedMentions: ChatMentionTarget[];
  clipboardImages: BinaryAttachment[];  queuedAttachmentNotices: QueuedAttachmentNotice[];
  linkOpenErrorText: string;
  transcribing: boolean;
  canRecord: boolean;
  recording: boolean;
  recordingMs: number;
  recordHotkey: string;
  conversationCallPrimaryApiConfigId: string;
  preferredChatModelId?: string;
  chatModelOptions: ApiConfigItem[];
  workspaceAccess?: "read_only" | "approval" | "full_access" | "";
  planModeEnabled: boolean;
  chatting: boolean;
  frontendRoundPhase?: "idle" | "queued" | "waiting" | "streaming";
  busy: boolean;
  stopChatDisabled?: boolean;
  frozen: boolean;
  goalActive: boolean;
  goalTitle: string;
  goalDisabled?: boolean;
  systemNotificationMode?: boolean;
  remoteContactMode?: boolean;
  showSideConversationList: boolean;
  activeConversationId: string;
  unarchivedConversationItems: ChatConversationOverviewItem[];
  remoteImContactConversations: RemoteImContactConversationOption[];
  userAlias: string;
  userAvatarUrl: string;
  personaName: string;
  personaNameMap: Record<string, string>;
  personaAvatarUrlMap: Record<string, string>;
  createConversationDepartmentOptions: ConversationDepartmentOption[];
  defaultCreateConversationDepartmentId: string;
  ideContextGroups: IdeContextWorkspaceGroup[];
  attachedIdeContextReferences: IdeContextReferenceItem[];
  currentTheme?: string;
  showConversationActions?: boolean;
  chatUsagePercent?: number;
  activeAgentId?: string;
}>();

const emit = defineEmits<{
  (e: "exitSelectionMode"): void;
  (e: "selectionActionBranch"): void;
  (e: "selectionActionForward", target: ConversationForwardTarget): void;
  (e: "selectionActionDelegate", payload: { departmentId: string; agentId: string; presetId: string; why: string; goal: string; todo: string }): void;
  (e: "selectionActionCopy"): void;
  (e: "selectionActionShare", format: "html" | "png" | "copyPng"): void;
  (e: "update:chatInput", value: string): void;
  (e: "addMention", value: ChatMentionTarget): void;
  (e: "removeMention", value: string | { agentId: string; departmentId?: string }): void;
  (e: "removeClipboardImage", index: number): void;
  (e: "removeQueuedAttachmentNotice", index: number): void;
  (e: "startRecording"): void;
  (e: "stopRecording"): void;
  (e: "pickAttachments"): void;
  (e: "update:conversationPreferredApiConfigId", value: string): void;
  (e: "update:workspaceAccess", value: "read_only" | "approval" | "full_access"): void;
  (e: "update:planModeEnabled", value: boolean): void;
  (e: "attachIdeContextReference", value: IdeContextReferenceItem): void;
  (e: "removeIdeContextReference", value: string): void;
  (e: "sendChat"): void;
  (e: "stopChat"): void;
  (e: "openDelegateSelection"): void;
  (e: "openTaskCreate"): void;
  (e: "openGoalTask"): void;
  (e: "open-conversation-list"): void;
  (e: "open-settings"): void;
  (e: "trim-conversation"): void;
  (e: "createConversation", input?: { departmentId?: string; agentId?: string }): void;
}>();

const { t } = useI18n();
const queueEnabled = computed(() => true);
const showConversationActions = computed(() => props.showConversationActions ?? true);
const systemNotificationMode = computed(() => !!props.systemNotificationMode);
const remoteContactMode = computed(() => !!props.remoteContactMode);

/** 输入区无可发送内容（无文字、无图片、无待发附件）时，发送按钮降级为菜单入口。 */
const composerInputBlank = computed(() => {
  if (String(props.chatInput || "").trim()) return false;
  return props.clipboardImages.length === 0 && props.queuedAttachmentNotices.length === 0;
});

/** 计划类请求关键词：命中时显示可点击的「计划」按钮（base-200），点击进入计划模式。 */
const PLAN_SUGGESTION_KEYWORDS = ["计划", "方案", "plan", "design"];
const planSuggestionVisible = computed(() => {
  if (props.planModeEnabled) return false;
  const text = String(props.chatInput || "").toLowerCase();
  if (!text) return false;
  return PLAN_SUGGESTION_KEYWORDS.some((keyword) => text.includes(keyword.toLowerCase()));
});

// Product rule: an in-flight assistant reply must not lock the input toolbar.
// Users can keep typing while streaming, so do not use `chatting` as the disabled
// condition for attach/record/command/task/delegate actions. Only gate on real
// hard blockers such as frozen state, explicit busy flows, permissions, or action-specific prerequisites.
const teleportTheme = computed(() => {
  const documentTheme = typeof document === "undefined" ? "" : document.documentElement.getAttribute("data-theme");
  return String(props.currentTheme || documentTheme || "light").trim() || "light";
});

function openCreateConversationDialog() {
  if (typeof window === "undefined") {
    emit("createConversation");
    return;
  }
  window.dispatchEvent(new CustomEvent("easy-call:open-draft-conversation"));
}

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
  const target = event.target as Node | null;
  if (menuOpen.value) {
    if (menuWrapperRef.value && menuWrapperRef.value.contains(target)) return;
    closeMenu();
  }
  if (sendModeMenuOpen.value) {
    const sendModeRoot = sendModeMenuRef.value;
    if (sendModeRoot && sendModeRoot.contains(target)) return;
    sendModeMenuOpen.value = false;
  }
}

onMounted(() => { document.addEventListener('pointerdown', onMenuOutsideClick); });
onBeforeUnmount(() => { document.removeEventListener('pointerdown', onMenuOutsideClick); });

const { queueEvents, sessionState, recallQueueEvent, markGuided } = useChatQueue({
  enabled: queueEnabled,
});

const visibleQueueEvents = computed(() => {
  const activeConversationId = String(props.activeConversationId || "").trim();
  if (!activeConversationId) return [];
  return queueEvents.value.filter(
    (event) => String(event.conversationId || "").trim() === activeConversationId,
  );
});

const queueUserPersonaName = computed(() =>
  String(props.personaNameMap["user-persona"] || props.userAlias || "").trim(),
);

/** 输入框占位文案：按忙碌/队列/引导状态切换，忙碌态嵌入人格名。 */
const effectiveChatInputPlaceholder = computed(() => {
  const personaName = String(props.personaName || "").trim();
  if (visibleQueueEvents.value.some((event) => event.queueMode === "guided")) {
    return t("chat.placeholderGuided", { personaName });
  }
  if (visibleQueueEvents.value.length > 0) {
    return t("chat.placeholderBusyQueued", { personaName });
  }
  if (props.busy || props.chatting) {
    return t("chat.placeholderBusyIdle", { personaName });
  }
  return t("chat.placeholder", { personaName });
});

const localChatInput = computed({
  get: () => props.chatInput,
  set: (value: string) => emit("update:chatInput", value),
});
const CHAT_INPUT_HISTORY_STORAGE_KEY = "easy_call.chat_input_history.v1";
const CHAT_INPUT_HISTORY_LIMIT = 100;
const SEND_MODE_STORAGE_KEY = "easy_call.send_mode.v1";
type SendMode = "enter" | "ctrl_enter";
const composerRootRef = ref<HTMLDivElement | null>(null);
const chatInputRef = ref<HTMLTextAreaElement | null>(null);
const chatInputComposing = ref(false);
const chatInputCompositionEndedAt = ref(0);

const sendMode = ref<SendMode>("enter");
const sendModeMenuOpen = ref(false);
const sendModeMenuRef = ref<HTMLDivElement | null>(null);

/** 手机窄屏（触摸）下的单行输入条：默认收起为单行，展开为完整输入模式。 */
const mobileViewport = ref(isMobileTouchViewport());
const mobileComposerExpanded = ref(false);
const isMobileCompact = computed(() => mobileViewport.value && !mobileComposerExpanded.value);

function syncMobileViewport() {
  mobileViewport.value = isMobileTouchViewport();
  if (!mobileViewport.value) {
    mobileComposerExpanded.value = false;
  }
}

/** 展开为完整输入模式；focus=false 时只展开不聚焦（外部文本注入场景，不弹键盘）。 */
function expandMobileComposer(focus = true) {
  mobileComposerExpanded.value = true;
  void nextTick(() => {
    if (focus) chatInputRef.value?.focus();
    scheduleResizeChatInput();
  });
}

watch(isMobileCompact, (compact) => {
  if (compact && chatInputRef.value) {
    chatInputRef.value.style.height = "";
  }
});

// 外部文本进入输入框（语音转写、消息召回等）时展开卡片，否则单行放不下也无法发送
watch(() => props.chatInput, (value) => {
  if (String(value || "") && isMobileCompact.value) expandMobileComposer(false);
});

function loadSendMode() {
  try {
    const raw = window.localStorage.getItem(SEND_MODE_STORAGE_KEY);
    if (raw === "ctrl_enter") sendMode.value = "ctrl_enter";
  } catch {
    // ignore storage failures
  }
}

function setSendMode(mode: SendMode) {
  sendMode.value = mode;
  sendModeMenuOpen.value = false;
  try {
    window.localStorage.setItem(SEND_MODE_STORAGE_KEY, mode);
  } catch {
    // ignore persistence failures
  }
}

const chatInputHistory = ref<string[]>([]);
const chatInputHistoryCursor = ref(-1);
const chatInputHistoryDraft = ref("");
const chatInputHistoryApplying = ref(false);
const resizeInputRaf = ref(0);
const instructionPanelOpen = ref(false);
const instructionFocusIndex = ref(0);
const mentionPanelOpen = ref(false);
const mentionQuery = ref("");
const mentionFocusIndex = ref(0);
const mentionRange = ref<{ start: number; end: number } | null>(null);
const mentionPanelScrollRef = ref<HTMLDivElement | null>(null);
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
const localModelOptionId = ref("");

function modelOptionIdFromProps(): string {
  return String(props.preferredChatModelId || "").trim()
    || String(props.conversationCallPrimaryApiConfigId || "").trim();
}

watch(
  () => String(props.activeConversationId || "").trim(),
  () => {
    localModelOptionId.value = modelOptionIdFromProps();
  },
  { immediate: true },
);

watch(
  () => [
    String(props.preferredChatModelId || "").trim(),
    String(props.conversationCallPrimaryApiConfigId || "").trim(),
  ].join("|"),
  () => {
    localModelOptionId.value = modelOptionIdFromProps();
  },
);

// 传给 ChatModelPicker 的当前模型 id：本地选择为空时回退会话主模型
const activeModelDisplayId = computed(() =>
  localModelOptionId.value || String(props.conversationCallPrimaryApiConfigId || "").trim());
const showIdeWorkspaceGroupLabel = computed(() => false);
const attachedIdeContextReferenceIds = computed(() => new Set((props.attachedIdeContextReferences || []).map((item) => item.id)));
const mergedIdeContextGroups = computed<IdeContextWorkspaceGroup[]>(() => mergeComposerIdeContextGroups(
  props.ideContextGroups || [],
  props.attachedIdeContextReferences || [],
));
function normalizedComposerPathKey(value: string): string {
  return String(value || "").trim().replace(/\\/g, "/").toLowerCase();
}
const mergedIdeContextPathKeys = computed(() => new Set(
  mergedIdeContextGroups.value
    .flatMap((group) => group.references || [])
    .map((item) => normalizedComposerPathKey(item.filePath || item.relativePath || ""))
    .filter(Boolean),
));
const visibleQueuedAttachmentNotices = computed(() =>
  (Array.isArray(props.queuedAttachmentNotices) ? props.queuedAttachmentNotices : []).filter((item) => {
    const pathKey = normalizedComposerPathKey(item.path || "");
    return !pathKey || !mergedIdeContextPathKeys.value.has(pathKey);
  }),
);

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
  (props.chatting || ["queued", "waiting", "streaming"].includes(String(props.frontendRoundPhase || "idle")))
  && composerInputBlank.value,
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
  const query = mentionQuery.value.trim().toLowerCase();
  return (Array.isArray(props.mentionEntries) ? props.mentionEntries : [])
    .map((item) => ({
      agentId: String(item?.agentId || "").trim(),
      agentName: String(item?.agentName || "").trim(),
      departmentId: String(item?.departmentId || "").trim(),
      departmentName: String(item?.departmentName || "").trim(),
      avatarUrl: String(item?.avatarUrl || "").trim() || undefined,
      mentionable: !!item?.mentionable,
      hidden: !!item?.hidden,
      unavailableReason: String(item?.unavailableReason || "").trim() || undefined,
    }))
    .filter((item) => !!item.agentId && !!item.agentName && !item.hidden)
    .filter((item) => {
      if (!query) return true;
      if (item.agentName.toLowerCase().includes(query)) return true;
      if (item.departmentName && item.departmentName.toLowerCase().includes(query)) return true;
      return false;
    });
});

const planModeToggleAllowed = computed(() => !props.frozen);

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

function buildInstructionPresetInput(currentText: string, prompt: string): string {
  const current = String(currentText || "");
  const nextPrompt = String(prompt || "").trim();
  if (!nextPrompt) return current;
  if (!current) return nextPrompt;
  return `${current}\n\n${nextPrompt}`;
}

function applyInstructionPreset(item: PromptCommandPreset | undefined) {
  if (!item) return;
  const prompt = String(item.prompt || item.name || "").trim();
  if (!prompt) return;
  const nextValue = buildInstructionPresetInput(localChatInput.value, prompt);
  localChatInput.value = nextValue;
  closeInstructionPanel();
  closeMentionPanel();
  nextTick(() => {
    scheduleResizeChatInput();
    const el = chatInputRef.value;
    if (!el) return;
    el.focus({ preventScroll: true });
    const cursor = nextValue.length;
    el.setSelectionRange(cursor, cursor);
  });
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
  const target = list[nextIndex];
  if (!target.mentionable) return;
  mentionFocusIndex.value = nextIndex;
  applyMention(target);
}

function moveMentionFocus(delta: number) {
  const list = filteredMentionOptions.value;
  if (list.length === 0) return;
  let next = mentionFocusIndex.value + delta;
  while (next >= 0 && next < list.length && !list[next].mentionable) {
    next += delta;
  }
  if (next < 0 || next >= list.length) return;
  mentionFocusIndex.value = next;
  scrollMentionFocusIntoView();
}

function scrollMentionFocusIntoView() {
  nextTick(() => {
    const container = mentionPanelScrollRef.value;
    if (!container) return;
    const active = container.querySelector<HTMLElement>(`[data-mention-option-index="${mentionFocusIndex.value}"]`);
    active?.scrollIntoView({ block: "nearest" });
  });
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
  const match = beforeCursor.match(/(?:^|\s)@([^\s@]*)$/);
  if (!match) {
    closeMentionPanel();
    return;
  }
  const query = match[1];
  mentionQuery.value = query;
  const atStart = cursor - 1 - query.length;
  mentionRange.value = { start: atStart, end: cursor };
  refreshMentionPanelPosition();
  mentionPanelOpen.value = true;
  const firstMentionable = filteredMentionOptions.value.findIndex((item) => item.mentionable);
  mentionFocusIndex.value = firstMentionable >= 0 ? firstMentionable : 0;
}

function selectConversationPreferredModel(id: string) {
  const nextId = String(id || "").trim();
  if (!nextId || nextId === activeModelDisplayId.value) return;
  localModelOptionId.value = nextId;
  emit("update:conversationPreferredApiConfigId", nextId);
}

function togglePlanMode() {
  if (!planModeToggleAllowed.value) return;
  emit("update:planModeEnabled", !props.planModeEnabled);
}

function resizeChatInput() {
  const el = chatInputRef.value;
  if (!el) return;
  // 移动紧凑态同样随内容增高（主流聊天应用行为）；软键盘收起时由
  // watch(isMobileCompact) 清空高度缩回单行。
  const minHeight = mobileViewport.value ? 24 : 48;
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

function handleChatInputCompositionStart() {
  chatInputComposing.value = true;
}

function handleChatInputCompositionEnd() {
  chatInputComposing.value = false;
  chatInputCompositionEndedAt.value = performance.now();
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
  if (isMobileTouchViewport()) {
    chatInputRef.value?.blur();
    mobileComposerExpanded.value = false;
  }
  emit("sendChat");
  recordSentTextIfNeeded(plainText);
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

function handleChatInputFocus() {
  if (props.composerScope) {
    registerChatComposerFocus(props.composerScope);
  }
  // 单行条聚焦即展开为完整输入模式
  if (isMobileCompact.value) {
    mobileComposerExpanded.value = true;
    scheduleResizeChatInput();
  }
}

function handleChatInputBlur(event: FocusEvent) {
  if (props.composerScope) {
    clearChatComposerFocus(props.composerScope);
  }
  if (!mobileViewport.value) return;
  const nextTarget = event.relatedTarget as Node | null;
  if (nextTarget && composerRootRef.value?.contains(nextTarget)) return;
  // 延迟一拍再收起：同一次点击可能正在打开 Teleport 到 body 的浮层（如模型抽屉），
  // 浮层存在时保持展开，避免第二行工具按钮被误缩回
  window.setTimeout(() => {
    if (!mobileComposerExpanded.value) return;
    if (document.querySelector("[data-composer-overlay]")) return;
    mobileComposerExpanded.value = false;
  }, 150);
}

function handleChatInputKeydown(event: KeyboardEvent) {
  if (
    chatInputEnterConfirmsComposition(
      event,
      chatInputComposing.value,
      chatInputCompositionEndedAt.value,
      performance.now(),
    )
  ) {
    return;
  }
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
  if (event.key === "Escape" && props.chatting && showStopAction.value && !props.stopChatDisabled) {
    event.preventDefault();
    emit("stopChat");
    return;
  }
  const ctrlEnterPressed = event.key === "Enter" && event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey;
  const plainEnterPressed = event.key === "Enter" && !event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey;
  // Alt+S 兜底发送：无论当前发送模式，始终可用（输入法组合中除外）
  const altSPressed = event.key.toLowerCase() === "s" && event.altKey && !event.ctrlKey && !event.metaKey && !event.shiftKey && !event.isComposing;
  if (altSPressed) {
    if (props.frozen) return;
    event.preventDefault();
    handleSendChat();
    return;
  }
  if (sendMode.value === "ctrl_enter") {
    if (ctrlEnterPressed) {
      if (props.frozen) return;
      event.preventDefault();
      handleSendChat();
      return;
    }
    // Ctrl+Enter 模式：普通 Enter 保留为换行
    if (plainEnterPressed) return;
  } else if (plainEnterPressed) {
    if (props.frozen) return;
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

function isPdfMime(mime: string): boolean {
  return (mime || "").trim().toLowerCase() === "application/pdf";
}

function clipboardImagePreviewSrc(image: BinaryAttachment): string {
  const previewDataUrl = String(image?.previewDataUrl || "").trim();
  if (previewDataUrl.startsWith("data:image/")) return previewDataUrl;
  const mime = String(image?.mime || "").trim().toLowerCase();
  const bytesBase64 = String(image?.bytesBase64 || "").trim();
  if (!mime.startsWith("image/") || !bytesBase64) return "";
  return `data:${mime};base64,${bytesBase64}`;
}

function removeClipboardImageAt(index: number) {
  emit("removeClipboardImage", index);
  void nextTick(() => focusInput({ preventScroll: true }));
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
  loadSendMode();
  syncMobileViewport();
  window.addEventListener("keydown", handleWindowKeydown);
  window.addEventListener("resize", refreshMentionPanelPosition);
  window.addEventListener("scroll", refreshMentionPanelPosition, true);
  window.addEventListener("resize", syncMobileViewport);
  nextTick(() => {
    resizeChatInput();
    refreshMentionPanelPosition();
  });
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", handleWindowKeydown);
  window.removeEventListener("resize", refreshMentionPanelPosition);
  window.removeEventListener("scroll", refreshMentionPanelPosition, true);
  window.removeEventListener("resize", syncMobileViewport);
  if (resizeInputRaf.value) {
    cancelAnimationFrame(resizeInputRaf.value);
    resizeInputRaf.value = 0;
  }
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
    if (wasChatting && !isChatting && !isMobileTouchViewport()) {
      nextTick(() => focusInput({ preventScroll: true }));
    }
  },
);

watch(
  () => props.activeConversationId,
  () => {
    closeInstructionPanel();
    closeMentionPanel();
    nextTick(() => scheduleResizeChatInput());
  },
);

watch(
  () => normalizedInstructionPresets.value,
  (list) => {
    if (list.length === 0) {
      instructionFocusIndex.value = 0;
      instructionPanelOpen.value = false;
      return;
    }
    if (instructionFocusIndex.value >= list.length) {
      instructionFocusIndex.value = list.length - 1;
    }
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
.ecall-chat-composer-input-shell {
  appearance: none;
  border: 0;
  box-sizing: border-box;
  display: flex;
  min-height: 48px;
  flex-direction: column;
  gap: 8px;
  background: transparent;
  box-shadow: none;
  outline: 0;
  padding: 0;
}
.ecall-chat-composer-input-shell:focus,
.ecall-chat-composer-input-shell:focus-within,
.ecall-chat-composer-input-shell:focus-visible {
  border: 0;
  box-shadow: none;
  outline: 0;
}
.ecall-chat-composer-input-shell-with-images {
  padding: 10px 12px 12px;
}
.ecall-chat-composer-image-previews {
  display: flex;
  max-height: 4.5rem;
  flex-wrap: wrap;
  gap: 8px;
  overflow: hidden;
}
.ecall-chat-composer-image-preview {
  position: relative;
  display: inline-flex;
  min-height: 3.5rem;
  max-width: 9.5rem;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  border-radius: 0.5rem;
  background: color-mix(in srgb, var(--color-base-200) 72%, transparent);
}
.ecall-chat-composer-image-preview-media {
  display: block;
  max-height: 3.75rem;
  max-width: 9.5rem;
  object-fit: contain;
}
.ecall-chat-composer-file-preview {
  display: inline-flex;
  height: 3.5rem;
  min-width: 5.75rem;
  align-items: center;
  justify-content: center;
  gap: 0.375rem;
  padding: 0 0.75rem;
  color: color-mix(in srgb, var(--color-base-content) 72%, transparent);
}
.ecall-chat-composer-image-remove {
  position: absolute;
  right: 4px;
  top: 4px;
  display: inline-flex;
  height: 1.25rem;
  width: 1.25rem;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-base-100) 88%, transparent);
  color: color-mix(in srgb, var(--color-base-content) 72%, transparent);
  opacity: 0;
  transition: opacity 120ms ease, color 120ms ease, background-color 120ms ease;
}
.ecall-chat-composer-image-preview:hover .ecall-chat-composer-image-remove,
.ecall-chat-composer-image-remove:focus-visible {
  opacity: 1;
}
.ecall-chat-composer-image-remove:hover {
  background: color-mix(in srgb, var(--color-error) 90%, transparent);
  color: var(--color-error-content);
}
.ecall-chat-composer-input {
  appearance: none;
  box-sizing: border-box;
  border: 0;
  background: transparent;
  line-height: 1.5;
  outline: 0;
  padding-top: 12px;
  padding-bottom: 12px;
}
.ecall-chat-composer-input:focus,
.ecall-chat-composer-input:focus-visible {
  border: 0;
  box-shadow: none;
  outline: 0;
}
.ecall-chat-composer-input-shell .ecall-chat-composer-input {
  padding-left: 12px;
  padding-right: 12px;
}
.ecall-chat-composer-input-shell-with-images .ecall-chat-composer-input {
  padding: 0;
}
</style>
