import { ref } from "vue";

type TrFn = (key: string, params?: Record<string, unknown>) => string;

export function useHotkeyRecordTest(options: {
  t: TrFn;
  setStatus: (text: string) => void;
  setStatusError: (key: string, error: unknown) => void;
  isBlocked?: () => boolean;
}) {
  const hotkeyTestRecording = ref(false);
  const hotkeyTestRecordingMs = ref(0);
  const hotkeyTestAudio = ref<{ mime: string; bytesBase64: string; durationMs: number } | null>(null);

  let recorder: MediaRecorder | null = null;
  let stream: MediaStream | null = null;
  let startedAt = 0;
  let tickTimer: ReturnType<typeof setInterval> | null = null;
  let player: HTMLAudioElement | null = null;

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

  async function startHotkeyRecordTest() {
    if (hotkeyTestRecording.value || options.isBlocked?.()) return;
    if (!navigator.mediaDevices?.getUserMedia || typeof MediaRecorder === "undefined") {
      options.setStatus(options.t("status.recordUnsupported"));
      return;
    }
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
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

  return {
    hotkeyTestRecording,
    hotkeyTestRecordingMs,
    hotkeyTestAudio,
    startHotkeyRecordTest,
    stopHotkeyRecordTest,
    playHotkeyRecordTest,
    cleanupHotkeyRecordTest,
  };
}
