<template>
  <div
    ref="chatLayoutRoot"
    class="relative flex h-full min-h-0 flex-row overflow-hidden"
  >
    <div
      v-if="showSideConversationList && !detachedChatWindow"
      :class="leftPaneInLayout ? 'flex h-full min-h-0 shrink-0' : 'absolute bottom-0 left-0 top-0 z-50 flex h-full min-h-0 border-r border-base-300 bg-base-100 shadow-2xl'"
      :style="{ width: `${leftPaneVisibleWidth}px` }"
    >
      <ChatConversationSidebar
        :items="conversationItems || unarchivedConversationItems"
        :active-conversation-id="activeConversationId"
        :user-alias="userAlias"
        :user-avatar-url="userAvatarUrl"
        :persona-name-map="personaNameMap"
        :persona-avatar-url-map="personaAvatarUrlMap"
        :active-tab="chatLeftPanelMode"
        :bridge-request="bridgeRequest"
        @update:active-tab="$emit('update:conversation-list-tab', $event)"
        @edit-task="openTaskEditDialog"
        @select="handleConversationListSelect"
        @rename="handleConversationRename"
        @toggle-pin-conversation="handleConversationPinToggle"
        @archive-conversation="handleConversationArchive"
        @export-conversation="handleConversationExport"
        @delete-conversation="handleConversationDelete"
      />
    </div>

    <div class="flex min-h-0 min-w-0 flex-1 overflow-hidden">
      <div class="relative flex min-h-0 min-w-0 flex-1 flex-col">
        <div
          v-if="mediaDragActive && !chatting && !frozen && !conversationInteractionBusy"
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
            :class="chatting || frozen || conversationInteractionBusy ? 'pointer-events-auto' : ''"
            :data-chat-interaction-locked="chatting || frozen || conversationInteractionBusy ? 'true' : undefined"
            @scroll="onConversationScroll"
            @wheel="handleShiftWheel"
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
                  <time class="shrink-0 text-[11px] font-semibold text-base-content/50"
                    :datetime="entry.item.createdAt">{{ formatTimeDividerLabel(entry.item.createdAt) }}</time>
                  <div class="h-px flex-1 bg-base-300/70"></div>
                </div>
                <div v-else-if="entry.item.kind === 'message'"
                  v-memo="[...messageMemoKey(entry.item.block, entry.item.renderId, entry.item.blockIndex, entry.item.compactWithPrevious), departmentNameMapSignature]">
                  <div class="ecall-elastic-item-shell">
                    <ChatMessageItem
                      :active-conversation-id="activeConversationId" :block="entry.item.block"
                      :selection-key="entry.item.renderId" :selection-mode-enabled="messageSelectionModeEnabled"
                      :selected="selectedMessageRenderIdSet.has(entry.item.renderId)"
                      :chatting="chatting" :busy="conversationInteractionBusy" :frozen="frozen"
                      :user-alias="userAlias" :user-avatar-url="userAvatarUrl"
                      :persona-name-map="personaNameMap" :persona-avatar-url-map="personaAvatarUrlMap"
                      :department-name-map="departmentNameMap"
                      :markdown-is-dark="markdownIsDark"
                      :playing-audio-id="playingAudioId" :active-turn-user="false"
                      :compact-with-previous="entry.item.compactWithPrevious"
                      :can-regenerate="!sidebarMode && canRegenerateBlock(entry.item.block, entry.item.blockIndex)"
                      :can-confirm-plan="canConfirmPlan(entry.item.block)"
                      :read-plan-file-content="readPlanFileContent"
                      :current-workspace-root-path="currentWorkspaceRootPath"
                      :bubble-background-hidden="isBubbleBackgroundHidden(entry.item.block)"
                      :hide-toggle-enabled="canToggleBubbleBackground(entry.item.block)"
                      :disable-markdown-render="sidebarMode"
                      @recall-turn="$emit('recallTurn', $event)" @regenerate-turn="$emit('regenerateTurn', $event)"
                      @confirm-plan="$emit('confirmPlan', $event)" @enter-selection-mode="handleEnterMessageSelectionMode"
                      @toggle-message-selected="toggleMessageSelected" @copy-message="copyMessage"
                      @open-image-preview="openImagePreview"
                      @toggle-audio-playback="toggleAudioPlayback($event.id, $event.audio)"
                      @assistant-link-click="handleAssistantLinkClick"
                      @toggle-bubble-background="toggleBubbleBackground(entry.item.block)"
                    />
                  </div>
                </div>
                <div v-else class="ecall-turn-group">
                  <div class="ecall-turn-stack">
                    <template v-for="groupItem in entry.item.items" :key="groupItem.renderId">
                      <ChatMessageItem
                        v-memo="[...messageMemoKey(groupItem.block, groupItem.renderId, groupItem.blockIndex, groupItem.compactWithPrevious), departmentNameMapSignature]"
                        :active-conversation-id="activeConversationId" :block="groupItem.block"
                        :selection-key="groupItem.renderId" :selection-mode-enabled="messageSelectionModeEnabled"
                        :selected="selectedMessageRenderIdSet.has(groupItem.renderId)"
                        :chatting="chatting" :busy="conversationInteractionBusy" :frozen="frozen"
                        :user-alias="userAlias" :user-avatar-url="userAvatarUrl"
                        :persona-name-map="personaNameMap" :persona-avatar-url-map="personaAvatarUrlMap"
                        :department-name-map="departmentNameMap"
                        :markdown-is-dark="markdownIsDark"
                        :playing-audio-id="playingAudioId" :active-turn-user="false"
                        :compact-with-previous="groupItem.compactWithPrevious"
                        :can-regenerate="!sidebarMode && canRegenerateBlock(groupItem.block, groupItem.blockIndex)"
                        :can-confirm-plan="canConfirmPlan(groupItem.block)"
                        :read-plan-file-content="readPlanFileContent"
                        :current-workspace-root-path="currentWorkspaceRootPath"
                        :bubble-background-hidden="isBubbleBackgroundHidden(groupItem.block)"
                        :hide-toggle-enabled="canToggleBubbleBackground(groupItem.block)"
                        :disable-markdown-render="sidebarMode"
                        @recall-turn="$emit('recallTurn', $event)" @regenerate-turn="$emit('regenerateTurn', $event)"
                        @confirm-plan="$emit('confirmPlan', $event)" @enter-selection-mode="handleEnterMessageSelectionMode"
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

            <div :style="{ minHeight: `${latestOwnTailSpacerMinHeight}px` }"></div>
            <div
              v-if="!activeConversationIsSystemNotification && !activeConversationIsRemoteContact"
              ref="toolbarContainer"
              class="ecall-chat-toolbar-shell mx-auto w-full max-w-[900px] px-4 pt-1 pb-2"
            >
                <ChatWorkspaceToolbar
                  :chatting="chatting" :frozen="frozen" :conversation-busy="conversationInteractionBusy"
                  :workspace-button-label="t('chat.allowedWorkspaceButton')" :workspace-button-name="currentWorkspaceName"
                  :workspace-button-disabled="!activeConversationId || activeConversationSummary?.kind === 'remote_im_contact'"
                  :auto-push-active="!!String(activeConversationSummary?.autoPushRemoteContactId || '').trim()"
                  :hide-menu-button="activeConversationSummary?.kind === 'remote_im_contact'"
                  :hide-workspace-button="hideWorkspaceButton || activeConversationSummary?.kind === 'remote_im_contact'"
                  :show-task-create-menu-item="!sidebarMode && !activeConversationIsRemoteContact && !activeConversationIsSystemNotification"
                  :show-forward-menu-item="!sidebarMode"
                :show-auto-push-menu-item="!sidebarMode && !activeConversationIsRemoteContact && !activeConversationIsSystemNotification"
                :show-share-menu-item="!sidebarMode"
                :show-workspace-menu-item="true"
                :show-code-review-menu-item="true"
                :mention-entries="mentionEntries" :selected-mention-keys="selectedMentionKeys"
                :delegate-statuses="delegateStatuses"
                :show-detach-button="!bridgeMode && !detachedChatWindow && !activeConversationIsSystemNotification"
                :detach-disabled="bridgeMode || !activeConversationId || activeConversationIsSystemNotification || chatting || frozen || conversationInteractionBusy"
                @lock-workspace="$emit('lockWorkspace')" @open-branch-selection="openBranchSelectionMenu"
                @open-task-create="openTaskCreateDialog"
                @open-delegate-selection="openDelegateSelectionMenu" @open-forward-selection="openForwardSelectionMenu"
                @open-auto-push="openAutoPushCard"
                @open-share-selection="openShareSelectionMenu"
                @open-delegate-summary="openDelegateSummaryPanel"
                @open-code-review="openCodeReviewDialog"
                @mention-entry="(entry) => {
                  const agentId = String(entry?.agentId || '').trim();
                  const departmentId = String(entry?.departmentId || '').trim();
                  if (!agentId || !departmentId) return;
                  const mentionKey = `${agentId}:${departmentId}`;
                  if (selectedMentionKeys.includes(mentionKey)) { emit('removeMention', { agentId, departmentId }); return; }
                  emit('addMention', { agentId, agentName: String(entry?.agentName || '').trim() || agentId, departmentId, departmentName: String(entry?.departmentName || '').trim() || departmentId, avatarUrl: String(entry?.avatarUrl || '').trim() || undefined });
                }"
                @detach-conversation="handleDetachConversationRequest"
              />
            </div>
          </div>
          </div>
          <FloatingScrollbar ref="chatScrollbarRef" :target="scrollContainer" />
        </div>
        <CompactionSummaryCard
          :visible="conversationSummaryCard.visible"
          :text="conversationSummaryCard.text"
          :is-dark="markdownIsDark"
          @close="closeConversationSummaryCard"
        />
        <ConversationAutoPushCard
          :open="autoPushCardOpen"
          :saving="autoPushSaving"
          :enabled="autoPushEnabled"
          :selected-contact-id="autoPushSelectedContactId"
          :options="props.remoteImContactConversations"
          @close="closeAutoPushCard"
          @save="saveAutoPushCard"
          @update:enabled="autoPushEnabled = $event"
          @update:selected-contact-id="autoPushSelectedContactId = $event"
        />
        <div v-show="showJumpToBottom" class="pointer-events-none absolute bottom-3 right-5 z-30 flex justify-end" :style="jumpToBottomStyle">
          <button class="btn btn-sm btn-circle btn-primary pointer-events-auto shadow-lg" :title="t('chat.jumpToBottom')" @click="handleJumpToBottom">
            <ChevronsDown class="h-4 w-4" />
          </button>
        </div>

        <div ref="composerContainer" class="relative shrink-0 border-t border-base-300 bg-base-100 p-2">
          <div v-if="chatStatusBanner" class="absolute inset-x-0 top-0 z-10 -translate-y-full">
            <div class="relative flex w-full items-center justify-center gap-2 rounded-none px-4 py-1.5 text-center text-[12px] backdrop-blur-md"
              :class="chatStatusBanner.tone === 'error' ? 'bg-error/12 text-error' : chatStatusBanner.text === t('chat.statusCompactingContext') ? 'bg-info/12 text-info' : 'bg-base-200/75 text-base-content'">
              <span class="relative z-1" :class="chatStatusBanner.tone === 'error' ? '' : 'text-base-content/80 ecall-shimmer-text ecall-reasoning-shimmer'"
                :data-shimmer-text="chatStatusBanner.tone === 'error' ? '' : chatStatusBanner.text">{{ chatStatusBanner.text }}</span>
              <button
                v-if="chatStatusBanner.tone === 'error'"
                type="button"
                class="btn btn-ghost btn-xs h-5 min-h-5 w-5 shrink-0 p-0 text-error hover:bg-error/15"
                :title="t('common.close')"
                @click="$emit('clearChatError')"
              >
                <X class="h-3.5 w-3.5" />
              </button>
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
            :record-hotkey="recordHotkey" :conversation-call-primary-api-config-id="conversationCallPrimaryApiConfigId"
            :preferred-chat-model-id="preferredChatModelId"
            :chat-model-options="chatModelOptions" :plan-mode-enabled="planModeEnabled"
            :workspace-access="workspaceAccess"
            :frontend-round-phase="frontendRoundPhase" :chat-usage-percent="chatUsagePercent"
            :trim-tip="trimTip" :chatting="chatting" :busy="conversationInteractionBusy"
            :stop-chat-disabled="isOrganizingContextBusy" :frozen="frozen"
            :supervision-active="supervisionActive"
            :supervision-title="supervisionButtonTitle"
            :supervision-disabled="activeConversationSummary?.kind === 'remote_im_contact'"
            :system-notification-mode="activeConversationIsSystemNotification"
            :remote-contact-mode="activeConversationIsRemoteContact"
            :selection-delegate-only="messageSelectionDelegateOnly"
            :show-side-conversation-list="detachedChatWindow ? false : showSideConversationList"
            :active-conversation-id="activeConversationId" :unarchived-conversation-items="unarchivedConversationItems"
            :remote-im-contact-conversations="remoteImContactConversations"
            :user-alias="userAlias" :user-avatar-url="userAvatarUrl"
            :persona-name-map="personaNameMap" :persona-avatar-url-map="personaAvatarUrlMap"
            :create-conversation-department-options="createConversationDepartmentOptions"
            :default-create-conversation-department-id="defaultCreateConversationDepartmentId"
            :ide-context-groups="mergedVisibleIdeContextGroups" :attached-ide-context-references="attachedIdeContextReferences"
            :sidebar-mode="sidebarMode"
            @update:chat-input="$emit('update:chatInput', $event)" @add-mention="$emit('addMention', $event)"
            @remove-mention="$emit('removeMention', $event)" @remove-clipboard-image="$emit('removeClipboardImage', $event)"
            @remove-queued-attachment-notice="$emit('removeQueuedAttachmentNotice', $event)"
            @start-recording="$emit('startRecording')" @stop-recording="$emit('stopRecording')"
            @pick-attachments="$emit('pickAttachments')"
            @update:conversation-preferred-api-config-id="$emit('update:conversationPreferredApiConfigId', $event)"
            @update:workspace-access="$emit('updateWorkspaceAccess', $event)"
            @update:plan-mode-enabled="$emit('update:planModeEnabled', $event)"
            @attach-ide-context-reference="handleAttachIdeContextReference"
            @remove-ide-context-reference="handleRemoveIdeContextReference"
            @send-chat="handleSendChat" @stop-chat="$emit('stopChat')"
            @open-delegate-selection="openDelegateSelectionMenu"
            @open-task-create="openTaskCreateDialog"
            @open-supervision-task="$emit('openSupervisionTask')"
            @exit-selection-mode="handleExitMessageSelectionMode"
            @selection-action-copy="copySelectedMessages"
            @selection-action-branch="emitSelectionAction('branch')"
            @selection-action-forward="emitSelectionAction('forward', $event)"
            @selection-action-delegate="emitSelectionAction('delegate', $event)"
            @selection-action-share="emitSelectionAction('share', $event)"
            @trim-conversation="$emit('trimConversation')" @open-conversation-list="$emit('openConversationList')" @open-settings="$emit('openSettings')" @switch-conversation="$emit('switchConversation', $event)"
            @create-conversation="$emit('createConversation', $event)"
          />
        </div>

        <ChatImagePreviewDialog
          :open="imagePreviewOpen" :data-url="imagePreviewDataUrl" :zoom="imagePreviewZoom"
          :min-zoom="IMAGE_PREVIEW_MIN_ZOOM" :max-zoom="IMAGE_PREVIEW_MAX_ZOOM"
          :offset-x="previewOffsetX" :offset-y="previewOffsetY" :dragging="previewDragging"
          :local-path="imagePreviewLocalPath"
          :copy-status="imagePreviewCopyStatus as any"
          :save-status="imagePreviewSaveStatus as any"
          @close="closeImagePreview" @zoom-in="zoomInPreview" @zoom-out="zoomOutPreview"
          @reset="resetPreviewZoom" @wheel="onPreviewWheel" @pointer-down="onPreviewPointerDown"
          @pointer-move="onPreviewPointerMove" @pointer-up="onPreviewPointerUp"
          @copy-image="handleCopyLocalImage" @save-image="handleSaveLocalImage"
        />

        <ChatSupervisionTaskDialog
          v-if="!sidebarMode"
          :open="supervisionDialogOpen" :saving="supervisionTaskSaving" :error-text="supervisionTaskError"
          :active-task="activeSupervisionTask" :recent-history="recentSupervisionTaskHistory"
          @close="$emit('closeSupervisionTask')" @save="$emit('saveSupervisionTask', $event)"
          @stop="$emit('stopSupervisionTask')"
        />
        <ToolReviewTargetDialog
          v-if="!sidebarMode"
          :open="codeReviewDialogOpen"
          :submitting="!!toolReviewSubmittingBatchKey"
          :error-text="codeReviewErrorText"
          :current-department-id="props.currentDepartmentId"
          :department-options="props.createConversationDepartmentOptions"
          :commit-options="commitOptions"
          :commit-options-loading="commitOptionsLoading"
          :commit-total="commitTotal"
          :commit-page="commitPage"
          :commit-page-size="commitPageSize"
          @close="closeCodeReviewDialog"
          @pick-commit-review="loadCodeReviewCommitOptions"
          @review-code="handleSubmitCodeReview"
        />
        <ChatTaskCreateDialog
          v-if="!sidebarMode"
          :open="taskDialogOpen"
          :mode="taskDialogMode"
          :conversation-id="activeConversationId"
          :task="taskDialogTask"
          :bridge-request="bridgeRequest"
          @close="closeTaskDialog"
          @created="handleTaskCreated"
          @updated="handleTaskUpdated"
        />
      </div>

    <div
      v-if="leftPaneOverlay || rightPaneOverlay"
      class="absolute inset-0 z-40 bg-base-300/20 backdrop-blur-[1px]"
      @click="closeOverlayPanes"
    ></div>

    <div
      v-if="collapsePreviewSide === 'left'"
      class="pointer-events-none absolute bottom-0 left-0 top-0 z-58 flex items-center justify-center border-r border-error/20 bg-error/12 backdrop-blur-[1px]"
      :style="{ width: `${collapsePreviewWidth}px` }"
    >
      <div class="rounded-full border border-error/25 bg-base-100/90 px-3 py-1.5 text-sm font-semibold text-error shadow-sm">
        {{ t("common.collapse") }}
      </div>
    </div>

    <div
      v-if="collapsePreviewSide === 'right'"
      class="pointer-events-none absolute bottom-0 right-0 top-0 z-58 flex items-center justify-center border-l border-error/20 bg-error/12 backdrop-blur-[1px]"
      :style="{ width: `${collapsePreviewWidth}px` }"
    >
      <div class="rounded-full border border-error/25 bg-base-100/90 px-3 py-1.5 text-sm font-semibold text-error shadow-sm">
        {{ t("common.collapse") }}
      </div>
    </div>

      <div v-if="effectiveToolReviewPanelOpen"
        :class="rightPaneInLayout ? 'flex h-full min-h-0 shrink-0 border-l border-base-300 bg-base-100' : 'absolute bottom-0 right-0 top-0 z-50 flex h-full min-h-0 border-l border-base-300 bg-base-100 shadow-2xl'"
        :style="{ width: `${rightPaneVisibleWidth}px` }">
        <FileReaderPanel
          v-if="chatRightPanelMode === 'reader'"
          ref="chatReaderPanelRef"
          class="h-full w-full"
          :initial-root-path="currentWorkspaceRootPath"
          :bridge-request="bridgeRequest"
          :session-key="chatFileReaderSessionKey"
          :legacy-session-key="legacyChatFileReaderSessionKey"
          :enable-global-drop="false"
          :markdown-is-dark="markdownIsDark"
          custom-markstream-id="chat-file-reader-markstream"
          @capture-context-reference="handleCaptureFileReaderContextReference"
          @clear-selection-context-reference="handleClearFileReaderSelectionContextReference"
        >
          <template #empty>
            <div class="space-y-2 px-5 text-center">
              <div class="font-medium text-base-content/70">选择文件开始阅读</div>
              <div class="text-xs leading-relaxed text-base-content/50">右侧目录会跟随当前会话工作区，也可以通过文件标签页同时阅读多个文件。</div>
            </div>
          </template>
        </FileReaderPanel>
        <ToolReviewSidebar v-else ref="toolReviewSidebarRef" class="w-full"
          :batches="toolReviewBatches" :current-batch-key="toolReviewCurrentBatchKey"
          :detail-map="toolReviewDetailMap" :detail-loading-call-id="toolReviewDetailLoadingCallId"
          :reviewing-call-id="toolReviewReviewingCallId" :batch-reviewing-key="toolReviewBatchReviewingKey"
          :error-text="toolReviewErrorText"
          :markdown-is-dark="markdownIsDark"
          :current-workspace-name="currentWorkspaceName" :current-workspace-root-path="currentWorkspaceRootPath"
          :workspaces="workspaces" :current-department-id="currentDepartmentId"
          :department-options="toolReviewDepartmentOptions"
          :delegate-statuses="delegateStatuses"
          :delegate-statuses-error-text="delegateStatusesErrorText"
          :persona-avatar-url-map="personaAvatarUrlMap"
          :bridge-request="bridgeRequest"
          @select-batch="setToolReviewCurrentBatchKey" @load-item-detail="loadToolReviewItemDetail"
          @review-item="runToolReviewForCall" @review-batch="runToolReviewForBatch"
          @open-delegate-detail="openDelegateArchiveDetail"
          @abort-delegate="abortDelegate"
        />
      </div>
    </div>

    <div
      v-if="showSideConversationList && !detachedChatWindow"
      class="ecall-pane-splitter ecall-pane-splitter-left absolute bottom-0 top-0 z-60"
      :class="{ 'ecall-pane-splitter-active': activePaneResizeSide === 'left' }"
      :style="{ left: `${leftPaneVisibleWidth - 2}px` }"
      role="separator"
      tabindex="0"
      aria-orientation="vertical"
      :aria-valuemin="PANE_WIDTH_LIMITS.left.min"
      :aria-valuemax="PANE_WIDTH_LIMITS.left.max"
      :aria-valuenow="leftPaneVisibleWidth"
      @pointerdown="startPaneResize('left', $event)"
      @keydown.left.prevent="adjustPaneWidthByKeyboard('left', -24)"
      @keydown.right.prevent="adjustPaneWidthByKeyboard('left', 24)"
    ></div>

    <div
      v-if="effectiveToolReviewPanelOpen"
      class="ecall-pane-splitter ecall-pane-splitter-right absolute bottom-0 top-0 z-60"
      :class="{ 'ecall-pane-splitter-active': activePaneResizeSide === 'right' }"
      :style="{ right: `${rightPaneVisibleWidth - 2}px` }"
      role="separator" tabindex="0" aria-orientation="vertical"
      :aria-valuemin="PANE_WIDTH_LIMITS.right.min" :aria-valuemax="PANE_WIDTH_LIMITS.right.max"
      :aria-valuenow="rightPaneVisibleWidth"
      @pointerdown="startPaneResize('right', $event)"
      @keydown.left.prevent="adjustPaneWidthByKeyboard('right', 24)"
      @keydown.right.prevent="adjustPaneWidthByKeyboard('right', -24)"
    ></div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, toRef, watch, type ComponentPublicInstance, type Ref } from "vue";
