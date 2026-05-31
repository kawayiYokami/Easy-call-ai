import type { Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";

type UseWindowActionsOptions = {
  isChatTauriWindow: Ref<boolean>;
  closeWindow: () => Promise<void>;
  minimizeWindow: () => Promise<void>;
  freezeForegroundConversation: (reason: string) => void;
};

export function useWindowActions(options: UseWindowActionsOptions) {
  function openSettingsWindow() {
    void invokeTauri("show_main_window");
  }

  function summonChatWindowFromConfig() {
    if (options.isChatTauriWindow.value) {
      options.freezeForegroundConversation("before_manual_summon");
    }
    void invokeTauri("show_chat_window");
  }

  async function closeWindowAndClearForeground() {
    if (options.isChatTauriWindow.value) {
      options.freezeForegroundConversation("close_window");
    }
    await options.closeWindow();
  }

  async function minimizeWindowAndClearForeground() {
    if (options.isChatTauriWindow.value) {
      options.freezeForegroundConversation("minimize_window");
    }
    await options.minimizeWindow();
  }

  async function openGithubRepository() {
    try {
      const url = await invokeTauri<string>("get_project_repository_url");
      void invokeTauri("open_external_url", { url });
    } catch (error) {
      console.warn("[关于] 获取项目仓库地址失败:", error);
    }
  }

  return {
    openSettingsWindow,
    summonChatWindowFromConfig,
    closeWindowAndClearForeground,
    minimizeWindowAndClearForeground,
    openGithubRepository,
  };
}
