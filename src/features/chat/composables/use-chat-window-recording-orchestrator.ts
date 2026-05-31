import { ref, watch, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { AppConfig } from "../../../types/app";
import { useRecordHotkey } from "./use-record-hotkey";

type RecordingActivationSource = "foreground" | "background";

type UseChatWindowRecordingOrchestratorOptions = {
  viewMode: Ref<"chat" | "archives" | "config">;
  config: AppConfig;
  recording: Ref<boolean>;
  tauriWindowLabel: Ref<string>;
  isChatTauriWindow: Ref<boolean>;
  detachedChatWindow: Ref<boolean>;
  currentChatConversationId: Ref<string>;
  startupDataReady: Ref<boolean>;
  recordHotkeyProbeLastSeq: Ref<number>;
  recordHotkeyProbeDown: Ref<boolean>;
  chatWindowActiveSynced: Ref<boolean | null>;
  startSpeechRecording: () => Promise<unknown>;
  stopSpeechRecording: (discard: boolean) => Promise<unknown>;
  prewarmMicrophone: () => Promise<unknown>;
  refreshChatUnarchivedConversations: () => Promise<void>;
  freezeForegroundConversation: (reason: string) => void;
  restoreForegroundConversationProjection: (conversationId: string, reason: string) => Promise<void>;
};

export function useChatWindowRecordingOrchestrator(options: UseChatWindowRecordingOrchestratorOptions) {
  const CHAT_WINDOW_MIC_PREWARM_DEBOUNCE_MS = 260;
  const foregroundRecordingActive = ref(false);
  let chatWindowActiveSyncTimer: ReturnType<typeof setTimeout> | null = null;
  let chatMicPrewarmTimer: ReturnType<typeof setTimeout> | null = null;

  function clearChatWindowActiveSyncTimer() {
    if (!chatWindowActiveSyncTimer) return;
    clearTimeout(chatWindowActiveSyncTimer);
    chatWindowActiveSyncTimer = null;
  }

  function clearChatMicPrewarmTimer() {
    if (!chatMicPrewarmTimer) return;
    clearTimeout(chatMicPrewarmTimer);
    chatMicPrewarmTimer = null;
  }

  async function tryPrewarmChatMic(reason: string) {
    if (options.viewMode.value !== "chat") return;
    if (document.visibilityState === "hidden") return;
    if (!document.hasFocus()) return;
    void reason;
    await options.prewarmMicrophone();
  }

  function isChatWindowActiveNow(): boolean {
    return options.viewMode.value === "chat" && document.visibilityState === "visible" && document.hasFocus();
  }

  async function startRecording(source: RecordingActivationSource = "foreground") {
    if (!options.recording.value) {
      foregroundRecordingActive.value = source === "foreground" && isChatWindowActiveNow();
    }
    await options.startSpeechRecording();
    if (!options.recording.value) {
      foregroundRecordingActive.value = false;
    }
  }

  async function stopRecording(discard: boolean) {
    foregroundRecordingActive.value = false;
    await options.stopSpeechRecording(discard);
  }

  const recordHotkey = useRecordHotkey({
    isActive: () => isChatWindowActiveNow(),
    getRecordHotkey: () => options.config.recordHotkey,
    onStartRecording: () => startRecording("foreground"),
    onStopRecording: (discard) => stopRecording(discard),
    startDelayMs: 0,
  });

  function cancelForegroundRecordingOnBackground(reason: string) {
    void reason;
    if (!foregroundRecordingActive.value) return;
    foregroundRecordingActive.value = false;
    recordHotkey.resetPressedState();
    void options.stopSpeechRecording(true);
  }

  watch(options.recording, (active) => {
    if (!active) {
      foregroundRecordingActive.value = false;
    }
  });

  function isPrimaryChatWindow(): boolean {
    return options.tauriWindowLabel.value === "chat" && !options.detachedChatWindow.value;
  }

  function clearRecordHotkeyProbeState() {
    options.recordHotkeyProbeDown.value = false;
    options.recordHotkeyProbeLastSeq.value = 0;
  }

  function scheduleChatWindowActiveStateSync(reason: string, delayMs = 0) {
    if (!isPrimaryChatWindow()) return;
    clearChatWindowActiveSyncTimer();
    if (delayMs <= 0) {
      syncChatWindowActiveState(reason);
      return;
    }
    chatWindowActiveSyncTimer = setTimeout(() => {
      chatWindowActiveSyncTimer = null;
      syncChatWindowActiveState(reason);
    }, delayMs);
  }

  function scheduleChatMicPrewarm(reason: string, delayMs = 0) {
    clearChatMicPrewarmTimer();
    if (delayMs <= 0) {
      void tryPrewarmChatMic(reason);
      return;
    }
    chatMicPrewarmTimer = setTimeout(() => {
      chatMicPrewarmTimer = null;
      void tryPrewarmChatMic(reason);
    }, delayMs);
  }

  function syncChatWindowActiveState(reason = "unknown") {
    if (!isPrimaryChatWindow()) return;
    const active = isChatWindowActiveNow();
    if (options.chatWindowActiveSynced.value === active) return;
    options.chatWindowActiveSynced.value = active;
    if (active) {
      void stopRecording(false);
      const activeConversationId = String(options.currentChatConversationId.value || "").trim();
      if (activeConversationId) {
        void options.restoreForegroundConversationProjection(activeConversationId, reason)
          .catch((error) => {
            console.warn("[聊天流式恢复] 前台激活同步失败", {
              conversationId: activeConversationId,
              reason,
              error,
            });
          });
      } else if (options.startupDataReady.value) {
        void options.refreshChatUnarchivedConversations()
          .catch((error) => {
            console.warn("[聊天追踪][前台会话] 激活恢复失败", error);
          });
      }
    }
    clearRecordHotkeyProbeState();
    void invokeTauri("set_chat_window_active", { active }).catch((error) => {
      console.warn("[热键] 设置聊天窗口激活状态失败:", error);
    });
  }

  function handleWindowFocusForStateSync() {
    scheduleChatWindowActiveStateSync("focus");
  }

  function handleWindowBlurForStateSync() {
    cancelForegroundRecordingOnBackground("blur");
    scheduleChatWindowActiveStateSync("blur");
  }

  function handleVisibilityForStateSync() {
    clearChatWindowActiveSyncTimer();
    clearChatMicPrewarmTimer();
    if (options.isChatTauriWindow.value && document.visibilityState !== "visible") {
      cancelForegroundRecordingOnBackground("visibility_hidden");
      options.freezeForegroundConversation("window_hidden");
    }
    syncChatWindowActiveState("visibilitychange");
  }

  function handleWindowFocusForMicPrewarm() {
    scheduleChatMicPrewarm("focus", CHAT_WINDOW_MIC_PREWARM_DEBOUNCE_MS);
  }

  function handleVisibilityForMicPrewarm() {
    if (document.visibilityState !== "visible") return;
    scheduleChatMicPrewarm("visibility_visible", CHAT_WINDOW_MIC_PREWARM_DEBOUNCE_MS);
  }

  return {
    recordHotkey,
    foregroundRecordingActive,
    clearChatWindowActiveSyncTimer,
    clearChatMicPrewarmTimer,
    isChatWindowActiveNow,
    startRecording,
    stopRecording,
    cancelForegroundRecordingOnBackground,
    isPrimaryChatWindow,
    clearRecordHotkeyProbeState,
    scheduleChatWindowActiveStateSync,
    scheduleChatMicPrewarm,
    syncChatWindowActiveState,
    handleWindowFocusForStateSync,
    handleWindowBlurForStateSync,
    handleVisibilityForStateSync,
    handleWindowFocusForMicPrewarm,
    handleVisibilityForMicPrewarm,
  };
}