import { useI18n } from "vue-i18n";
import { isDarkAppTheme } from "../../shell/composables/use-app-theme";
import { ChevronsDown, History, X } from "@lucide/vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { ApiConfigItem, ChatConversationOverviewItem, ChatMentionEntry, ChatMentionTarget, ChatMessageBlock, ChatPersonaPresenceChip, ChatTodoItem, ConversationDelegateStatusSummary, ConversationForwardTarget, IdeContextReferenceItem, IdeContextWorkspaceGroup, PromptCommandPreset, RemoteImContactConversationOption, ShellWorkspace } from "../../../types/app";
import ChatMessageItem from "../components/ChatMessageItem.vue";
import ChatApprovalPanel from "../components/ChatApprovalPanel.vue";
import ChatComposerPanel from "../components/ChatComposerPanel.vue";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";
import ChatConversationSidebar from "../components/ChatConversationSidebar.vue";
import ChatWorkspaceToolbar from "../components/ChatWorkspaceToolbar.vue";
import ToolReviewSidebar from "../components/ToolReviewSidebar.vue";
import ToolReviewTargetDialog from "../components/ToolReviewTargetDialog.vue";
import FileReaderPanel from "../../file-reader/components/FileReaderPanel.vue";
import ChatImagePreviewDialog from "../components/dialogs/ChatImagePreviewDialog.vue";
import ChatSupervisionTaskDialog from "../components/dialogs/ChatSupervisionTaskDialog.vue";
import ChatTaskCreateDialog from "../components/dialogs/ChatTaskCreateDialog.vue";
import ConversationTodoDropdown from "../components/ConversationTodoDropdown.vue";
import CompactionSummaryCard from "../components/CompactionSummaryCard.vue";
import ConversationAutoPushCard from "../components/ConversationAutoPushCard.vue";
import { useChatImagePreview } from "../composables/use-chat-image-preview";
import { useChatMessageActions } from "../composables/use-chat-message-actions";
import { useChatScrollLayout } from "../composables/use-chat-scroll-layout";
import type { TerminalApprovalConversationItem } from "../../shell/composables/use-terminal-approval";
import { isAbsoluteLocalPath, normalizeLocalLinkHref, parseLocalFileReference } from "../utils/local-link";
import { type ChatRenderItem, isRightAlignedMessage, canOpenInFileReader, fileExtensionFromPath } from "../utils/chat-render";
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
import type { ToolReviewCodeReviewScope, ToolReviewCommitOption } from "../composables/use-chat-tool-review";
import { useChatBlockTracking } from "../composables/use-chat-block-tracking";
import type { TaskEntry } from "../../config/views/config-tabs/task-editor";
import type { DepartmentPersonaOption } from "../../shared/department-persona-options";

