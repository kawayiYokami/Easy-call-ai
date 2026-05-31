<template>
  <div
    ref="chatLayoutRoot"
    class="relative h-full min-h-0"
    :class="showSideConversationList && !detachedChatWindow ? 'flex flex-row overflow-hidden' : 'flex flex-col relative'"
  >
    <div
      v-if="showSideConversationList && !detachedChatWindow"
      class="flex h-full min-h-0 shrink-0"
      :style="{ width: `${leftSidebarWidth}px` }"
    >
      <ChatConversationSidebar
        :items="conversationItems || unarchivedConversationItems"
        :active-conversation-id="activeConversationId"
        :user-alias="userAlias"
        :user-avatar-url="userAvatarUrl"
        :persona-name-map="personaNameMap"
        :persona-avatar-url-map="personaAvatarUrlMap"
        :active-tab="conversationListTab"
        @update:active-tab="$emit('updateConversationListTab', $event)"
        @select="handleConversationListSelect"
        @rename="handleConversationRename"
        @toggle-pin-conversation="handleConversationPinToggle"
        @archive-conversation="handleConversationArchive"
        @delete-conversation="handleConversationDelete"
      />
    </div>

    <div class="flex min-h-0 min-w-0 flex-1 overflow-hidden">
      <div class="relative flex min-h-0 min-w-0 flex-1 flex-col">
        <div
          v-if="mediaDragActive && !chatting && !frozen && !conversationBusy"
          class="pointer-events-none absolute inset-0 z-40 flex items-center justify-center bg-base-100/70 backdrop-blur-[1px]"
        >
          <div class="rounded-box border border-primary/40 bg-base-100 px-4 py-2 text-sm font-medium text-primary">
            {{ t("chat.dropImageOrPdf") }}
          </div>
        </div>

        <div class="relative flex min-h-0 flex-1 overflow-hidden" @mouseenter="chatScrollbarRef?.reveal()" @mouseleave="chatScrollbarRef?.hide()">
          <div
            ref="scrollContainer"
            class="ecall-chat-scroll-container relative flex flex-1 min-h-0 flex-col overflow-x-hidden overflow-y-auto px-0 py-3"
            :class="chatting || frozen || conversationBusy ? 'pointer-events-auto' : ''"
            :data-chat-interaction-locked="chatting || frozen || conversationBusy ? 'true' : undefined"
            @scroll="onConversationScroll"
          >
          <div
            v-if="loadingOlderHistory"
            class="pointer-events-none sticky top-0 z-10 flex justify-center pb-2"
          >
            <div class="badge badge-ghost gap-2 border-base-300 bg-base-100/90 px-3 py-3 shadow-sm">
              <span class="loading loading-spinner loading-xs"></span>
              <span>{{ t("chat.loadingOlderMessages") }}</span>
            </div>
          </div>

          <ConversationTodoDropdown :todos="normalizedConversationTodos" :persona-name="personaName" />
          <div class="ecall-chat-history-flow flex min-w-0 shrink-0 flex-col">
            <div class="relative min-w-0 w-full shrink-0" :style="{ height: `${totalVirtualSize}px` }">
              <div
                v-for="entry in virtualEntries"
                :key="entry.item.id"
                :data-index="entry.row.index"
                :data-render-item-id="entry.item.id"
                :ref="(el) => measureVirtualRow(entry.item.id, el)"
                class="absolute left-0 top-0 w-full ecall-chat-virtual-item"
                :style="{ transform: `translateY(${entry.row.start}px)` }"
              >
                <div
                  v-if="entry.item.kind === 'compaction'"
                  class="mt-4 flex items-center gap-3 text-[11px] text-base-content/45"
                >
                  <div class="h-px flex-1 bg-base-300/80"></div>
                  <button type="button" class="btn btn-ghost btn-xs shrink-0 gap-1.5 px-2 text-base-content/60 hover:text-base-content"
                    :title="t('chat.viewSummary')" @click="openConversationSummary(entry.item.block, $event)">
                    <History class="h-3.5 w-3.5" />
                    <span>{{ t("chat.viewSummary") }}</span>
                  </button>
                  <div class="h-px flex-1 bg-base-300/80"></div>
                </div>
                <div v-else-if="entry.item.kind === 'plan_started'" class="mt-4 flex items-center gap-3 text-[11px] text-base-content/45">
                  <div class="h-px flex-1 bg-base-300/80"></div>
                  <span class="shrink-0 rounded-full border border-base-300/80 bg-base-100 px-3 py-1 text-base-content/55">{{ t("chat.planStartedDivider") }}</span>
                  <div class="h-px flex-1 bg-base-300/80"></div>
                </div>
                <div v-else-if="entry.item.kind === 'time_divider'" class="my-3 flex items-center gap-3 px-3 text-[11px] text-base-content/45">
                  <div class="h-px flex-1 bg-base-300/70"></div>
                  <time class="shrink-0 rounded-full border border-base-300/70 bg-base-100/90 px-3 py-1 text-base-content/55 shadow-sm"
                    :datetime="entry.item.createdAt">{{ formatTimeDividerLabel(entry.item.createdAt) }}</time>
                  <div class="h-px flex-1 bg-base-300/70"></div>
                </div>
                <div v-else-if="entry.item.kind === 'message'"
                  v-memo="messageMemoKey(entry.item.block, entry.item.renderId, entry.item.blockIndex, entry.item.compactWithPrevious)">
                  <div class="ecall-elastic-item-shell"
                    :style="entry.item.id === latestOwnElasticItemId ? { minHeight: `${latestOwnElasticMinHeight}px` } : undefined">
                    <ChatMessageItem
                      :active-conversation-id="activeConversationId" :block="entry.item.block"
                      :selection-key="entry.item.renderId" :selection-mode-enabled="messageSelectionModeEnabled"
                      :selected="selectedMessageRenderIdSet.has(entry.item.renderId)"
                      :chatting="chatting" :busy="conversationBusy" :frozen="frozen"
                      :user-alias="userAlias" :user-avatar-url="userAvatarUrl"
                      :persona-name-map="personaNameMap" :persona-avatar-url-map="personaAvatarUrlMap"
                      :stream-tool-calls="visibleStreamToolCalls" :markdown-is-dark="markdownIsDark"
                      :playing-audio-id="playingAudioId" :active-turn-user="false"
                      :compact-with-previous="entry.item.compactWithPrevious"
                      :can-regenerate="canRegenerateBlock(entry.item.block, entry.item.blockIndex)"
                      :can-confirm-plan="canConfirmPlan(entry.item.block)"
                      :bubble-background-hidden="isBubbleBackgroundHidden(entry.item.block)"
                      :hide-toggle-enabled="canToggleBubbleBackground(entry.item.block)"
                      @recall-turn="$emit('recallTurn', $event)" @regenerate-turn="$emit('regenerateTurn', $event)"
                      @confirm-plan="$emit('confirmPlan', $event)" @enter-selection-mode="enterMessageSelectionMode"
                      @toggle-message-selected="toggleMessageSelected" @copy-message="copyMessage"
                      @open-image-preview="openImagePreview"
                      @toggle-audio-playback="toggleAudioPlayback($event.id, $event.audio)"
                      @assistant-link-click="handleAssistantLinkClick"
                      @toggle-bubble-background="toggleBubbleBackground(entry.item.block)"
                    />
                  </div>
                </div>
                <div v-else class="ecall-turn-group">
                  <div class="ecall-turn-stack" :style="entry.item.id === latestOwnElasticItemId ? { minHeight: `${latestOwnElasticMinHeight}px` } : undefined">
                    <template v-for="groupItem in entry.item.items" :key="groupItem.renderId">
                      <ChatMessageItem
                        v-memo="messageMemoKey(groupItem.block, groupItem.renderId, groupItem.blockIndex, groupItem.compactWithPrevious)"
                        :active-conversation-id="activeConversationId" :block="groupItem.block"
                        :selection-key="groupItem.renderId" :selection-mode-enabled="messageSelectionModeEnabled"
                        :selected="selectedMessageRenderIdSet.has(groupItem.renderId)"
                        :chatting="chatting" :busy="conversationBusy" :frozen="frozen"
                        :user-alias="userAlias" :user-avatar-url="userAvatarUrl"
                        :persona-name-map="personaNameMap" :persona-avatar-url-map="personaAvatarUrlMap"
                        :stream-tool-calls="visibleStreamToolCalls" :markdown-is-dark="markdownIsDark"
                        :playing-audio-id="playingAudioId" :active-turn-user="false"
                        :compact-with-previous="groupItem.compactWithPrevious"
                        :can-regenerate="canRegenerateBlock(groupItem.block, groupItem.blockIndex)"
                        :can-confirm-plan="canConfirmPlan(groupItem.block)"
                        :bubble-background-hidden="isBubbleBackgroundHidden(groupItem.block)"
                        :hide-toggle-enabled="canToggleBubbleBackground(groupItem.block)"
                        @recall-turn="$emit('recallTurn', $event)" @regenerate-turn="$emit('regenerateTurn', $event)"
                        @confirm-plan="$emit('confirmPlan', $event)" @enter-selection-mode="enterMessageSelectionMode"
                        @toggle-message-selected="toggleMessageSelected" @copy-message="copyMessage"
                        @open-image-preview="openImagePreview"
                        @toggle-audio-playback="toggleAudioPlayback($event.id, $event.audio)"
                        @assistant-link-click="handleAssistantLinkClick"
                        @toggle-bubble-background="toggleBubbleBackground(groupItem.block)"
                      />
                    </template>
                  </div>
                </div>
              </div>
            </div>

            <div ref="toolbarContainer" class="ecall-chat-toolbar-shell px-2 pt-1 pb-2">
              <ChatWorkspaceToolbar
                :chatting="chatting" :frozen="frozen" :conversation-busy="conversationBusy"
                :workspace-button-label="t('chat.allowedWorkspaceButton')" :workspace-button-name="currentWorkspaceName"
                :workspace-button-disabled="!activeConversationId || activeConversationSummary?.kind === 'remote_im_contact'"
                :hide-menu-button="activeConversationSummary?.kind === 'remote_im_contact'"
                :hide-workspace-button="activeConversationSummary?.kind === 'remote_im_contact'"
                :mention-entries="mentionEntries" :selected-mention-keys="selectedMentionKeys"
                :supervision-active="supervisionActive" :supervision-label="t('chat.supervision.button')"
                :supervision-active-label="t('chat.supervision.buttonActive')" :supervision-title="supervisionButtonTitle"
                :supervision-disabled="activeConversationSummary?.kind === 'remote_im_contact'"
                :show-detach-button="!detachedChatWindow && !activeConversationSummary?.isMainConversation"
                :detach-disabled="!activeConversationId || activeConversationSummary?.isMainConversation || chatting || frozen || conversationBusy"
                @lock-workspace="$emit('lockWorkspace')" @open-branch-selection="openBranchSelectionMenu"
                @open-delegate-selection="openDelegateSelectionMenu" @open-forward-selection="openForwardSelectionMenu"
                @open-share-selection="openShareSelectionMenu"
                @mention-entry="(entry) => {
                  const agentId = String(entry?.agentId || '').trim();
                  const departmentId = String(entry?.departmentId || '').trim();
                  if (!agentId || !departmentId) return;
                  const mentionKey = `${agentId}:${departmentId}`;
                  if (selectedMentionKeys.includes(mentionKey)) { emit('removeMention', { agentId, departmentId }); return; }
                  emit('addMention', { agentId, agentName: String(entry?.agentName || '').trim() || agentId, departmentId, departmentName: String(entry?.departmentName || '').trim() || departmentId, avatarUrl: String(entry?.avatarUrl || '').trim() || undefined });
                }"
                @open-supervision-task="$emit('openSupervisionTask')" @detach-conversation="handleDetachConversationRequest"
              />
            </div>
          </div>
          </div>
          <FloatingScrollbar ref="chatScrollbarRef" :target="scrollContainer" />
        </div>
        <CompactionSummaryCard :visible="conversationSummaryCard.visible" :text="conversationSummaryCard.text" @close="closeConversationSummaryCard" />
        <div v-show="showJumpToBottom" class="pointer-events-none absolute bottom-3 right-5 z-30 flex justify-end" :style="jumpToBottomStyle">
          <button class="btn btn-sm btn-circle btn-primary pointer-events-auto shadow-lg" :title="t('chat.jumpToBottom')" @click="handleJumpToBottom">
            <ChevronsDown class="h-4 w-4" />
          </button>
        </div>

        <div ref="composerContainer" class="relative shrink-0 border-t border-base-300 bg-base-100 p-2">
          <div v-if="chatStatusBanner" class="pointer-events-none absolute inset-x-0 top-0 z-10 -translate-y-full">
            <div class="relative flex w-full items-center justify-center rounded-none px-4 py-1.5 text-center text-[12px] backdrop-blur-md"
              :class="chatStatusBanner.tone === 'error' ? 'bg-error/12 text-error' : chatStatusBanner.text === t('chat.statusCompactingContext') ? 'bg-info/12 text-info' : 'bg-base-200/75 text-base-content'">
              <span class="relative z-1" :class="chatStatusBanner.tone === 'error' ? '' : 'text-base-content/80 ecall-shimmer-text ecall-reasoning-shimmer'"
                :data-shimmer-text="chatStatusBanner.tone === 'error' ? '' : chatStatusBanner.text">{{ chatStatusBanner.text }}</span>
            </div>
          </div>
          <ChatApprovalPanel
            v-if="activeConversationTerminalApprovals.length > 0"
            :approvals="activeConversationTerminalApprovals" :resolving="terminalApprovalResolving"
            @approve="$emit('approveTerminalApproval', $event)" @deny="$emit('denyTerminalApproval', $event)"
          />
          <ChatComposerPanel
            v-else ref="composerPanelRef" :selection-mode-enabled="messageSelectionModeEnabled"
            :selected-message-count="selectedMessageBlocks.length"
            :chat-input="chatInput" :instruction-presets="instructionPresets" :mention-entries="mentionEntries"
            :selected-mentions="selectedMentions" :chat-input-placeholder="chatInputPlaceholder"
            :clipboard-images="clipboardImages" :queued-attachment-notices="queuedAttachmentNotices"
            :link-open-error-text="linkOpenErrorText" :chat-error-text="chatErrorText"
            :transcribing="transcribing" :can-record="canRecord" :recording="recording" :recording-ms="recordingMs"
            :record-hotkey="recordHotkey" :selected-chat-model-id="selectedChatModelId"
            :chat-model-options="chatModelOptions" :plan-mode-enabled="planModeEnabled"
            :frontend-round-phase="frontendRoundPhase" :chat-usage-percent="chatUsagePercent"
            :force-archive-tip="forceArchiveTip" :chatting="chatting" :busy="conversationBusy"
            :stop-chat-disabled="isOrganizingContextBusy" :frozen="frozen"
            :show-side-conversation-list="detachedChatWindow ? false : showSideConversationList"
            :active-conversation-id="activeConversationId" :unarchived-conversation-items="unarchivedConversationItems"
            :user-alias="userAlias" :user-avatar-url="userAvatarUrl"
            :persona-name-map="personaNameMap" :persona-avatar-url-map="personaAvatarUrlMap"
            :create-conversation-department-options="createConversationDepartmentOptions"
            :delegate-department-ids="delegateDepartmentIds"
            :default-create-conversation-department-id="defaultCreateConversationDepartmentId"
            :ide-context-groups="visibleIdeContextGroups" :attached-ide-context-references="attachedIdeContextReferences"
            @update:chat-input="$emit('update:chatInput', $event)" @add-mention="$emit('addMention', $event)"
            @remove-mention="$emit('removeMention', $event)" @remove-clipboard-image="$emit('removeClipboardImage', $event)"
            @remove-queued-attachment-notice="$emit('removeQueuedAttachmentNotice', $event)"
            @start-recording="$emit('startRecording')" @stop-recording="$emit('stopRecording')"
            @pick-attachments="$emit('pickAttachments')"
            @update:selected-chat-model-id="$emit('update:selectedChatModelId', $event)"
            @update:plan-mode-enabled="$emit('update:planModeEnabled', $event)"
            @attach-ide-context-reference="handleAttachIdeContextReference"
            @remove-ide-context-reference="handleRemoveIdeContextReference"
            @send-chat="handleSendChat" @stop-chat="$emit('stopChat')"
            @exit-selection-mode="exitMessageSelectionMode"
            @selection-action-copy="copySelectedMessages"
            @selection-action-branch="emitSelectionAction('branch')"
            @selection-action-forward="emitSelectionAction('forward', $event)"
            @selection-action-delegate="emitSelectionAction('delegate', $event)"
            @selection-action-share="emitSelectionAction('share', $event)"
            @force-archive="$emit('forceArchive')" @switch-conversation="$emit('switchConversation', $event)"
            @create-conversation="$emit('createConversation', $event)"
          />
        </div>

        <ChatImagePreviewDialog
          :open="imagePreviewOpen" :data-url="imagePreviewDataUrl" :zoom="imagePreviewZoom"
          :min-zoom="IMAGE_PREVIEW_MIN_ZOOM" :max-zoom="IMAGE_PREVIEW_MAX_ZOOM"
          :offset-x="previewOffsetX" :offset-y="previewOffsetY" :dragging="previewDragging"
          @close="closeImagePreview" @zoom-in="zoomInPreview" @zoom-out="zoomOutPreview"
          @reset="resetPreviewZoom" @wheel="onPreviewWheel" @pointer-down="onPreviewPointerDown"
          @pointer-move="onPreviewPointerMove" @pointer-up="onPreviewPointerUp"
        />

        <ChatSupervisionTaskDialog
          :open="supervisionDialogOpen" :saving="supervisionTaskSaving" :error-text="supervisionTaskError"
          :active-task="activeSupervisionTask" :recent-history="recentSupervisionTaskHistory"
          @close="$emit('closeSupervisionTask')" @save="$emit('saveSupervisionTask', $event)"
        />
      </div>

      <div v-if="toolReviewPanelOpen" class="flex h-full min-h-0 shrink-0 border-l border-base-300 bg-base-100 pt-2"
        :style="{ width: `${rightSidebarWidth}px` }">
        <ToolReviewSidebar ref="toolReviewSidebarRef" class="w-full"
          :batches="toolReviewBatches" :current-batch-key="toolReviewCurrentBatchKey"
          :detail-map="toolReviewDetailMap" :detail-loading-call-id="toolReviewDetailLoadingCallId"
          :reviewing-call-id="toolReviewReviewingCallId" :batch-reviewing-key="toolReviewBatchReviewingKey"
          :submitting-batch-key="toolReviewSubmittingBatchKey" :error-text="toolReviewErrorText"
          :report-error-text="toolReviewReportErrorText" :reports="toolReviewReports"
          :current-report-id="toolReviewCurrentReportId" :markdown-is-dark="markdownIsDark"
          :current-workspace-name="currentWorkspaceName" :current-workspace-root-path="currentWorkspaceRootPath"
          :workspaces="workspaces" :current-department-id="currentDepartmentId"
          :department-options="toolReviewDepartmentOptions" :delegate-statuses="delegateStatuses"
          :delegate-loading="delegateStatusesLoading" :delegate-error-text="delegateStatusesErrorText"
          @select-batch="setToolReviewCurrentBatchKey" @load-item-detail="loadToolReviewItemDetail"
          @review-item="runToolReviewForCall" @review-batch="runToolReviewForBatch"
          @pick-commit-review="handlePickCommitReview" @review-code="handleToolReviewCode"
          @retry-report="handleRetryToolReviewReport" @delete-report="handleDeleteToolReviewReport"
          @copy-report="copyToolReviewReport" @attach-report="$emit('attachToolReviewReport', $event)"
          @open-delegate-detail="openDelegateArchiveDetail" @abort-delegate="abortDelegate"
        />
      </div>
    </div>

    <div
      v-if="showSideConversationList && !detachedChatWindow"
      class="ecall-pane-splitter ecall-pane-splitter-left absolute bottom-0 top-0 z-30"
      :class="{ 'ecall-pane-splitter-active': activePaneResizeSide === 'left' }"
      :style="{ left: `${leftSidebarWidth - 2}px` }"
      role="separator"
      tabindex="0"
      aria-orientation="vertical"
      :aria-valuemin="PANE_WIDTH_LIMITS.left.min"
      :aria-valuemax="PANE_WIDTH_LIMITS.left.max"
      :aria-valuenow="leftSidebarWidth"
      @pointerdown="startPaneResize('left', $event)"
      @keydown.left.prevent="adjustPaneWidthByKeyboard('left', -24)"
      @keydown.right.prevent="adjustPaneWidthByKeyboard('left', 24)"
    ></div>

    <div
      v-if="toolReviewPanelOpen"
      class="ecall-pane-splitter ecall-pane-splitter-right absolute bottom-0 top-0 z-30"
      :class="{ 'ecall-pane-splitter-active': activePaneResizeSide === 'right' }"
      :style="{ right: `${rightSidebarWidth - 2}px` }"
      role="separator" tabindex="0" aria-orientation="vertical"
      :aria-valuemin="PANE_WIDTH_LIMITS.right.min" :aria-valuemax="PANE_WIDTH_LIMITS.right.max"
      :aria-valuenow="rightSidebarWidth"
      @pointerdown="startPaneResize('right', $event)"
      @keydown.left.prevent="adjustPaneWidthByKeyboard('right', 24)"
      @keydown.right.prevent="adjustPaneWidthByKeyboard('right', -24)"
    ></div>

    <!-- Relationship State 面板 -->
    <RelationshipPanel v-if="!toolReviewPanelOpen" :conversation-id="activeConversationId" :agent-id="activeAgentId" />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, toRef, type ComponentPublicInstance, type Ref } from "vue";
