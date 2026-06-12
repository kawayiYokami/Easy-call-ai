import { createApp } from "vue";
import App from "./App.vue";
import { i18n } from "../../i18n";
import "../../style.css";
import "../chat/markdown/markdown-content.css";
import { LUCIDE_CONTEXT } from "../../lucide-context";
import { useAppTheme } from "../shell/composables/use-app-theme";

function isVsCodeSidebarHost(): boolean {
  const bridgeWindow = window as Window & { acquireVsCodeApi?: unknown };
  return typeof bridgeWindow.acquireVsCodeApi === "function" || window.location.protocol === "vscode-webview:";
}

async function prepareSidebarThemeHost() {
  if (isVsCodeSidebarHost()) {
    document.documentElement.setAttribute("data-host", "vscode");
    await import("./assets/sidebar-theme.css");
    return;
  }
  document.documentElement.setAttribute("data-host", "web");
  const { restoreThemeFromStorage } = useAppTheme();
  restoreThemeFromStorage();
}

await prepareSidebarThemeHost();

createApp(App).use(i18n).provide(LUCIDE_CONTEXT, {}).mount("#app");