// ==================== props / emits ====================

const props = defineProps<{
  userAlias: string; personaName: string; userAvatarUrl: string; assistantAvatarUrl: string;
  personaNameMap: Record<string, string>; personaAvatarUrlMap: Record<string, string>;
  mentionEntries: ChatMentionEntry[]; selectedMentions: ChatMentionTarget[];
  latestUserText: string; latestUserImages: Array<{ mime: string; bytesBase64: string }>;
  latestAssistantText: string;
  frontendRoundPhase: "idle" | "queued" | "waiting" | "streaming";
  toolStatusText: string; toolStatusState: "running" | "done" | "failed" | "";
  chatErrorText: string; clipboardImages: Array<{ mime: string; bytesBase64: string }>;
  queuedAttachmentNotices: Array<{ id: string; fileName: string; relativePath: string; mime: string }>;
  chatInput: string; instructionPresets: PromptCommandPreset[]; chatInputPlaceholder: string;
  canRecord: boolean; recording: boolean; recordingMs: number; transcribing: boolean; recordHotkey: string;
  conversationCallPrimaryApiConfigId: string; preferredChatModelId?: string; toolReviewRefreshTick: number; chatModelOptions: ApiConfigItem[];
  planModeEnabled: boolean; chatUsagePercent: number; trimTip: string;
  mediaDragActive: boolean; chatting: boolean; trimming: boolean; trimmingConversationId?: string;
  compactingConversation: boolean; compactingConversationId?: string;
  conversationBusy: boolean; frozen: boolean; messageBlocks: ChatMessageBlock[];
  hasMoreHistory: boolean; loadingOlderHistory: boolean;
  latestOwnMessageAlignRequest: number; conversationScrollToBottomRequest: number;
  currentWorkspaceName: string; currentWorkspaceRootPath: string; workspaces: ShellWorkspace[];
  currentDepartmentId: string; activeConversationId: string; currentTodos: ChatTodoItem[];
  supervisionActive: boolean; supervisionTitle: string; supervisionDialogOpen: boolean;
  supervisionTaskSaving: boolean; supervisionTaskError: string;
  activeSupervisionTask: { taskId: string; goal: string; why: string; todo: string; endAtLocal: string; remainingHours: number } | null;
  recentSupervisionTaskHistory: Array<{ goal: string; why: string; todo: string; durationHours: number }>;
  currentTheme: string; unarchivedConversationItems: ChatConversationOverviewItem[];
  remoteImContactConversations: RemoteImContactConversationOption[];
  conversationItems?: ChatConversationOverviewItem[]; sideConversationListVisible: boolean;
  initialToolReviewPanelOpen: boolean;
  conversationListTab: "local" | "contact" | "task";
  chatLeftPanelMode: "local" | "contact" | "task";
  chatRightPanelMode: "reader" | "review" | "delegate";
  createConversationDepartmentOptions: DepartmentPersonaOption[];
  defaultCreateConversationDepartmentId: string;
  ideContextGroups: IdeContextWorkspaceGroup[]; attachedIdeContextReferences: IdeContextReferenceItem[];
  detachedChatWindow?: boolean; terminalApprovals?: TerminalApprovalConversationItem[];
  terminalApprovalResolving?: boolean;
  sidebarMode?: boolean;
  bridgeMode?: boolean;
  openLocalFilesInHost?: boolean;
  bridgeRequest?: <T = unknown>(method: string, params?: Record<string, unknown>, timeoutMs?: number) => Promise<T>;
  systemNotificationMode?: boolean;
  hideWorkspaceButton?: boolean;
  workspaceAccess?: "read_only" | "approval" | "full_access" | "";
  readPlanFileContent?: (input: { conversationId: string; path: string }) => Promise<string>;
}>();

