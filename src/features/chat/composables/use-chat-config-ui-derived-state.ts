import { computed, type Ref } from "vue";
import type { AppConfig, PersonaProfile, ResponseStyleOption } from "../../../types/app";
import responseStylesJson from "../../../constants/response-styles.json";
import { buildPersonasSnapshotJson as buildPersonasSnapshotJsonValue } from "./use-chat-window-message-helpers";

type UseChatConfigUiDerivedStateOptions = {
  config: AppConfig;
  apiModelOptions: Ref<Record<string, string[]>>;
  modelRefreshOkFlags: Ref<Record<string, boolean>>;
  selectedApiConfig: Ref<{ requestFormat?: string } | null>;
  personas: Ref<PersonaProfile[]>;
  lastSavedConfigJson: Ref<string>;
  lastSavedPersonasJson: Ref<string>;
  buildConfigSnapshotJson: () => string;
  t: (key: string) => string;
};

export function useChatConfigUiDerivedState(options: UseChatConfigUiDerivedStateOptions) {
  const selectedModelOptions = computed(() => {
    const id = options.config.selectedApiConfigId;
    if (!id) return [];
    return options.apiModelOptions.value[id] ?? [];
  });
  const selectedModelRefreshOk = computed(() => {
    const id = options.config.selectedApiConfigId;
    if (!id) return false;
    return !!options.modelRefreshOkFlags.value[id];
  });
  const responseStyleOptions = responseStylesJson as ResponseStyleOption[];
  const baseUrlReference = computed(() => {
    const format = options.selectedApiConfig.value?.requestFormat ?? "openai";
    if (format === "gemini") return "https://generativelanguage.googleapis.com";
    if (format === "gemini_embedding") return "https://generativelanguage.googleapis.com";
    if (format === "anthropic") return "https://api.anthropic.com";
    if (format === "openai_tts") return "https://api.openai.com/v1/audio/speech";
    if (format === "openai_stt") return "https://api.openai.com/v1";
    if (format === "openai_embedding") return "https://api.openai.com/v1";
    if (format === "openai_rerank") return "https://api.openai.com/v1";
    return "https://api.openai.com/v1";
  });
  const chatInputPlaceholder = computed(() => options.t("chat.placeholder"));
  const defaultCreateConversationDepartmentId = computed(() => "assistant-department");
  const configDirty = computed(() => options.buildConfigSnapshotJson() !== options.lastSavedConfigJson.value);
  const personaDirty = computed(() => buildPersonasSnapshotJsonValue(options.personas.value) !== options.lastSavedPersonasJson.value);
  const responseStyleIds = computed(() => responseStyleOptions.map((item) => item.id));

  return {
    selectedModelOptions,
    selectedModelRefreshOk,
    responseStyleOptions,
    baseUrlReference,
    chatInputPlaceholder,
    defaultCreateConversationDepartmentId,
    configDirty,
    personaDirty,
    responseStyleIds,
  };
}