import { useI18n } from "vue-i18n";
import { isDarkAppTheme } from "../../shell/composables/use-app-theme";
import { ChevronsDown, History } from "lucide-vue-next";
import "markstream-vue/index.css";
import { invokeTauri } from "../../../services/tauri-api";
import type { ApiConfigItem, ChatConversationOverviewItem, ChatMentionEntry, ChatMentionTarget, ChatMessageBlock, ChatPersonaPresenceChip, ChatTodoItem, ConversationDelegateStatusSummary, IdeContextReferenceItem, IdeContextWorkspaceGroup, PromptCommandPreset, ShellWorkspace } from "../../../types/app";
import ChatMessageItem from "../components/ChatMessageItem.vue";
import ChatApprovalPanel from "../components/ChatApprovalPanel.vue";
import ChatComposerPanel from "../components/ChatComposerPanel.vue";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";
import ChatConversationSidebar from "../components/ChatConversationSidebar.vue";
import ChatWorkspaceToolbar from "../components/ChatWorkspaceToolbar.vue";
import ToolReviewSidebar from "../components/ToolReviewSidebar.vue";
import RelationshipPanel from "../components/RelationshipPanel.vue";
import ChatImagePreviewDialog from "../components/dialogs/ChatImagePreviewDialog.vue";
import ChatSupervisionTaskDialog from "../components/dialogs/ChatSupervisionTaskDialog.vue";
import ConversationTodoDropdown from "../components/ConversationTodoDropdown.vue";
import CompactionSummaryCard from "../components/CompactionSummaryCard.vue";
import { useChatImagePreview } from "../composables/use-chat-image-preview";
import { useChatMessageActions } from "../composables/use-chat-message-actions";
import { useChatScrollLayout } from "../composables/use-chat-scroll-layout";
import { useChatToolReview, type ToolReviewCodeReviewScope, type ToolReviewCommitOption, type ToolReviewReportRecord } from "../composables/use-chat-tool-review";
import type { TerminalApprovalConversationItem } from "../../shell/composables/use-terminal-approval";
import { isAbsoluteLocalPath, normalizeLocalLinkHref } from "../utils/local-link";
import { type ChatRenderItem, isRightAlignedMessage, canOpenInFileReader } from "../utils/chat-render";
import { useIdeContext } from "../composables/use-ide-context";
import { useDelegateStatus } from "../composables/use-delegate-status";
import { useBubbleBackground } from "../composables/use-bubble-background";
import { useChatVirtualList } from "../composables/use-chat-virtual-list";
import { useChatVirtualScroll } from "../composables/use-chat-virtual-scroll";
import { useChatPanes, PANE_WIDTH_LIMITS, type UseChatPanesOptions } from "../composables/use-chat-panes";
import { useChatSelection } from "../composables/use-chat-selection";
import { useChatConversationCtx } from "../composables/use-chat-conversation-ctx";
import { useChatScrollOrchestration } from "../composables/use-chat-scroll-orchestration";
import { useChatToolReviewHandlers } from "../composables/use-chat-tool-review-handlers";
import { useChatBlockTracking } from "../composables/use-chat-block-tracking";

