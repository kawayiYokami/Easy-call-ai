import { ref } from "vue";

type TrFn = (key: string, params?: Record<string, unknown>) => string;
type PermissionStateText = "granted" | "denied" | "prompt" | "unsupported" | "unknown";

export function useHotkeyRecordTest(options: {
  t: TrFn;
  setStatus: (text: string) => void;
  setStatusError: (key: string, error: unknown) => void;
  isBlocked?: () => boolean;
}) {
  const hotkeyTestRecording = ref(false);
  const hotkeyTestRecordingMs = ref(0);
  const hotkeyTestAudio = ref<{ mime: string; bytesBase64: string; durationMs: number } | null>(null);
  const microphonePermissionState = ref<PermissionStateText>("unknown");
  const microphonePermissionRequesting = ref(false);
  let permissionStatus: PermissionStatus | null = null;
  let permissionStatusChangeHandler: (() => void) | null = null;

  let recorder: MediaRecorder | null = null;
  let stream: MediaStream | null = null;
  let startedAt = 0;
  let tickTimer: ReturnType<typeof setInterval> | null = null;
  let player: HTMLAudioElement | null = null;

  function setMicrophonePermissionState(value: unknown) {
    const text = String(value || "").trim();
    if (text === "granted" || text === "denied" || text === "prompt") {
      microphonePermissionState.value = text;
      return;
    }
    microphonePermissionState.value = text ? "unknown" : "unsupported";
  }

  function clearTimers() {
    if (!tickTimer) return;
    clearInterval(tickTimer);
    tickTimer = null;
  }

  function stopStream() {
    if (!stream) return;
    for (const track of stream.getTracks()) track.stop();
    stream = null;
  }

  async function readBlobAsDataUrl(blob: Blob): Promise<string> {
    return await new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result || ""));
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(blob);
    });
  }

  async function refreshMicrophonePermissionState() {
    if (!navigator.mediaDevices?.getUserMedia || typeof MediaRecorder === "undefined") {
      microphonePermissionState.value = "unsupported";
      return microphonePermissionState.value;
    }
    if (!navigator.permissions?.query) {
      microphonePermissionState.value = "unknown";
      return microphonePermissionState.value;
    }
    try {
      permissionStatus = await navigator.permissions.query({ name: "microphone" as PermissionName });
      setMicrophonePermissionState(permissionStatus.state);
      if (!permissionStatusChangeHandler) {
        permissionStatusChangeHandler = () => {
          setMicrophonePermissionState(permissionStatus?.state);
        };
      }
      permissionStatus.onchange = permissionStatusChangeHandler;
    } catch {
      microphonePermissionState.value = "unknown";
    }
    return microphonePermissionState.value;
  }

  async function requestMicrophonePermission() {
    if (microphonePermissionRequesting.value) return false;
    if (!navigator.mediaDevices?.getUserMedia || typeof MediaRecorder === "undefined") {
      microphonePermissionState.value = "unsupported";
      options.setStatus(options.t("status.recordUnsupported"));
      return false;
    }
    microphonePermissionRequesting.value = true;
    try {
      const nextStream = await navigator.mediaDevices.getUserMedia({ audio: true });
      for (const track of nextStream.getTracks()) track.stop();
      microphonePermissionState.value = "granted";
      await refreshMicrophonePermissionState();
      options.setStatus(options.t("status.microphonePermissionGranted"));
      return true;
    } catch (error) {
      await refreshMicrophonePermissionState();
      options.setStatusError("status.microphonePermissionRequestFailed", error);
      return false;
    } finally {
      microphonePermissionRequesting.value = false;
    }
  }

  async function startHotkeyRecordTest() {
    if (hotkeyTestRecording.value || options.isBlocked?.()) return;
    if (!navigator.mediaDevices?.getUserMedia || typeof MediaRecorder === "undefined") {
      microphonePermissionState.value = "unsupported";
      options.setStatus(options.t("status.recordUnsupported"));
      return;
    }
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      microphonePermissionState.value = "granted";
      recorder = new MediaRecorder(stream);
      const chunks: BlobPart[] = [];
      recorder.ondataavailable = (event: BlobEvent) => {
        if (event.data && event.data.size > 0) chunks.push(event.data);
      };
      recorder.onstop = async () => {
        const durationMs = Math.max(0, Date.now() - startedAt);
        try {
          if (chunks.length === 0) return;
          const blob = new Blob(chunks, { type: recorder?.mimeType || "audio/webm" });
          const dataUrl = await readBlobAsDataUrl(blob);
          const base64 = dataUrl.includes(",") ? dataUrl.split(",")[1] : "";
          if (!base64) return;
          hotkeyTestAudio.value = { mime: blob.type || "audio/webm", bytesBase64: base64, durationMs };
          options.setStatus(options.t("status.recordTestDone", { seconds: Math.max(1, Math.round(durationMs / 1000)) }));
        } catch (error) {
          options.setStatusError("status.recordTestFailed", error);
        } finally {
          hotkeyTestRecording.value = false;
          clearTimers();
          stopStream();
        }
      };
      recorder.start();
      startedAt = Date.now();
      hotkeyTestRecording.value = true;
      hotkeyTestRecordingMs.value = 0;
      clearTimers();
      tickTimer = setInterval(() => {
        hotkeyTestRecordingMs.value = Math.max(0, Date.now() - startedAt);
      }, 100);
    } catch (error) {
      hotkeyTestRecording.value = false;
      clearTimers();
      stopStream();
      await refreshMicrophonePermissionState();
      options.setStatusError("status.recordTestFailed", error);
    }
  }

  async function stopHotkeyRecordTest() {
    if (!hotkeyTestRecording.value) return;
    if (recorder && recorder.state !== "inactive") {
      recorder.stop();
      return;
    }
    hotkeyTestRecording.value = false;
    clearTimers();
    stopStream();
  }

  function playHotkeyRecordTest() {
    if (!hotkeyTestAudio.value) return;
    if (player) {
      player.pause();
      player.currentTime = 0;
      player = null;
    }
    player = new Audio(`data:${hotkeyTestAudio.value.mime};base64,${hotkeyTestAudio.value.bytesBase64}`);
    void player.play().catch(() => {
      player = null;
    });
  }

  async function cleanupHotkeyRecordTest() {
    await stopHotkeyRecordTest();
    if (player) {
      player.pause();
      player.currentTime = 0;
      player = null;
    }
    clearTimers();
    stopStream();
  }

  void refreshMicrophonePermissionState();

  return {
    hotkeyTestRecording,
    hotkeyTestRecordingMs,
    hotkeyTestAudio,
    microphonePermissionState,
    microphonePermissionRequesting,
    startHotkeyRecordTest,
    stopHotkeyRecordTest,
    playHotkeyRecordTest,
    requestMicrophonePermission,
    refreshMicrophonePermissionState,
    cleanupHotkeyRecordTest,
  };
}