const emit = defineEmits<{
  (e: "update:chatInput", value: string): void;
  (e: "addMention", value: ChatMentionTarget): void;
  (e: "removeMention", value: string | { agentId: string; departmentId?: string }): void;
  (e: "sideConversationListVisibleChange", value: boolean): void;
  (e: "toolReviewPanelOpenChange", value: boolean): void;
  (e: "sidePanelWidthsChange", value: { leftWidth: number; rightWidth: number }): void;
  (e: "sidePanelWidthsCommit", value: { leftWidth: number; rightWidth: number }): void;
  (e: "update:conversation-list-tab", value: "local" | "contact" | "task"): void;
  (e: "update:chatLeftPanelMode", value: "local" | "contact" | "task"): void;
  (e: "update:chatRightPanelMode", value: "reader" | "review" | "delegate"): void;
  (e: "removeClipboardImage", index: number): void;
  (e: "removeQueuedAttachmentNotice", index: number): void;
  (e: "startRecording"): void; (e: "stopRecording"): void; (e: "pickAttachments"): void;
  (e: "update:conversationPreferredApiConfigId", value: string): void;
  (e: "updateWorkspaceAccess", value: "read_only" | "approval" | "full_access"): void;
  (e: "update:planModeEnabled", value: boolean): void;
  (e: "sendChat", payload?: { extraTextBlocks?: string[] }): void;
  (e: "stopChat"): void; (e: "trimConversation"): void; (e: "openConversationList"): void; (e: "openSettings"): void;
  (e: "clearChatError"): void;
  (e: "recallTurn", payload: { turnId: string }): void;
  (e: "regenerateTurn", payload: { turnId: string }): void;
  (e: "confirmPlan", payload: { messageId: string }): void;
  (e: "lockWorkspace"): void; (e: "openSupervisionTask"): void; (e: "openCodeReview"): void;
  (e: "detachConversation"): void; (e: "closeSupervisionTask"): void;
  (e: "saveSupervisionTask", payload: { durationHours: number; goal: string; why: string; todo: string }): void;
  (e: "stopSupervisionTask"): void;
  (e: "taskCreated", task: TaskEntry): void;
  (e: "taskUpdated", task: TaskEntry): void;
  (e: "switchConversation", payload: { conversationId: string; kind?: "local_unarchived" | "remote_im_contact"; remoteContactId?: string }): void;
  (e: "renameConversation", payload: { conversationId: string; title: string }): void;
  (e: "togglePinConversation", conversationId: string): void;
  (e: "archiveConversation", conversationId: string): void;
  (e: "exportConversation", conversationId: string): void;
  (e: "deleteConversation", conversationId: string): void;
  (e: "createConversation", input?: { title?: string; departmentId?: string; agentId?: string; copyCurrent?: boolean; importPath?: string; shellWorkspaces?: ShellWorkspace[]; shellAutonomousMode?: boolean }): void;
  (e: "loadOlderHistory"): void; (e: "reachedBottom"): void;
  (e: "jumpToConversationBottom"): void;
  (e: "refreshToolReviewMessage", payload: { conversationId: string; messageId: string }): void;
  (e: "selectionActionCopy", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[] }): void;
  (e: "selectionActionCopyError", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; error: string }): void;
  (e: "selectionActionBranch", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[] }): void;
  (e: "selectionActionForward", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; target: ConversationForwardTarget }): void;
  (e: "selectionActionDelegate", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; departmentId: string; agentId: string; presetId: string; why: string; goal: string; todo: string }): void;
  (e: "selectionActionShare", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; exportFormat?: "html" | "png" }): void;
  (e: "approveTerminalApproval", requestId: string): void;
  (e: "denyTerminalApproval", requestId: string): void;
  (e: "openSidebarFileReference", href: string): void;
  (e: "openSidebarExternalUrl", url: string): void;
}>();