// ==================== props / emits ====================

const props = defineProps<{
  userAlias: string; personaName: string; userAvatarUrl: string; assistantAvatarUrl: string;
  personaNameMap: Record<string, string>; personaAvatarUrlMap: Record<string, string>;
  mentionEntries: ChatMentionEntry[]; selectedMentions: ChatMentionTarget[];
  latestUserText: string; latestUserImages: Array<{ mime: string; bytesBase64: string }>;
  latestAssistantText: string; latestReasoningStandardText: string; latestReasoningInlineText: string;
  frontendRoundPhase: "idle" | "queued" | "waiting" | "streaming";
  toolStatusText: string; toolStatusState: "running" | "done" | "failed" | "";
  streamToolCalls: Array<{ name: string; argsText: string; status?: "doing" | "done" }>;
  chatErrorText: string; clipboardImages: Array<{ mime: string; bytesBase64: string }>;
  queuedAttachmentNotices: Array<{ id: string; fileName: string; relativePath: string; mime: string }>;
  chatInput: string; instructionPresets: PromptCommandPreset[]; chatInputPlaceholder: string;
  canRecord: boolean; recording: boolean; recordingMs: number; transcribing: boolean; recordHotkey: string;
  selectedChatModelId: string; toolReviewRefreshTick: number; chatModelOptions: ApiConfigItem[];
  planModeEnabled: boolean; chatUsagePercent: number; forceArchiveTip: string;
  mediaDragActive: boolean; chatting: boolean; forcingArchive: boolean; forcingArchiveConversationId?: string;
  compactingConversation: boolean; compactingConversationId?: string;
  conversationBusy: boolean; frozen: boolean; messageBlocks: ChatMessageBlock[];
  hasMoreHistory: boolean; loadingOlderHistory: boolean;
  latestOwnMessageAlignRequest: number; conversationScrollToBottomRequest: number;
  currentWorkspaceName: string; currentWorkspaceRootPath: string; workspaces: ShellWorkspace[];
  currentDepartmentId: string; activeConversationId: string; activeAgentId: string; currentTodos: ChatTodoItem[];
  supervisionActive: boolean; supervisionTitle: string; supervisionDialogOpen: boolean;
  supervisionTaskSaving: boolean; supervisionTaskError: string;
  activeSupervisionTask: { taskId: string; goal: string; why: string; todo: string; endAtLocal: string; remainingHours: number } | null;
  recentSupervisionTaskHistory: Array<{ goal: string; why: string; todo: string; durationHours: number }>;
  currentTheme: string; unarchivedConversationItems: ChatConversationOverviewItem[];
  conversationItems?: ChatConversationOverviewItem[]; sideConversationListVisible: boolean;
  initialToolReviewPanelOpen: boolean;
  conversationListTab: "local" | "contact";
  createConversationDepartmentOptions: Array<{ id: string; name: string; ownerAgentId?: string; ownerName: string; providerName?: string; modelName?: string }>;
  delegateDepartmentIds: string[]; defaultCreateConversationDepartmentId: string;
  ideContextGroups: IdeContextWorkspaceGroup[]; attachedIdeContextReferences: IdeContextReferenceItem[];
  detachedChatWindow?: boolean; terminalApprovals?: TerminalApprovalConversationItem[];
  terminalApprovalResolving?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:chatInput", value: string): void;
  (e: "addMention", value: ChatMentionTarget): void;
  (e: "removeMention", value: string | { agentId: string; departmentId?: string }): void;
  (e: "sideConversationListVisibleChange", value: boolean): void;
  (e: "toolReviewPanelOpenChange", value: boolean): void;
  (e: "sidePanelWidthsChange", value: { leftWidth: number; rightWidth: number }): void;
  (e: "sidePanelWidthsCommit", value: { leftWidth: number; rightWidth: number }): void;
  (e: "updateConversationListTab", value: "local" | "contact"): void;
  (e: "removeClipboardImage", index: number): void;
  (e: "removeQueuedAttachmentNotice", index: number): void;
  (e: "startRecording"): void; (e: "stopRecording"): void; (e: "pickAttachments"): void;
  (e: "update:selectedChatModelId", value: string): void;
  (e: "update:planModeEnabled", value: boolean): void;
  (e: "sendChat", payload?: { extraTextBlocks?: string[] }): void;
  (e: "stopChat"): void; (e: "forceArchive"): void;
  (e: "recallTurn", payload: { turnId: string }): void;
  (e: "regenerateTurn", payload: { turnId: string }): void;
  (e: "confirmPlan", payload: { messageId: string }): void;
  (e: "lockWorkspace"): void; (e: "openSupervisionTask"): void;
  (e: "detachConversation"): void; (e: "closeSupervisionTask"): void;
  (e: "saveSupervisionTask", payload: { durationHours: number; goal: string; why: string; todo: string }): void;
  (e: "switchConversation", payload: { conversationId: string; kind?: "local_unarchived" | "remote_im_contact"; remoteContactId?: string }): void;
  (e: "renameConversation", payload: { conversationId: string; title: string }): void;
  (e: "togglePinConversation", conversationId: string): void;
  (e: "archiveConversation", conversationId: string): void;
  (e: "deleteConversation", conversationId: string): void;
  (e: "createConversation", input?: { title?: string; departmentId?: string }): void;
  (e: "loadOlderHistory"): void; (e: "reachedBottom"): void;
  (e: "jumpToConversationBottom"): void;
  (e: "refreshToolReviewMessage", payload: { conversationId: string; messageId: string }): void;
  (e: "attachToolReviewReport", reportText: string): void;
  (e: "selectionActionCopy", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[] }): void;
  (e: "selectionActionCopyError", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; error: string }): void;
  (e: "selectionActionBranch", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[] }): void;
  (e: "selectionActionForward", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; targetConversationId: string }): void;
  (e: "selectionActionDelegate", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; departmentId: string; presetId: string; background: string; question: string; focus: string }): void;
  (e: "selectionActionShare", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; exportFormat?: "html" | "png" }): void;
  (e: "approveTerminalApproval", requestId: string): void;
  (e: "denyTerminalApproval", requestId: string): void;
}>();

