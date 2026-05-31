import { computed, type Ref } from "vue";
import { useGithubUpdate } from "./use-github-update";
import type { GithubUpdateMethod } from "../../../types/app";

export type GithubUpdateViewBindings = {
  t: (key: string, params?: Record<string, unknown>) => string;
  viewMode: Ref<"chat" | "archives" | "config">;
  status: Ref<string>;
  updateMethod: Ref<GithubUpdateMethod | undefined>;
};

export function useGithubUpdateView(bindings: GithubUpdateViewBindings) {
  const githubUpdate = useGithubUpdate({
    viewMode: bindings.viewMode,
    status: bindings.status,
    updateMethod: bindings.updateMethod,
  });

  const showUpdateToLatestButton = computed(() => githubUpdate.hasAvailableUpdate.value);
  const updateToLatestLabel = computed(() =>
    githubUpdate.updateReadyToRestart.value
      ? bindings.t("about.updateAndRestart")
      : githubUpdate.checkingUpdate.value
        ? bindings.t("about.updating")
        : bindings.t("about.updateNow"),
  );
  const updateToLatestTitle = computed(() => {
    const latestVersion = String(githubUpdate.latestCheckResult.value?.latestVersion || "").trim();
    if (githubUpdate.updateReadyToRestart.value && latestVersion) {
      return bindings.t("about.updateReadyAction", { version: latestVersion });
    }
    if (latestVersion) {
      return bindings.t("about.updateAvailableAction", { version: latestVersion });
    }
    return bindings.t("about.updateNow");
  });

  return {
    ...githubUpdate,
    showUpdateToLatestButton,
    updateToLatestLabel,
    updateToLatestTitle,
  };
}
