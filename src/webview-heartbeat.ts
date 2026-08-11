import { acknowledgeTransportWebviewHeartbeat, onTransportNotification } from "./services/tauri-api";

// 按需测活响应：后端仅在唤出窗口时发 ping，前端收到即回 pong，用于确认 WebView 存活。
// 无常驻轮询，平时零开销。
onTransportNotification("webview.ping", () => {
  acknowledgeTransportWebviewHeartbeat().catch(() => {});
});
