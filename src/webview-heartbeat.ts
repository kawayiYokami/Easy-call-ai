import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

listen("easy-call:webview-ping", () => {
  invoke("webview_pong").catch(() => {});
});