// ==================== basic state ====================

const { t } = useI18n();
const toolReviewSidebarRef = ref<ComponentPublicInstance<{ setCommitOptions: (items: ToolReviewCommitOption[], loading?: boolean, total?: number, page?: number, pageSize?: number) => void }> | null>(null);
const chatScrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);
const linkOpenErrorText = ref("");
const conversationSummaryCard = ref<{ visible: boolean; text: string }>({ visible: false, text: "" });
const composerPanelRef = ref<{ focusInput: (opts?: FocusOptions) => void } | null>(null);

// ==================== context computed ====================

const {
  markdownIsDark, visibleStreamToolCalls, normalizedConversationTodos,
  activeConversationSummary, isCurrentConversationCompacting,
  activeConversationTerminalApprovals, supervisionButtonTitle,
  isOrganizingContextBusy, chatStatusBanner, selectedMentionKeys,
  latestPendingPlanMessageId,
} = useChatConversationCtx({ ...props, isDarkAppTheme }, t);

const toolReviewDepartmentOptions = computed(() => {
  const allowed = new Set((Array.isArray(props.delegateDepartmentIds) ? props.delegateDepartmentIds : []).map((id) => String(id || "").trim()).filter(Boolean));
  return (Array.isArray(props.createConversationDepartmentOptions) ? props.createConversationDepartmentOptions : []).filter((item) => allowed.has(String(item.id || "").trim()));
});

