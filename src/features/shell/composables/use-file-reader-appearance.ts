import { ref } from "vue";
import { emitTransportEvent, onTransportNotification } from "../../../services/tauri-api";

const FILE_READER_LINE_WRAP_STORAGE_KEY = "easy-call.file-reader.line-wrap.v1";
export const FILE_READER_LINE_WRAP_DEFAULT = false;

type FileReaderAppearancePayload = {
  lineWrapEnabled?: unknown;
};

const fileReaderLineWrapEnabled = ref(readStoredLineWrapEnabled());
let initialized = false;
let eventUnlisten: (() => void) | null = null;

export function normalizeFileReaderLineWrap(value: unknown): boolean {
  return typeof value === "boolean" ? value : FILE_READER_LINE_WRAP_DEFAULT;
}

function readStoredLineWrapEnabled(): boolean {
  if (typeof window === "undefined") return FILE_READER_LINE_WRAP_DEFAULT;
  const stored = window.localStorage.getItem(FILE_READER_LINE_WRAP_STORAGE_KEY);
  return stored === null ? FILE_READER_LINE_WRAP_DEFAULT : stored === "1";
}

function persistLineWrapEnabled(enabled: boolean) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(FILE_READER_LINE_WRAP_STORAGE_KEY, enabled ? "1" : "0");
}

function applyLineWrapEnabled(value: unknown) {
  fileReaderLineWrapEnabled.value = normalizeFileReaderLineWrap(value);
}

function restoreLineWrapEnabledFromStorage() {
  applyLineWrapEnabled(readStoredLineWrapEnabled());
}

function handleStorageEvent(event: StorageEvent) {
  if (event.key !== FILE_READER_LINE_WRAP_STORAGE_KEY) return;
  restoreLineWrapEnabledFromStorage();
}

export function initFileReaderAppearance() {
  if (initialized) return;
  initialized = true;
  restoreLineWrapEnabledFromStorage();
  if (typeof window !== "undefined") {
    window.addEventListener("storage", handleStorageEvent);
  }
  eventUnlisten = onTransportNotification<FileReaderAppearancePayload>("fileReaderAppearance.changed", (payload) => {
    applyLineWrapEnabled(payload?.lineWrapEnabled);
  });
}

export function disposeFileReaderAppearance() {
  if (!initialized) return;
  initialized = false;
  if (typeof window !== "undefined") {
    window.removeEventListener("storage", handleStorageEvent);
  }
  eventUnlisten?.();
  eventUnlisten = null;
}

export function useFileReaderAppearance() {
  initFileReaderAppearance();

  function setFileReaderLineWrapEnabled(enabled: boolean) {
    const normalized = normalizeFileReaderLineWrap(enabled);
    const changed = fileReaderLineWrapEnabled.value !== normalized;
    applyLineWrapEnabled(normalized);
    persistLineWrapEnabled(normalized);
    if (!changed) return;
    void emitTransportEvent("fileReaderAppearance.changed", { lineWrapEnabled: normalized }).catch((error) => {
      console.warn("[文件浏览器外观] 同步换行设置失败", error);
    });
  }

  function toggleFileReaderLineWrapEnabled() {
    setFileReaderLineWrapEnabled(!fileReaderLineWrapEnabled.value);
  }

  return {
    fileReaderLineWrapEnabled,
    setFileReaderLineWrapEnabled,
    toggleFileReaderLineWrapEnabled,
  };
}