// ==================== basic state ====================

const { t } = useI18n();
const toolReviewSidebarRef = ref<ComponentPublicInstance<{
  openDelegatesTab: () => void;
}> | null>(null);
const chatReaderPanelRef = ref<InstanceType<typeof FileReaderPanel> | null>(null);
const chatScrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);
const linkOpenErrorText = ref("");
const conversationSummaryCard = ref<{ visible: boolean; text: string }>({ visible: false, text: "" });
const composerPanelRef = ref<{ focusInput: (opts?: FocusOptions) => void } | null>(null);
const taskDialogOpen = ref(false);
const taskDialogMode = ref<"create" | "edit">("create");
const taskDialogTask = ref<TaskEntry | null>(null);
const autoPushCardOpen = ref(false);
const autoPushSaving = ref(false);
const autoPushEnabled = ref(false);
const autoPushSelectedContactId = ref("");
const codeReviewDialogOpen = ref(false);
const codeReviewErrorText = ref("");
const commitOptions = ref<ToolReviewCommitOption[]>([]);
const commitOptionsLoading = ref(false);
const commitTotal = ref(0);
const commitPage = ref(1);
const commitPageSize = ref(30);

// ==================== context computed ====================

const {
  markdownIsDark, normalizedConversationTodos,
  activeConversationSummary, isCurrentConversationCompacting,
  activeConversationTerminalApprovals, supervisionButtonTitle,
  isOrganizingContextBusy, chatStatusBanner, selectedMentionKeys,
  latestPendingPlanMessageId,
} = useChatConversationCtx(props, isDarkAppTheme, t);
const conversationInteractionBusy = computed(() =>
  props.conversationBusy || isOrganizingContextBusy.value,
);
const activeConversationIsSystemNotification = computed(() =>
  !!props.systemNotificationMode || !!activeConversationSummary.value?.isSystemNotificationConversation,
);
const activeConversationIsRemoteContact = computed(() =>
  activeConversationSummary.value?.kind === 'remote_im_contact',
);

watch(
  () => ({
    activeConversationId: props.activeConversationId,
    propSystemNotificationMode: props.systemNotificationMode,
    summarySystemNotificationMode: activeConversationSummary.value?.isSystemNotificationConversation,
    effectiveSystemNotificationMode: activeConversationIsSystemNotification.value,
    activeConversationSummary: activeConversationSummary.value,
  }),
  (snapshot) => {
    if (!props.sidebarMode) return;
    // 系统会话识别日志已移除
  },
  { immediate: true, deep: true },
);

const toolReviewDepartmentOptions = computed(() =>
  // 用户主动发起代码审查不受 AI delegate 工具的“直接下级部门”限制。
  (Array.isArray(props.createConversationDepartmentOptions) ? props.createConversationDepartmentOptions : []),
);

const departmentNameMap = computed<Record<string, string>>(() => {
  const map: Record<string, string> = {};
  for (const option of props.createConversationDepartmentOptions || []) {
    const departmentId = String(option.departmentId || "").trim();
    if (!departmentId || map[departmentId]) continue;
    map[departmentId] = String(option.departmentName || option.name || departmentId).trim() || departmentId;
  }
  return map;
});