// ==================== messages / audio / bubble ====================

const { playingAudioId, copyMessage, stopAudioPlayback, toggleAudioPlayback } = useChatMessageActions();
const { isHidden: isBubbleBackgroundHidden, canToggle: canToggleBubbleBackground, toggle: toggleBubbleBackground } = useBubbleBackground(toRef(props, "activeConversationId"));
const showSideConversationList = computed(() => !!props.sideConversationListVisible);

function canRegenerateBlock(block: ChatMessageBlock, blockIndex: number): boolean {
  if (block.role !== "assistant" || block.isExtraTextBlock) return false;
  for (let idx = props.messageBlocks.length - 1; idx >= 0; idx -= 1) {
    const candidate = props.messageBlocks[idx];
    if (candidate.role !== "assistant" || candidate.isExtraTextBlock) continue;
    return idx === blockIndex;
  }
  return false;
}

function canConfirmPlan(block: ChatMessageBlock): boolean {
  if (block.role !== "assistant" || block.isExtraTextBlock) return false;
  if (block.planCard?.action !== "present") return false;
  const targetId = String(block.sourceMessageId || block.id || "").trim();
  if (targetId !== latestPendingPlanMessageId.value) return false;
  const blockIndex = props.messageBlocks.findIndex((item) => String(item.id || "").trim() === String(block.id || "").trim());
  if (blockIndex < 0) return false;
  return !props.messageBlocks.slice(blockIndex + 1).some((item) => !item.isExtraTextBlock && item.role === "user");
}

