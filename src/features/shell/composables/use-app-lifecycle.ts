import { onBeforeUnmount, onMounted, type Ref } from "vue";

type UseAppLifecycleOptions = {
  appBootstrapMount: () => Promise<void>;
  appBootstrapUnmount: () => void;
  restoreThemeFromStorage: () => void;
  onPaste: (event: ClipboardEvent) => void;
  onDragOver: (event: DragEvent) => void;
  onDrop: (event: DragEvent) => void;
  onNativeFileDrop: (paths: string[]) => Promise<void> | void;
  onNativeDragState: (active: boolean) => void;
  recordHotkeyMount: () => void;
  recordHotkeyUnmount: () => void;
  refreshAllViewData: () => Promise<void>;
  configAutosaveReady: Ref<boolean>;
  personasAutosaveReady: Ref<boolean>;
  chatSettingsAutosaveReady: Ref<boolean>;
  viewMode: Ref<"chat" | "archives" | "config">;
  syncAlwaysOnTop: () => Promise<void>;
  disposeAutosaveTimers: () => void;
  clearStreamBuffer: () => void;
  stopRecording: (discard: boolean) => Promise<void>;
  cleanupSpeechRecording: () => void;
  cleanupChatMedia: () => Promise<void>;
  afterMountedReady?: () => Promise<void> | void;
};

export function useAppLifecycle(options: UseAppLifecycleOptions) {
  onMounted(async () => {
    await options.appBootstrapMount();
    options.restoreThemeFromStorage();
    window.addEventListener("paste", options.onPaste);
    window.addEventListener("dragover", options.onDragOver);
    window.addEventListener("drop", options.onDrop);
    options.recordHotkeyMount();
    await options.refreshAllViewData();
    options.configAutosaveReady.value = true;
    options.personasAutosaveReady.value = true;
    options.chatSettingsAutosaveReady.value = true;
    if (options.viewMode.value === "chat") {
      await options.syncAlwaysOnTop();
    }
    await options.afterMountedReady?.();
  });

  onBeforeUnmount(() => {
    options.appBootstrapUnmount();
    options.disposeAutosaveTimers();
    options.clearStreamBuffer();
    void options.stopRecording(true);
    options.cleanupSpeechRecording();
    options.recordHotkeyUnmount();
    void options.cleanupChatMedia();
    window.removeEventListener("paste", options.onPaste);
    window.removeEventListener("dragover", options.onDragOver);
    window.removeEventListener("drop", options.onDrop);
  });
}