const departmentNameMapSignature = computed(() =>
  Object.entries(departmentNameMap.value)
    .map(([id, name]) => `${id}:${name}`)
    .sort()
    .join("|"),
);

const chatFileReaderSessionKey = computed(() => {
  const conversationId = String(props.activeConversationId || "").trim();
  return conversationId ? `easy_call.chat_file_reader_session.${conversationId}.v1` : "";
});

const legacyChatFileReaderSessionKey = computed(() => {
  const conversationId = String(props.activeConversationId || "").trim();
  return conversationId ? `easy-call.chat.file-reader-session.${conversationId}` : "";
});

// ==================== messages / audio / bubble ====================

const { playingAudioId, copyMessage, stopAudioPlayback, toggleAudioPlayback } = useChatMessageActions();
const { isHidden: isBubbleBackgroundHidden, canToggle: canToggleBubbleBackground, toggle: toggleBubbleBackground } = useBubbleBackground(toRef(props, "activeConversationId"));
const showSideConversationList = computed(() => !!props.sideConversationListVisible);
const sidebarMode = computed(() => !!props.sidebarMode);
const bridgeMode = computed(() => !!props.bridgeMode);
const openLocalFilesInHost = computed(() => !!props.openLocalFilesInHost);

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

const messageSelectionDelegateOnly = ref(false);

function openSelectionMenu(options: { delegateOnly?: boolean } = {}) {
  if (props.chatting || props.frozen || conversationInteractionBusy.value) return;
  messageSelectionDelegateOnly.value = !!options.delegateOnly;
  messageSelectionModeEnabled.value = true;
  selectedMessageRenderIds.value = [];
  void nextTick(() => composerPanelRef.value?.focusInput?.({ preventScroll: true }));
}
const openBranchSelectionMenu = () => openSelectionMenu();
const openDelegateSelectionMenu = () => openSelectionMenu({ delegateOnly: true });
const openForwardSelectionMenu = () => openSelectionMenu();
const openShareSelectionMenu = () => openSelectionMenu();

function openTaskCreateDialog() {
  if (props.chatting || props.frozen || conversationInteractionBusy.value) return;
  taskDialogMode.value = "create";
  taskDialogTask.value = null;
  taskDialogOpen.value = true;
}

function openTaskEditDialog(task: TaskEntry) {
  taskDialogMode.value = "edit";
  taskDialogTask.value = task;
  taskDialogOpen.value = true;
}

function closeTaskDialog() {
  taskDialogOpen.value = false;
}

function handleTaskCreated(task: TaskEntry) {
  taskDialogOpen.value = false;
  emit("taskCreated", task);
}

function handleTaskUpdated(task: TaskEntry) {
  taskDialogOpen.value = false;
  emit("taskUpdated", task);
}

function openConversationSummary(block: ChatMessageBlock, event?: MouseEvent) {
  event?.stopPropagation();
  const text = String(block?.text || "").trim();
  if (!text) return;
  conversationSummaryCard.value = { visible: true, text };
}
function closeConversationSummaryCard() {
  conversationSummaryCard.value = { visible: false, text: "" };
}

function openAutoPushCard() {
  const currentTargetId = String(activeConversationSummary.value?.autoPushRemoteContactId || "").trim();
  autoPushEnabled.value = !!currentTargetId;
  autoPushSelectedContactId.value = currentTargetId;
  autoPushCardOpen.value = true;
}

function closeAutoPushCard() {
  autoPushCardOpen.value = false;
  autoPushSaving.value = false;
}

async function saveAutoPushCard() {
  const conversationId = String(props.activeConversationId || "").trim();
  if (!conversationId || autoPushSaving.value) return;
  autoPushSaving.value = true;
  try {
    await invokeTauri("set_conversation_auto_push_remote_contact", {
      input: {
        conversationId,
        remoteContactId: autoPushEnabled.value
          ? String(autoPushSelectedContactId.value || "").trim() || null
          : null,
      },
    });
    autoPushCardOpen.value = false;
  } finally {
    autoPushSaving.value = false;
  }
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
  enabled: computed(() => !sidebarMode.value),
});

const fileReaderVisibleContextReference = ref<IdeContextReferenceItem | null>(null);
const fileReaderSelectionContextReference = ref<IdeContextReferenceItem | null>(null);
const fileReaderContextReferences = computed<IdeContextReferenceItem[]>(() => {
  const visible = fileReaderVisibleContextReference.value;
  const selection = fileReaderSelectionContextReference.value;
  if (!visible && !selection) return [];
  if (!visible) return selection ? [selection] : [];
  if (!selection) return [visible];
  const visibleFilePath = String(visible.filePath || "").trim();
  const selectionFilePath = String(selection.filePath || "").trim();
  return visibleFilePath && visibleFilePath === selectionFilePath ? [selection] : [visible];
});
const mergedVisibleIdeContextGroups = computed<IdeContextWorkspaceGroup[]>(() => {
  const propGroups = Array.isArray(props.ideContextGroups) ? props.ideContextGroups : [];
  const baseGroups = propGroups.length > 0 ? propGroups : visibleIdeContextGroups.value;
  if (fileReaderContextReferences.value.length === 0) return baseGroups;
  return [
    {
      workspacePath: String(props.currentWorkspaceRootPath || "").trim(),
      workspaceName: String(props.currentWorkspaceName || "").trim() || t("chat.allowedWorkspaceButton"),
      references: fileReaderContextReferences.value,
    },
    ...baseGroups,
  ];
});

function handleCaptureFileReaderContextReference(reference: IdeContextReferenceItem) {
  const source = String(reference.source || "").trim();
  if (source === "visible_range") {
    fileReaderVisibleContextReference.value = { ...reference };
  } else {
    fileReaderSelectionContextReference.value = { ...reference };
  }
}

function handleClearFileReaderSelectionContextReference() {
  fileReaderSelectionContextReference.value = null;
}

watch(() => props.activeConversationId, () => {
  fileReaderVisibleContextReference.value = null;
  fileReaderSelectionContextReference.value = null;
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
  chatting: toRef(props, "chatting"), conversationBusy: conversationInteractionBusy,
  frozen: toRef(props, "frozen"), messageSelectionModeEnabled,
  selectedMessageRenderIdSet,
  isBubbleBackgroundHidden, canToggleBubbleBackground, canRegenerateBlock, canConfirmPlan,
});

const virtualRenderItems = computed<ChatRenderItem[]>(() => [...chatRenderItems.value]);

// ==================== block tracking ====================

const { isOwnMessage, latestOwnMessageId, latestOwnElasticItemId } =
  useChatBlockTracking(toRef(props, "messageBlocks"), chatRenderItems);

// ==================== selection mode ====================