function openSelectionMenu() {
  if (props.chatting || props.frozen || props.conversationBusy) return;
  messageSelectionModeEnabled.value = true;
  selectedMessageRenderIds.value = [];
  void nextTick(() => composerPanelRef.value?.focusInput?.({ preventScroll: true }));
}
const openBranchSelectionMenu = openSelectionMenu;
const openDelegateSelectionMenu = openSelectionMenu;
const openForwardSelectionMenu = openSelectionMenu;
const openShareSelectionMenu = openSelectionMenu;

function openConversationSummary(block: ChatMessageBlock, event?: MouseEvent) {
  event?.stopPropagation();
  const text = String(block?.text || "").trim();
  if (!text) return;
  conversationSummaryCard.value = { visible: true, text };
}
function closeConversationSummaryCard() {
  conversationSummaryCard.value = { visible: false, text: "" };
}

// ==================== ide context ====================

const {
  visibleIdeContextGroups, attachedIdeContextReferences,
  attachReference: handleAttachIdeContextReference,
  removeReference: handleRemoveIdeContextReference,
  clearAttachedReferences: clearAttachedIdeContextReferences,
} = useIdeContext({
  activeConversationId: toRef(props, "activeConversationId"),
  workspaces: toRef(props, "workspaces"),
  currentWorkspaceRootPath: toRef(props, "currentWorkspaceRootPath"),
  currentWorkspaceName: toRef(props, "currentWorkspaceName"),
});

// ==================== selection state shared between virtual list & selection mode ====================

const messageSelectionModeEnabled = ref(false);
const selectedMessageRenderIds = ref<string[]>([]);
const selectedMessageRenderIdSet = computed(() => new Set(selectedMessageRenderIds.value));

// ==================== virtual list ====================

const { chatRenderItems, messageMemoKey } = useChatVirtualList({
  messageBlocks: toRef(props, "messageBlocks"), markdownIsDark, playingAudioId,
  userAlias: toRef(props, "userAlias"), userAvatarUrl: toRef(props, "userAvatarUrl"),
  personaNameMap: toRef(props, "personaNameMap"), personaAvatarUrlMap: toRef(props, "personaAvatarUrlMap"),
  chatting: toRef(props, "chatting"), conversationBusy: toRef(props, "conversationBusy"),
  frozen: toRef(props, "frozen"), messageSelectionModeEnabled,
  selectedMessageRenderIdSet,
  isBubbleBackgroundHidden, canToggleBubbleBackground, canRegenerateBlock, canConfirmPlan,
});

const virtualRenderItems = computed<ChatRenderItem[]>(() => [...chatRenderItems.value]);

// ==================== block tracking ====================

