import { ref, type Ref } from "vue";
import type { ConfigSaveErrorInfo } from "../../config/composables/use-config-persistence";

type TrFn = (key: string, params?: Record<string, unknown>) => string;

export function useConfigSaveErrorDialog(options: {
  t: TrFn;
  configTab: Ref<string>;
}) {
  const configSaveErrorDialogOpen = ref(false);
  const configSaveErrorDialogTitle = ref("");
  const configSaveErrorDialogBody = ref("");
  const configSaveErrorDialogKind = ref<"warning" | "error">("error");

  function closeSettingsSaveErrorDialog() {
    configSaveErrorDialogOpen.value = false;
  }

  function openSettingsSaveErrorDialog(info: ConfigSaveErrorInfo) {
    configSaveErrorDialogTitle.value = options.t("status.saveConfigDialogTitle");
    if (info.kind === "hotkey_conflict") {
      configSaveErrorDialogKind.value = "warning";
      configSaveErrorDialogBody.value = `${options.t("status.saveConfigHotkeyOccupied", { hotkey: info.hotkey })}\n${options.t("status.saveConfigDialogHint")}`;
      options.configTab.value = "hotkey";
    } else if (info.kind === "backend_404") {
      configSaveErrorDialogKind.value = "error";
      configSaveErrorDialogBody.value = options.t("status.saveConfigBackend404");
    } else {
      configSaveErrorDialogKind.value = "error";
      configSaveErrorDialogBody.value = options.t("status.saveConfigFailed", { err: info.errorText });
    }
    configSaveErrorDialogOpen.value = true;
  }

  return {
    configSaveErrorDialogOpen,
    configSaveErrorDialogTitle,
    configSaveErrorDialogBody,
    configSaveErrorDialogKind,
    closeSettingsSaveErrorDialog,
    openSettingsSaveErrorDialog,
  };
}