const {
  selectedMessageBlocks, enterMessageSelectionMode, toggleMessageSelected,
  exitMessageSelectionMode: resetMessageSelectionMode, copySelectedMessages, emitSelectionAction,
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

function handleEnterMessageSelectionMode(selectionKey: string) {
  messageSelectionDelegateOnly.value = false;
  enterMessageSelectionMode(selectionKey);
}

function handleExitMessageSelectionMode() {
  messageSelectionDelegateOnly.value = false;
  resetMessageSelectionMode();
}

defineExpose({ exitMessageSelectionMode: handleExitMessageSelectionMode });

// ==================== scroll layout ====================

const {
  scrollContainer, composerContainer, toolbarContainer, chatLayoutRoot,
  latestOwnElasticMinHeight, showJumpToBottom, jumpToBottomStyle, onScroll,
  prepareBottomAlignmentLayout,
} = useChatScrollLayout({
  activeConversationId: toRef(props, "activeConversationId"),
  chatting: toRef(props, "chatting"), busy: conversationInteractionBusy,
  frozen: toRef(props, "frozen"),
  messageBlockCount: computed(() => props.messageBlocks.length),
  onReachedBottom: () => emit("reachedBottom"),
  focusComposerInput: (options) => composerPanelRef.value?.focusInput(options),
});

// ==================== virtual scroll ====================

const {
  virtualizer, virtualEntries, totalVirtualSize, measureVirtualRow,
  latestOwnTailContentHeight, scheduleVirtualMeasure, syncViewportMetrics,
  resetVirtualizerAtConversationBottom, alignItemToTop, refreshObservedVirtualItemElements,
} = useChatVirtualScroll({
  renderItems: virtualRenderItems,
  scrollContainer, scrollbarRef: chatScrollbarRef as Ref<{ updateThumb: () => void } | null>,
  activeConversationId: toRef(props, "activeConversationId"),
  latestOwnElasticItemId,
  latestOwnElasticMinHeight,
  debugEnabled: computed(() => !sidebarMode.value),
  smoothScrollEnabled: computed(() => !sidebarMode.value),
  onUserScroll: () => onScroll(),
});

const latestOwnTailSpacerMinHeight = computed(() => {
  if (!latestOwnElasticItemId.value || latestOwnTailContentHeight.value <= 0) return 0;
  return Math.max(0, latestOwnElasticMinHeight.value - latestOwnTailContentHeight.value);
});

// ==================== tool review ====================

const {
  toolReviewPanelOpen, toolReviewBatches, toolReviewCurrentBatchKey,
  toolReviewDetailMap, toolReviewDetailLoadingCallId, toolReviewReviewingCallId,
  toolReviewBatchReviewingKey, toolReviewSubmittingBatchKey, toolReviewErrorText,
  setToolReviewCurrentBatchKey,
  loadToolReviewItemDetail, runToolReviewForCall, runToolReviewForBatch,
  submitToolReviewCode, listToolReviewCommitOptions,
} = useChatToolReviewHandlers({
  activeConversationId: toRef(props, "activeConversationId"),
  toolReviewRefreshTick: toRef(props, "toolReviewRefreshTick"),
  currentDepartmentId: toRef(props, "currentDepartmentId"),
  departmentOptions: toolReviewDepartmentOptions,
  initialPanelOpen: toRef(props, "initialToolReviewPanelOpen"),
  bridgeRequest: toRef(props, "bridgeRequest"),
  t, syncViewportMetrics,
  onRefreshMessage: (payload) => emit("refreshToolReviewMessage", payload),
  onToolReviewPanelOpenChange: (open) => emit("toolReviewPanelOpenChange", open),
});
const effectiveToolReviewPanelOpen = computed(() => !sidebarMode.value && toolReviewPanelOpen.value);

// ==================== delegate status ====================

const {
  delegateStatuses, delegateStatusesErrorText,
  openDelegateArchiveDetail, abortDelegate,
} = useDelegateStatus({
  activeConversationId: toRef(props, "activeConversationId"),
  panelOpen: computed(() => !sidebarMode.value),
  enabled: computed(() => !sidebarMode.value),
  bridgeRequest: toRef(props, "bridgeRequest"),
});

// ==================== panes ====================

const panesCleanupFns: Array<() => void> = [];
const {
  leftPaneInLayout, rightPaneInLayout,
  leftPaneOverlay, rightPaneOverlay, leftPaneVisibleWidth, rightPaneVisibleWidth, activePaneResizeSide,
  collapsePreviewSide, collapsePreviewWidth,
  startPaneResize, adjustPaneWidthByKeyboard,
} = useChatPanes({
  chatLayoutRoot, toolReviewPanelOpen: effectiveToolReviewPanelOpen,
  showSideConversationList, detachedChatWindow: !!props.detachedChatWindow,
  syncViewportMetrics,
  onPaneWidthsChange: (left, right) => emit("sidePanelWidthsChange", { leftWidth: left, rightWidth: right }),
  onPaneWidthsCommit: (left, right) => emit("sidePanelWidthsCommit", { leftWidth: left, rightWidth: right }),
  onPaneCloseRequest: (side) => {
    if (side === "left") {
      emit("sideConversationListVisibleChange", false);
      return;
    }
    emit("toolReviewPanelOpenChange", false);
  },
  onBeforeUnmountCleanup: (fn) => panesCleanupFns.push(fn),
});

function closeOverlayPanes() {
  if (leftPaneOverlay.value) emit("sideConversationListVisibleChange", false);
  if (rightPaneOverlay.value) emit("toolReviewPanelOpenChange", false);
}

// ==================== scroll orchestration ====================

const {
  onConversationScroll, handleJumpToBottom,
  alignLatestOwnMessageToTop,
} = useChatScrollOrchestration({
  scrollContainer, chatScrollbarRef: chatScrollbarRef as Ref<{ updateThumb: () => void; hide?: () => void } | null>,
  prepareBottomAlignmentLayout,
  onScroll, scheduleVirtualMeasure, syncViewportMetrics,
  resetConversationToBottom: resetVirtualizerAtConversationBottom,
  alignItemToTop,
  refreshObservedVirtualItemElements,
  latestOwnElasticItemId,
  props: {
    hasMoreHistory: toRef(props, "hasMoreHistory"), loadingOlderHistory: toRef(props, "loadingOlderHistory"),
    chatting: toRef(props, "chatting"), conversationBusy: conversationInteractionBusy, frozen: toRef(props, "frozen"),
    activeConversationId: toRef(props, "activeConversationId"),
    conversationScrollToBottomRequest: toRef(props, "conversationScrollToBottomRequest"),
    latestOwnMessageAlignRequest: toRef(props, "latestOwnMessageAlignRequest"),
    messageBlocks: toRef(props, "messageBlocks"),
  },
  emit: { loadOlderHistory: () => emit("loadOlderHistory"), jumpToConversationBottom: () => emit("jumpToConversationBottom") },
});

// ==================== image preview ====================

const {
  imagePreviewOpen, imagePreviewDataUrl, imagePreviewLocalPath, imagePreviewZoom,
  IMAGE_PREVIEW_MIN_ZOOM, IMAGE_PREVIEW_MAX_ZOOM,
  previewOffsetX, previewOffsetY, previewDragging,
  zoomInPreview, zoomOutPreview, resetPreviewZoom,
  onPreviewWheel, openImagePreview, closeImagePreview,
  onPreviewPointerDown, onPreviewPointerMove, onPreviewPointerUp,
} = useChatImagePreview();

const imagePreviewCopyStatus = ref<string>('idle');
const imagePreviewSaveStatus = ref<string>('idle');

async function handleCopyLocalImage(path: string) {
  imagePreviewCopyStatus.value = 'doing';
  try {
    await invokeTauri('copy_local_chat_image_to_clipboard', { input: { path } });
  } catch (error) {
    console.warn('[预览] 复制图片失败', error);
  } finally {
    imagePreviewCopyStatus.value = 'idle';
  }
}

async function handleSaveLocalImage(path: string) {
  imagePreviewSaveStatus.value = 'doing';
  try {
    await invokeTauri('save_local_chat_image_as', { input: { path } });
  } catch (error) {
    console.warn('[预览] 保存图片失败', error);
  } finally {
    imagePreviewSaveStatus.value = 'idle';
  }
}

// ==================== conversation actions ====================

function handleDetachConversationRequest() { emit("detachConversation"); }
function openCodeReviewDialog() {
  codeReviewErrorText.value = "";
  codeReviewDialogOpen.value = true;
}
function closeCodeReviewDialog() {
  if (toolReviewSubmittingBatchKey.value) return;
  codeReviewDialogOpen.value = false;
  codeReviewErrorText.value = "";
}
async function loadCodeReviewCommitOptions(page = 1) {
  const conversationId = String(props.activeConversationId || "").trim();
  if (!conversationId) return;
  commitOptionsLoading.value = true;
  try {
    const result = await listToolReviewCommitOptions(conversationId, page, commitPageSize.value);
    commitOptions.value = Array.isArray(result.commits) ? result.commits : [];
    commitTotal.value = Number(result.total || 0);
    commitPage.value = Number(result.page || page);
    commitPageSize.value = Number(result.pageSize || commitPageSize.value);
    codeReviewErrorText.value = "";
  } catch (error) {
    commitOptions.value = [];
    codeReviewErrorText.value = t("chat.readCommitFailed");
    console.error("[代码审查] 读取 commit 失败", error);
  } finally {
    commitOptionsLoading.value = false;
  }
}
async function handleSubmitCodeReview(input: { scope: ToolReviewCodeReviewScope; target?: string; departmentId: string }) {
  const conversationId = String(props.activeConversationId || "").trim();
  if (!conversationId || toolReviewSubmittingBatchKey.value) return;
  codeReviewErrorText.value = "";
  const report = await submitToolReviewCode({
    conversationId,
    scope: input.scope,
    target: String(input.target || "").trim() || undefined,
    departmentId: String(input.departmentId || "").trim() || undefined,
  });
  if (!report) {
    codeReviewErrorText.value = t("chat.startCodeReviewFailed");
    return;
  }
  codeReviewDialogOpen.value = false;
}
function openDelegateSummaryPanel() {
  emit("update:chatRightPanelMode", "delegate");
  emit("toolReviewPanelOpenChange", true);
  void nextTick(() => toolReviewSidebarRef.value?.openDelegatesTab());
}
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
function handleConversationExport(id: string) { emit("exportConversation", String(id || "").trim()); }
function handleConversationDelete(id: string) { emit("deleteConversation", String(id || "").trim()); }

function handleShiftWheel(event: WheelEvent) {
  if (!event.shiftKey) return;
  event.preventDefault();
  const items = props.conversationItems || props.unarchivedConversationItems;
  if (!items || items.length === 0) return;
  const currentId = String(props.activeConversationId || "").trim();
  const currentIndex = items.findIndex((item) => String(item.conversationId || "").trim() === currentId);
  if (currentIndex < 0) return;
  const direction = event.deltaY > 0 ? 1 : -1;
  let step = direction;
  // 沿方向搜索第一个可切换的会话（跳过当前和不可用）
  const maxSteps = items.length;
  while (Math.abs(step) <= maxSteps) {
    const targetIndex = currentIndex + step;
    if (targetIndex < 0 || targetIndex >= items.length) break;
    const target = items[targetIndex];
    const openState = String(target.state?.openState || "").trim();
    const openViewerId = String(target.state?.openViewerId || "").trim();
    const currentViewerId = String(target.state?.currentViewerId || "").trim();
    const openedByAnotherViewer = !target.isSystemNotificationConversation
      && openState === "open"
      && !!openViewerId
      && !!currentViewerId
      && openViewerId !== currentViewerId;
    const isDisabled = target.runtimeState === "organizing_context"
      || target.runtimeState === "archiving"
      || target.runtimeState === "compacting"
      || !!target.detachedWindowOpen
      || openedByAnotherViewer;
    if (!isDisabled) {
      emit("switchConversation", {
        conversationId: String(target.conversationId || "").trim(),
        kind: target.kind,
        remoteContactId: String(target.remoteContactId || "").trim() || undefined,
      });
      break;
    }
    step += direction;
  }
}

// ==================== link / copy ====================

async function handleAssistantLinkClick(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  const localImage = target?.closest("[data-local-image-path]") as HTMLElement | null;
  if (localImage) {
    const rawPath = localImage.getAttribute("data-local-image-path") || "";
    let path = normalizeLocalLinkHref(rawPath);
    if (!path) return;
    if (!isAbsoluteLocalPath(path)) {
      const root = String(props.currentWorkspaceRootPath || "").trim().replace(/\\/g, "/").replace(/\/$/, "");
      if (root) path = `${root}/${path.replace(/^\.\//, "")}`;
    }
    event.preventDefault(); event.stopPropagation();
    if (bridgeMode.value && openLocalFilesInHost.value) {
      emit("openSidebarFileReference", path);
      return;
    }
    try {
      const result = await invokeTauri<{ dataUrl: string; mime: string }>("read_local_chat_image_original", {
        input: { path },
      });
      const dataUrl = String(result?.dataUrl || "").trim();
      if (!dataUrl) return;
      openImagePreview({ mime: result.mime, dataUrl, localPath: path });
      linkOpenErrorText.value = "";
    } catch (error) {
      linkOpenErrorText.value = t("status.openLinkFailed", { err: String(error) });
    }
    return;
  }
  const anchor = target?.closest("a") as HTMLAnchorElement | null;
  if (!anchor) return;
  const rawHref = anchor.getAttribute("data-href") || anchor.getAttribute("href")?.trim() || "";
  let href = normalizeLocalLinkHref(rawHref);
  if (!href || href === "#") return;
  // 相对路径：基于当前工作目录解析为绝对路径
  if (!isAbsoluteLocalPath(href) && !href.startsWith("http://") && !href.startsWith("https://")) {
    const root = String(props.currentWorkspaceRootPath || "").trim().replace(/\\/g, "/").replace(/\/$/, "");
    if (root) {
      href = `${root}/${href}`;
    }
  }
  const localReference = parseLocalFileReference(href);
  const localPath = localReference?.path || href;
  if (isAbsoluteLocalPath(localPath)) {
    event.preventDefault(); event.stopPropagation();
    if (bridgeMode.value && openLocalFilesInHost.value) {
      emit("openSidebarFileReference", href);
      return;
    }
    try {
      if (canOpenInFileReader(localPath) || !fileExtensionFromPath(localPath)) { await openLocalFileInChatReader(localPath); }
      else { await invokeTauri("open_local_file_directory", { path: localPath }); }
      linkOpenErrorText.value = "";
    } catch (error) { linkOpenErrorText.value = t("status.openLinkFailed", { err: String(error) }); }
    return;
  }
  if (href.startsWith("http://") || href.startsWith("https://")) {
    event.preventDefault(); event.stopPropagation();
    if (bridgeMode.value) {
      emit("openSidebarExternalUrl", href);
      return;
    }
    try { await invokeTauri("open_external_url", { url: href }); linkOpenErrorText.value = ""; }
    catch (error) { linkOpenErrorText.value = t("status.openLinkFailed", { err: String(error) }); }
  }
}

async function openLocalFileInChatReader(path: string) {
  emit("update:chatRightPanelMode", "reader");
  if (!props.initialToolReviewPanelOpen) {
    emit("toolReviewPanelOpenChange", true);
  }
  await nextTick();
  await nextTick();
  await chatReaderPanelRef.value?.openPath(path);
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