const { isOwnMessage, latestOwnMessageId, latestOwnElasticItemId, blockChronologicalIndexMap, renderItemById } =
  useChatBlockTracking(toRef(props, "messageBlocks"), chatRenderItems);

// ==================== selection mode ====================

const {
  selectedMessageBlocks, enterMessageSelectionMode, toggleMessageSelected,
  exitMessageSelectionMode, copySelectedMessages, emitSelectionAction,
} = useChatSelection({
  chatRenderItems: computed(() => chatRenderItems.value.flatMap((item) => {
    if (item.kind === "message") return [{ renderId: item.renderId, block: item.block }];
    if (item.kind === "group") return item.items.map((g) => ({ renderId: g.renderId, block: g.block }));
    return [];
  })),
  messageSelectionModeEnabled,
  selectedMessageRenderIds,
  personaNameMap: props.personaNameMap, userAlias: props.userAlias, t,
  onEmit: {
    selectionActionCopy: (payload) => emit("selectionActionCopy", payload),
    selectionActionCopyError: (payload) => emit("selectionActionCopyError", payload),
    selectionActionBranch: (payload) => emit("selectionActionBranch", payload),
    selectionActionForward: (payload) => emit("selectionActionForward", payload),
    selectionActionDelegate: (payload) => emit("selectionActionDelegate", payload),
    selectionActionShare: (payload) => emit("selectionActionShare", payload),
  },
});

defineExpose({ exitMessageSelectionMode });

// ==================== scroll layout ====================

const {
  scrollContainer, composerContainer, toolbarContainer, chatLayoutRoot,
  latestOwnElasticMinHeight, showJumpToBottom, jumpToBottomStyle, onScroll,
  prepareBottomAlignmentLayout,
} = useChatScrollLayout({
  activeConversationId: toRef(props, "activeConversationId"),
  chatting: toRef(props, "chatting"), busy: toRef(props, "conversationBusy"),
  frozen: toRef(props, "frozen"),
  messageBlockCount: computed(() => props.messageBlocks.length),
  onReachedBottom: () => emit("reachedBottom"),
  focusComposerInput: (options) => composerPanelRef.value?.focusInput(options),
});

// ==================== virtual scroll ====================

const activeJumpToBottomRequest = ref(0);

const {
  virtualizer, virtualEntries, totalVirtualSize, measureVirtualRow,
  scheduleVirtualMeasure, syncViewportMetrics,
  resetVirtualizerAtConversationBottom, alignItemToTop, captureVisibleAnchor, findRenderedMessageElement,
  resolveMessageAnchorElement, syncVisibleStreamingVirtualItemViewportTops,
} = useChatVirtualScroll({
  renderItems: virtualRenderItems, renderItemById, blockChronologicalIndexMap,
  scrollContainer, scrollbarRef: chatScrollbarRef as Ref<{ updateThumb: () => void } | null>,
  activeJumpToBottomRequest,
  activeConversationId: toRef(props, "activeConversationId"),
  latestOwnElasticItemId,
  latestOwnElasticMinHeight,
  onUserScroll: () => onScroll(),
});

// ==================== tool review ====================

const {
  toolReviewPanelOpen, toolReviewBatches, toolReviewCurrentBatchKey,
  toolReviewDetailMap, toolReviewDetailLoadingCallId, toolReviewReviewingCallId,
  toolReviewBatchReviewingKey, toolReviewSubmittingBatchKey, toolReviewErrorText,
  toolReviewReportErrorText, toolReviewReports, toolReviewCurrentReportId,
  setToolReviewCurrentBatchKey,
  loadToolReviewItemDetail, runToolReviewForCall, runToolReviewForBatch,
  handlePickCommitReview, handleDeleteToolReviewReport, handleToolReviewCode,
  handleRetryToolReviewReport,
} = useChatToolReviewHandlers({
  activeConversationId: toRef(props, "activeConversationId"),
  toolReviewRefreshTick: toRef(props, "toolReviewRefreshTick"),
  currentDepartmentId: toRef(props, "currentDepartmentId"),
  departmentOptions: toolReviewDepartmentOptions,
  initialPanelOpen: toRef(props, "initialToolReviewPanelOpen"),
  t, syncViewportMetrics,
  onRefreshMessage: (payload) => emit("refreshToolReviewMessage", payload),
  onToolReviewPanelOpenChange: (open) => emit("toolReviewPanelOpenChange", open),
  toolReviewSidebarRef,
});

// ==================== delegate status ====================

const {
  delegateStatuses, delegateStatusesLoading, delegateStatusesErrorText,
  openDelegateArchiveDetail, abortDelegate,
} = useDelegateStatus({
  activeConversationId: toRef(props, "activeConversationId"),
  panelOpen: toolReviewPanelOpen,
});

// ==================== panes ====================

const panesCleanupFns: Array<() => void> = [];
const {
  leftSidebarWidth, rightSidebarWidth, activePaneResizeSide,
  startPaneResize, adjustPaneWidthByKeyboard,
} = useChatPanes({
  chatLayoutRoot, toolReviewPanelOpen,
  showSideConversationList, detachedChatWindow: !!props.detachedChatWindow,
  syncViewportMetrics,
  onPaneWidthsChange: (left, right) => emit("sidePanelWidthsChange", { leftWidth: left, rightWidth: right }),
  onPaneWidthsCommit: (left, right) => emit("sidePanelWidthsCommit", { leftWidth: left, rightWidth: right }),
  onBeforeUnmountCleanup: (fn) => panesCleanupFns.push(fn),
});

// ==================== scroll orchestration ====================

