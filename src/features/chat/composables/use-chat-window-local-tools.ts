import { invokeTauri } from "../../../services/tauri-api";

export function useChatWindowLocalTools(bindings: Record<string, any>) {
  function updatePersonaEditorIdWithNotice(value: string) {
    const nextId = String(value || "").trim();
    if (!nextId || nextId === bindings.personaEditorId.value) return;
    if (bindings.personaDirty.value) {
      const currentName = String(bindings.selectedPersonaEditor.value?.name || bindings.personaEditorId.value || "").trim()
        || bindings.t("config.persona.title");
      bindings.status.value = bindings.t("status.personaUnsavedSwitchHint", { name: currentName });
    }
    bindings.personaEditorId.value = nextId;
  }

  function updateAssistantDepartmentAgentId(value: string) {
    bindings.assistantDepartmentAgentId.value = value;
  }

  async function updateForegroundDepartmentPrimaryApiConfig(value: string) {
    const nextId = String(value || "").trim();
    if (!nextId) return;
    if (!bindings.config.apiConfigs.some((item: any) => String(item.id || "").trim() === nextId)) {
      console.warn("[聊天模型] 选择的模型不存在，忽略更新", { nextId });
      return;
    }
    const currentDepartmentId = String(bindings.currentForegroundDepartmentId.value || "").trim();
    const currentDepartment = bindings.config.departments.find(
      (item: any) => String(item.id || "").trim() === currentDepartmentId,
    );
    if (!currentDepartment) {
      console.warn("[聊天模型] 当前前台部门不存在，忽略更新", { currentDepartmentId, nextId });
      return;
    }
    const previousDepartment = {
      apiConfigId: String(currentDepartment.apiConfigId || "").trim(),
      apiConfigIds: [...(currentDepartment.apiConfigIds || [])],
      updatedAt: String(currentDepartment.updatedAt || ""),
    };
    const previousAssistantDepartmentApiConfigId = String(bindings.config.assistantDepartmentApiConfigId || "").trim();
    const previousSelectedApiConfigId = String(bindings.config.selectedApiConfigId || "").trim();
    const changed = bindings.applyDepartmentPrimaryApiConfigLocally(currentDepartment, nextId);
    if (!changed) return;
    try {
      await invokeTauri("set_department_primary_api_config", {
        input: {
          departmentId: currentDepartmentId,
          apiConfigId: nextId,
        },
      });
    } catch (error) {
      currentDepartment.apiConfigId = previousDepartment.apiConfigId;
      currentDepartment.apiConfigIds = previousDepartment.apiConfigIds;
      currentDepartment.updatedAt = previousDepartment.updatedAt;
      bindings.config.assistantDepartmentApiConfigId = previousAssistantDepartmentApiConfigId;
      bindings.config.selectedApiConfigId = previousSelectedApiConfigId;
      bindings.setStatusError("status.saveConfigFailed", error);
    }
  }

  function updateAssistantDepartmentApiConfigId(value: string) {
    if (bindings.detachedChatWindow.value) {
      const nextId = String(value || "").trim();
      if (nextId && !bindings.config.apiConfigs.some((item: any) => item.id === nextId && item.enableText)) {
        bindings.setStatus("当前模型不可用，请重新选择。");
        return;
      }
      bindings.detachedTemporaryApiConfigId.value = nextId;
      return;
    }
    void updateForegroundDepartmentPrimaryApiConfig(value);
  }

  function updateSelectedResponseStyleId(value: string) {
    bindings.selectedResponseStyleId.value = value;
  }

  function updateSelectedPdfReadMode(value: "text" | "image") {
    bindings.selectedPdfReadMode.value = value;
  }

  function updateBackgroundVoiceScreenshotKeywords(value: string) {
    bindings.backgroundVoiceScreenshotKeywords.value = String(value || "").replace(/，/g, ",");
  }

  function updateBackgroundVoiceScreenshotMode(value: "desktop" | "focused_window") {
    bindings.backgroundVoiceScreenshotMode.value = value;
  }

  function updateInstructionPresets(value: any[]) {
    bindings.instructionPresets.value = Array.isArray(value)
      ? value
          .map((item) => ({
            id: String(item?.id || "").trim(),
            name: String(item?.prompt || item?.name || "").trim(),
            prompt: String(item?.prompt || item?.name || "").trim(),
          }))
          .filter((item) => !!item.id && !!item.prompt)
      : [];
  }

  function parseBackgroundVoiceScreenshotKeywords(raw: string): string[] {
    return Array.from(
      new Set(
        String(raw || "")
          .split(/[,\n;，；]+/)
          .map((item) => item.trim())
          .filter(Boolean),
      ),
    );
  }

  function matchBackgroundVoiceScreenshotKeyword(text: string, keywords: string[]): string | null {
    const normalize = (value: string) => String(value || "").replace(/\s+/g, "").toLocaleLowerCase();
    const target = normalize(text);
    if (!target || keywords.length === 0) return null;
    for (const keyword of keywords) {
      const normalized = normalize(keyword);
      if (!normalized) continue;
      if (target.includes(normalized)) {
        return keyword;
      }
    }
    return null;
  }

  async function queueAutoScreenshotFromVoice(input: {
    source: "local" | "remote";
    keyword: string;
    mode: "desktop" | "focused_window";
    startedAt: number;
  }) {
    const apiConfig = bindings.currentForegroundApiConfig.value;
    if (!apiConfig) {
      console.warn("[后台语音截图] 跳过：当前无可用对话模型配置");
      return;
    }
    const screenshotModeLabel = input.mode === "focused_window" ? "前台窗口" : "全屏";
    try {
      let imageMime = "";
      let imageBase64 = "";
      if (input.mode === "focused_window") {
        const output = await invokeTauri<{ data?: { imageMime?: string; imageBase64?: string } }>("xcap", {
          input: {
            method: "capture_focused_window",
            args: {},
          },
        });
        imageMime = String(output?.data?.imageMime || "").trim();
        imageBase64 = String(output?.data?.imageBase64 || "").trim();
      } else {
        const output = await invokeTauri<{ imageMime?: string; imageBase64?: string }>("desktop_screenshot", {
          input: {
            mode: "desktop",
          },
        });
        imageMime = String(output?.imageMime || "").trim();
        imageBase64 = String(output?.imageBase64 || "").trim();
      }
      if (!imageBase64) {
        throw new Error("截图结果为空");
      }
      const queued = await invokeTauri<{
        mime: string;
        fileName: string;
        savedPath: string;
        attachAsMedia: boolean;
        bytesBase64?: string | null;
      }>("queue_inline_file_attachment", {
        input: {
          fileName: `voice-auto-${Date.now()}.webp`,
          mime: imageMime || "image/webp",
          bytesBase64: imageBase64,
        },
      });
      const mime = String(queued.mime || "").trim().toLowerCase();
      const imageSupported = !!apiConfig.enableImage || bindings.hasVisionFallback.value;
      const canSendAsImage =
        !!queued.attachAsMedia
        && !!String(queued.bytesBase64 || "").trim()
        && mime.startsWith("image/")
        && imageSupported;
      if (canSendAsImage) {
        bindings.clipboardImages.value.push({
          mime,
          bytesBase64: String(queued.bytesBase64 || "").trim(),
        });
      } else {
        const savedPath = String(queued.savedPath || "").trim();
        const relativePath = savedPath.replace(/\\/g, "/").replace(/^.*\/downloads\//, "downloads/");
        const fileName = String(queued.fileName || "").trim() || relativePath.split("/").pop() || "attachment";
        const id = `${relativePath || fileName}::${mime}`;
        if (!bindings.queuedAttachmentNotices.value.some((item: any) => item.id === id)) {
          bindings.queuedAttachmentNotices.value.push({
            id,
            fileName,
            relativePath: relativePath || savedPath || fileName,
            mime,
          });
        }
      }
      const elapsedMs = Date.now() - input.startedAt;
      console.info(
        "[后台语音截图] 完成：命中关键词=%s，模式=%s，来源=%s，耗时=%dms",
        input.keyword,
        screenshotModeLabel,
        input.source,
        elapsedMs,
      );
    } catch (error) {
      const elapsedMs = Date.now() - input.startedAt;
      console.error(
        "[后台语音截图] 失败：命中关键词=%s，模式=%s，来源=%s，耗时=%dms，原因=%s",
        input.keyword,
        screenshotModeLabel,
        input.source,
        elapsedMs,
        String(error),
      );
      bindings.setStatus(`后台语音截图失败：${String(error)}`);
    }
  }

  return {
    updatePersonaEditorIdWithNotice,
    updateAssistantDepartmentAgentId,
    updateForegroundDepartmentPrimaryApiConfig,
    updateAssistantDepartmentApiConfigId,
    updateSelectedResponseStyleId,
    updateSelectedPdfReadMode,
    updateBackgroundVoiceScreenshotKeywords,
    updateBackgroundVoiceScreenshotMode,
    updateInstructionPresets,
    parseBackgroundVoiceScreenshotKeywords,
    matchBackgroundVoiceScreenshotKeyword,
    queueAutoScreenshotFromVoice,
  };
}
