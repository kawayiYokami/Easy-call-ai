import { computed } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import { useChatAttachmentActions } from "./use-chat-attachment-actions";
import { useChatAttachmentPickerFlow } from "./use-chat-attachment-picker-flow";
import { useChatMedia } from "./use-chat-media";
import { useChatWindowRecordingOrchestrator } from "./use-chat-window-recording-orchestrator";
import { useSpeechRecording } from "./use-speech-recording";

export function useChatWindowMediaOrchestrator(bindings: Record<string, any>) {
  const {
    supported: speechRecognitionSupported,
    recording,
    recordingMs,
    transcribing,
    startRecording: startSpeechRecording,
    stopRecording: stopSpeechRecording,
    prewarmMicrophone,
    cleanup: cleanupSpeechRecording,
  } = useSpeechRecording({
    t: bindings.tr,
    canStart: () => !bindings.chatting.value && !bindings.trimming.value,
    getLanguage: () => bindings.normalizeLocale(bindings.config.uiLanguage),
    getMinRecordSeconds: () => bindings.config.minRecordSeconds,
    getMaxRecordSeconds: () => bindings.config.maxRecordSeconds,
    shouldUseRemoteStt: () => bindings.shouldUseRemoteStt.value,
    transcribeRemoteStt: async (audio: any) => {
      const sttApiConfigId = bindings.activeSttApiConfig.value?.id;
      if (!sttApiConfigId) throw new Error("No STT API selected.");
      const out = await invokeTauri<{ text: string }>("stt_transcribe", {
        input: {
          sttApiConfigId,
          mime: audio.mime,
          bytesBase64: audio.bytesBase64,
        },
      });
      return String(out.text || "").trim();
    },
    appendRecognizedText: (text: string) => {
      bindings.chatInput.value = bindings.chatInput.value.trim()
        ? `${bindings.chatInput.value.trim()}\n${text}`
        : text;
    },
    onTranscribed: async ({ source, text }: { source: string; text: string }) => {
      const wasBackgroundWake = !recordingWindow.isChatWindowActiveNow();
      if (wasBackgroundWake) {
        const startedAt = Date.now();
        const keywords = bindings.parseBackgroundVoiceScreenshotKeywords(bindings.backgroundVoiceScreenshotKeywords.value);
        const recognizedText = String(text || "").trim();
        const matched = bindings.matchBackgroundVoiceScreenshotKeyword(recognizedText, keywords);
        if (!matched) {
          console.info(
            "[后台语音截图] 跳过：未命中关键词，关键词数=%d，转写长度=%d",
            keywords.length,
            recognizedText.length,
          );
        } else {
          await bindings.queueAutoScreenshotFromVoice({
            source,
            keyword: matched,
            mode: bindings.backgroundVoiceScreenshotMode.value,
            startedAt,
          });
        }
        void invokeTauri("show_chat_window").catch((error) => {
          console.warn("[音频] 打开聊天窗口失败:", error);
        });
      }
      if (source !== "remote") return;
      if (!bindings.config.sttAutoSend) return;
      if (bindings.chatting.value || bindings.trimming.value) return;
      setTimeout(() => {
        bindings.sendChatFromCurrentWindow();
      }, 0);
    },
    setStatus: (text: string) => {
      bindings.status.value = text;
    },
  });

  const recordingWindow = useChatWindowRecordingOrchestrator({
    viewMode: bindings.viewMode,
    config: bindings.config,
    recording,
    tauriWindowLabel: bindings.tauriWindowLabel,
    isChatTauriWindow: bindings.isChatTauriWindow,
    detachedChatWindow: bindings.detachedChatWindow,
    currentChatConversationId: bindings.currentChatConversationId,
    startupDataReady: bindings.startupDataReady,
    recordHotkeyProbeLastSeq: bindings.recordHotkeyProbeLastSeq,
    recordHotkeyProbeDown: bindings.recordHotkeyProbeDown,
    chatWindowActiveSynced: bindings.chatWindowActiveSynced,
    startSpeechRecording,
    stopSpeechRecording,
    prewarmMicrophone,
    refreshChatUnarchivedConversations: bindings.refreshChatUnarchivedConversations,
    freezeForegroundConversation: bindings.freezeForegroundConversation,
    restoreForegroundConversationProjection: bindings.restoreForegroundConversationProjection,
  });

  const chatMedia = useChatMedia({
    t: bindings.tr,
    setStatus: (text: string) => {
      bindings.status.value = text;
    },
    setChatError: (text: string) => {
      bindings.chatErrorText.value = text;
    },
    setStatusError: bindings.setStatusError,
    viewMode: bindings.viewMode,
    chatting: bindings.chatting,
    trimming: bindings.trimming,
    isRecording: () => recording.value,
    activeChatApiConfig: computed(() => bindings.getCurrentForegroundApiConfig()),
    hasVisionFallback: bindings.hasVisionFallback,
    chatInput: bindings.chatInput,
    clipboardImages: bindings.clipboardImages,
    queuedAttachmentNotices: bindings.queuedAttachmentNotices,
  });

  const attachmentPicker = useChatAttachmentPickerFlow({
    chatting: bindings.chatting,
    trimming: bindings.trimming,
    queuedAttachmentNotices: bindings.queuedAttachmentNotices,
    onNativeFileDrop: chatMedia.onNativeFileDrop,
    setStatusError: bindings.setStatusError,
  });

  const attachmentActions = useChatAttachmentActions({
    queueTextAttachment: chatMedia.queueTextAttachment,
    status: bindings.status,
    setStatusError: bindings.setStatusError,
  });

  return {
    speechRecognitionSupported,
    recording,
    recordingMs,
    transcribing,
    cleanupSpeechRecording,
    ...recordingWindow,
    hotkeyTestRecording: chatMedia.hotkeyTestRecording,
    hotkeyTestRecordingMs: chatMedia.hotkeyTestRecordingMs,
    hotkeyTestAudio: chatMedia.hotkeyTestAudio,
    mediaDragActive: chatMedia.mediaDragActive,
    onPaste: chatMedia.onPaste,
    onDragOver: chatMedia.onDragOver,
    onDrop: chatMedia.onDrop,
    onNativeFileDrop: chatMedia.onNativeFileDrop,
    removeClipboardImage: chatMedia.removeClipboardImage,
    startHotkeyRecordTest: chatMedia.startHotkeyRecordTest,
    stopHotkeyRecordTest: chatMedia.stopHotkeyRecordTest,
    playHotkeyRecordTest: chatMedia.playHotkeyRecordTest,
    cleanupChatMedia: chatMedia.cleanupChatMedia,
    removeQueuedAttachmentNotice: attachmentPicker.removeQueuedAttachmentNotice,
    pickChatAttachments: attachmentPicker.pickChatAttachments,
    attachToolReviewReport: attachmentActions.attachToolReviewReport,
  };
}