const {
  onConversationScroll, handleJumpToBottom,
  alignLatestOwnMessageToTop,
} = useChatScrollOrchestration({
  scrollContainer, chatScrollbarRef: chatScrollbarRef as Ref<{ updateThumb: () => void; hide?: () => void } | null>,
  activeJumpToBottomRequest,
  prepareBottomAlignmentLayout,
  onScroll, scheduleVirtualMeasure, syncViewportMetrics,
  resetConversationToBottom: resetVirtualizerAtConversationBottom,
  alignItemToTop, captureVisibleAnchor, findRenderedMessageElement, resolveMessageAnchorElement,
  syncVisibleStreamingVirtualItemViewportTops,
  refreshObservedVirtualItemElements: () => {},
  latestOwnElasticItemId,
  props: {
    hasMoreHistory: toRef(props, "hasMoreHistory"), loadingOlderHistory: toRef(props, "loadingOlderHistory"),
    chatting: toRef(props, "chatting"), conversationBusy: toRef(props, "conversationBusy"), frozen: toRef(props, "frozen"),
    activeConversationId: toRef(props, "activeConversationId"),
    conversationScrollToBottomRequest: toRef(props, "conversationScrollToBottomRequest"),
    latestOwnMessageAlignRequest: toRef(props, "latestOwnMessageAlignRequest"),
    messageBlocks: toRef(props, "messageBlocks"),
  },
  emit: { loadOlderHistory: () => emit("loadOlderHistory"), jumpToConversationBottom: () => emit("jumpToConversationBottom") },
});

// ==================== image preview ====================

const {
  imagePreviewOpen, imagePreviewDataUrl, imagePreviewZoom,
  IMAGE_PREVIEW_MIN_ZOOM, IMAGE_PREVIEW_MAX_ZOOM,
  previewOffsetX, previewOffsetY, previewDragging,
  zoomInPreview, zoomOutPreview, resetPreviewZoom,
  onPreviewWheel, openImagePreview, closeImagePreview,
  onPreviewPointerDown, onPreviewPointerMove, onPreviewPointerUp,
} = useChatImagePreview();

// ==================== conversation actions ====================

function handleDetachConversationRequest() { emit("detachConversation"); }
function handleSendChat() {
  const extraTextBlocks = attachedIdeContextReferences.value.map((item) => String(item.textBlock || "").trim()).filter(Boolean);
  emit("sendChat", extraTextBlocks.length > 0 ? { extraTextBlocks } : undefined);
  clearAttachedIdeContextReferences();
}
function handleConversationListSelect(payload: { conversationId: string; kind?: "local_unarchived" | "remote_im_contact"; remoteContactId?: string }) {
  const id = String(payload?.conversationId || "").trim();
  if (!id || id === String(props.activeConversationId || "").trim()) return;
  const target = (props.conversationItems || props.unarchivedConversationItems).find((item) => String(item.conversationId || "").trim() === id);
  emit("switchConversation", { conversationId: id, kind: payload?.kind || target?.kind, remoteContactId: String(payload?.remoteContactId || target?.remoteContactId || "").trim() || undefined });
}
function handleConversationRename(payload: { conversationId: string; title: string }) {
  const id = String(payload?.conversationId || "").trim();
  if (id) emit("renameConversation", { conversationId: id, title: String(payload?.title || "").trim() });
}
function handleConversationPinToggle(id: string) { emit("togglePinConversation", String(id || "").trim()); }
function handleConversationArchive(id: string) { emit("archiveConversation", String(id || "").trim()); }
function handleConversationDelete(id: string) { emit("deleteConversation", String(id || "").trim()); }

// ==================== link / copy ====================

async function copyToolReviewReport(reportText: string) {
  const text = String(reportText || "").trim();
  if (!text) return;
  try { await navigator.clipboard.writeText(text); } catch { /* best-effort */ }
}

async function handleAssistantLinkClick(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  const anchor = target?.closest("a") as HTMLAnchorElement | null;
  if (!anchor) return;
  const href = normalizeLocalLinkHref(anchor.getAttribute("href")?.trim() || "");
  if (!href) return;
  if (isAbsoluteLocalPath(href)) {
    event.preventDefault(); event.stopPropagation();
    try {
      if (canOpenInFileReader(href)) { await invokeTauri("open_file_reader_window_command", { path: href }); }
      else { await invokeTauri("open_local_file_directory", { path: href }); }
      linkOpenErrorText.value = "";
    } catch (error) { linkOpenErrorText.value = t("status.openLinkFailed", { err: String(error) }); }
    return;
  }
  if (href.startsWith("http://") || href.startsWith("https://")) {
    event.preventDefault(); event.stopPropagation();
    try { await invokeTauri("open_external_url", { url: href }); linkOpenErrorText.value = ""; }
    catch (error) { linkOpenErrorText.value = t("status.openLinkFailed", { err: String(error) }); }
  }
}

// ==================== time divider ====================

const timeDividerNowTick = ref(Date.now());
let timeDividerNowTimer = 0;

function padTimePart(value: number): string { return String(value).padStart(2, "0"); }
function formatLocalClock(date: Date): string { return `${padTimePart(date.getHours())}:${padTimePart(date.getMinutes())}`; }
function formatTimeDividerLabel(value?: string): string {
  const raw = String(value || "").trim();
  if (!raw) return "";
  const date = new Date(raw);
  const timestamp = date.getTime();
  if (!Number.isFinite(timestamp)) return raw;
  const now = new Date(timeDividerNowTick.value);
  const sameYear = date.getFullYear() === now.getFullYear();
  const sameMonth = date.getMonth() === now.getMonth();
  const sameDate = date.getDate() === now.getDate();
  const sameHour = sameYear && sameMonth && sameDate && date.getHours() === now.getHours();
  const elapsedMinutes = Math.floor((now.getTime() - timestamp) / 60000);
  if (sameHour && elapsedMinutes > 0) return t("chat.timeDivider.minutesAgo", { count: elapsedMinutes });
  const clock = formatLocalClock(date);
  if (sameYear && sameMonth && sameDate) return clock;
  const monthDay = `${padTimePart(date.getMonth() + 1)}-${padTimePart(date.getDate())}`;
  if (sameYear) return `${monthDay} ${clock}`;
  return `${date.getFullYear()}-${monthDay} ${clock}`;
}

// ==================== lifecycle ====================

onMounted(() => {
  timeDividerNowTimer = window.setInterval(() => { timeDividerNowTick.value = Date.now(); }, 60_000);
  void nextTick(() => chatScrollbarRef.value?.updateThumb());
});

onBeforeUnmount(() => {
  if (timeDividerNowTimer) { window.clearInterval(timeDividerNowTimer); timeDividerNowTimer = 0; }
  panesCleanupFns.forEach((fn) => fn());
  stopAudioPlayback();
});
</script>
