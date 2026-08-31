import { ref, type Ref } from "vue";
import { openTransportFileDialog, openTransportWorkspaceDirectory } from "../../../services/tauri-api";
import type { ChatWorkspaceChoice } from "./use-chat-workspace";
import type { ShellWorkMode } from "../../../types/app";
import { normalizeShellWorkMode, normalizeWorkspaceAccess } from "../../../utils/shell-workspaces";

type UseChatWorkspacePickerFlowOptions = {
  chatWorkspaceChoices: Ref<ChatWorkspaceChoice[]>;
  chatWorkspaceAutonomousMode: Ref<boolean>;
  chatWorkspaceWorkMode: Ref<ShellWorkMode>;
  chatWorkspaceBranch: Ref<string>;
  openChatWorkspacePickerBase: () => void;
  closeChatWorkspacePickerBase: () => void;
  saveChatWorkspaces: (items: ChatWorkspaceChoice[], autonomousMode?: boolean, workMode?: ShellWorkMode, shellWorkBranch?: string) => Promise<void>;
  setStatus: (message: string) => void;
  setStatusError: (key: string, error: unknown) => void;
  workspaceAlreadyExistsText: string;
  worktreeRequiresApprovalText: string;
  worktreeUnavailableText: string;
  checkChatWorkspaceGitRoot: (path: string) => Promise<boolean>;
};

export function useChatWorkspacePickerFlow(options: UseChatWorkspacePickerFlowOptions) {
  const chatWorkspaceDraftChoices = ref<ChatWorkspaceChoice[]>([]);
  const chatWorkspaceDraftAutonomousMode = ref(false);
  const chatWorkspaceDraftWorkMode = ref<ShellWorkMode>("directory");
  const chatWorkspaceDraftBranch = ref("");
  const chatWorkspaceDraftError = ref("");
  const chatWorkspacePickerSaving = ref(false);

  function cloneChatWorkspaceChoices(items: ChatWorkspaceChoice[]): ChatWorkspaceChoice[] {
    return (items || []).map((item) => ({
      id: String(item.id || "").trim(),
      name: String(item.name || "").trim(),
      path: String(item.path || "").trim(),
      level: item.level,
      access: item.access,
    }));
  }

  function syncChatWorkspaceDraftFromCurrentState() {
    chatWorkspaceDraftChoices.value = cloneChatWorkspaceChoices(options.chatWorkspaceChoices.value);
    chatWorkspaceDraftAutonomousMode.value = Boolean(options.chatWorkspaceAutonomousMode.value);
    chatWorkspaceDraftWorkMode.value = normalizeShellWorkMode(String(options.chatWorkspaceWorkMode.value || ""));
    chatWorkspaceDraftBranch.value = String(options.chatWorkspaceBranch.value || "").trim();
    chatWorkspaceDraftError.value = "";
  }

  function openChatWorkspacePicker() {
    syncChatWorkspaceDraftFromCurrentState();
    options.openChatWorkspacePickerBase();
  }

  function closeChatWorkspacePicker() {
    if (chatWorkspacePickerSaving.value) return;
    options.closeChatWorkspacePickerBase();
    syncChatWorkspaceDraftFromCurrentState();
  }

  async function addChatWorkspace() {
    try {
      const picked = await openTransportFileDialog({
        directory: true,
        multiple: false,
      });
      if (!picked || Array.isArray(picked)) return;
      const nextPath = String(picked || "").trim();
      if (!nextPath) return;
      const draft = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value);
      const existed = draft.some((item) => String(item.path || "").trim().toLowerCase() === nextPath.toLowerCase());
      if (existed) {
        options.setStatus(options.workspaceAlreadyExistsText);
        return;
      }
      const hasMain = draft.some((item) => item.level === "main");
      // 新会话多目录：权限统一为 approval，不再按目录分离
      const unifiedAccess = normalizeWorkspaceAccess(String(draft.find((w) => w.level === "main")?.access || draft[0]?.access || "approval"));
      draft.push({
        id: `conversation-workspace-${Math.random().toString(36).slice(2, 8)}`,
        name: nextPath.replace(/\\/g, "/").replace(/\/+$/, "").split("/").pop() || nextPath,
        path: nextPath,
        level: hasMain ? "secondary" : "main",
        access: unifiedAccess,
      });
      chatWorkspaceDraftChoices.value = draft;
    } catch (error) {
      options.setStatusError("status.requestFailed", error);
    }
  }

  async function setChatWorkspaceAsMain(workspaceId: string) {
    chatWorkspaceDraftError.value = "";
    const draft: ChatWorkspaceChoice[] = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value).map((item): ChatWorkspaceChoice => {
      if (item.level === "system") return item;
      if (item.id === workspaceId) {
        return { ...item, level: "main", access: item.access || "approval" };
      }
      if (item.level === "main") {
        return { ...item, level: "secondary" };
      }
      return item;
    });
    chatWorkspaceDraftChoices.value = draft;
  }

  function setChatWorkspaceAccess(access: ChatWorkspaceChoice["access"]) {
    chatWorkspaceDraftError.value = "";
    const normalized = normalizeWorkspaceAccess(String(access || ""));
    const draft = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value).map((item) => {
      if (item.level === "system") return item;
      return { ...item, access: normalized as ChatWorkspaceChoice["access"] };
    });
    chatWorkspaceDraftChoices.value = draft;
  }

  // 兼容旧按目录调用：统一权限，忽略 workspaceId
  function setChatWorkspaceAccessLegacy(_workspaceId: string, access: ChatWorkspaceChoice["access"]) {
    setChatWorkspaceAccess(access);
  }

  async function removeChatWorkspace(workspaceId: string) {
    chatWorkspaceDraftError.value = "";
    const current = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value);
    const removing = current.find((item) => item.id === workspaceId);
    const draft = current.filter((item) => item.id !== workspaceId || item.level === "system");
    if (removing?.level === "main") {
      const promoteTarget = draft.find((item) => item.level === "secondary");
      if (promoteTarget) {
        draft.forEach((item) => {
          if (item.level === "system") return;
          if (item.id === promoteTarget.id) {
            item.level = "main";
          } else if (item.level === "main") {
            item.level = "secondary";
          }
        });
      }
    }
    chatWorkspaceDraftChoices.value = draft;
    // 主目录被删后，若 branch 仍存在但新主不是 git，分支会在 WorkspaceConfigCard 隐藏，保存时自动清空
    if (removing?.level === "main") {
      chatWorkspaceDraftBranch.value = "";
    }
  }

  function setChatWorkspaceAutonomousMode(enabled: boolean) {
    chatWorkspaceDraftAutonomousMode.value = Boolean(enabled);
  }

  function setChatWorkspaceWorkMode(mode: ShellWorkMode) {
    chatWorkspaceDraftError.value = "";
    const next = normalizeShellWorkMode(String(mode || ""));
    chatWorkspaceDraftWorkMode.value = next;
    if (next !== "worktree") {
      chatWorkspaceDraftBranch.value = "";
    }
  }

  function setChatWorkspaceBranch(branch: string) {
    chatWorkspaceDraftBranch.value = String(branch || "").trim();
  }

  async function openChatWorkspaceDir(workspaceId: string) {
    const draft = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value);
    const target = draft.find((item) => item.id === workspaceId);
    if (!target?.path) return;
    try {
      const opened = await openTransportWorkspaceDirectory(target.path);
      if (opened) options.setStatus(`已打开目录: ${opened}`);
    } catch (error) {
      options.setStatusError("config.tools.openDirFailed", error);
    }
  }

  function setMainPath(path: string) {
    const normalized = String(path || "").trim();
    if (!normalized) return;
    const draft = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value);
    const existing = draft.find((item) => String(item.path || "").trim().toLowerCase() === normalized.toLowerCase());
    if (existing) {
      void setChatWorkspaceAsMain(existing.id);
      return;
    }
    const hasMain = draft.some((item) => item.level === "main");
    if (hasMain) {
      const newDraft = draft.map((item) => item.level === "main" ? { ...item, level: "secondary" as const } : item);
      const unifiedAccess = normalizeWorkspaceAccess(String(newDraft.find((w) => w.level === "secondary")?.access || newDraft[0]?.access || "approval"));
      newDraft.push({
        id: `conversation-workspace-${Math.random().toString(36).slice(2, 8)}`,
        name: normalized.replace(/\\/g, "/").replace(/\/+$/, "").split("/").pop() || normalized,
        path: normalized,
        level: "main",
        access: unifiedAccess,
      });
      chatWorkspaceDraftChoices.value = newDraft;
    } else {
      const unifiedAccess = normalizeWorkspaceAccess(String(draft[0]?.access || "approval"));
      draft.push({
        id: `conversation-workspace-${Math.random().toString(36).slice(2, 8)}`,
        name: normalized.replace(/\\/g, "/").replace(/\/+$/, "").split("/").pop() || normalized,
        path: normalized,
        level: "main",
        access: unifiedAccess,
      });
      chatWorkspaceDraftChoices.value = draft;
    }
    chatWorkspaceDraftBranch.value = "";
  }

  function addSecondaryPath(path: string) {
    const normalized = String(path || "").trim();
    if (!normalized) return;
    const draft = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value);
    if (draft.some((item) => String(item.path || "").trim().toLowerCase() === normalized.toLowerCase())) {
      options.setStatus(options.workspaceAlreadyExistsText);
      return;
    }
    const unifiedAccess = normalizeWorkspaceAccess(String(draft.find((w) => w.level === "main")?.access || draft[0]?.access || "approval"));
    draft.push({
      id: `conversation-workspace-${Math.random().toString(36).slice(2, 8)}`,
      name: normalized.replace(/\\/g, "/").replace(/\/+$/, "").split("/").pop() || normalized,
      path: normalized,
      level: draft.some((item) => item.level === "main") ? "secondary" : "main",
      access: unifiedAccess,
    });
    chatWorkspaceDraftChoices.value = draft;
  }

  function removeSecondaryPath(path: string) {
    const key = String(path || "").trim().toLowerCase();
    if (!key) return;
    const matched = chatWorkspaceDraftChoices.value.find((item) => String(item.path || "").trim().toLowerCase() === key);
    if (matched) void removeChatWorkspace(matched.id);
  }

  async function saveChatWorkspacePicker() {
    if (chatWorkspacePickerSaving.value) return;
    chatWorkspacePickerSaving.value = true;
    try {
      const normalizedMode = normalizeShellWorkMode(String(chatWorkspaceDraftWorkMode.value || ""));
      const draft = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value);
      // 旧 read_only 已迁移为 approval，此处仅保留 Git 根校验提示，worktree 不再阻断
      if (normalizedMode === "worktree") {
        const mainWorkspace = draft.find((item) => item.level === "main") || draft[0];
        if (mainWorkspace) {
          const gitOk = await options.checkChatWorkspaceGitRoot(String(mainWorkspace.path || ""));
          if (!gitOk) {
            chatWorkspaceDraftError.value = options.worktreeUnavailableText || options.worktreeRequiresApprovalText;
            options.setStatus(chatWorkspaceDraftError.value);
            return;
          }
        }
      }
      const branchToSave = normalizedMode === "worktree" ? String(chatWorkspaceDraftBranch.value || "").trim() : "";
      await options.saveChatWorkspaces(draft, chatWorkspaceDraftAutonomousMode.value, normalizedMode, branchToSave);
      options.closeChatWorkspacePickerBase();
      syncChatWorkspaceDraftFromCurrentState();
    } finally {
      chatWorkspacePickerSaving.value = false;
    }
  }

  return {
    chatWorkspaceDraftChoices,
    chatWorkspaceDraftAutonomousMode,
    chatWorkspaceDraftWorkMode,
    chatWorkspaceDraftBranch,
    chatWorkspaceDraftError,
    chatWorkspacePickerSaving,
    openChatWorkspacePicker,
    closeChatWorkspacePicker,
    addChatWorkspace,
    setChatWorkspaceAsMain,
    setMainPath,
    setChatWorkspaceAccess,
    setChatWorkspaceAccessLegacy,
    setChatWorkspaceBranch,
    setChatWorkspaceAutonomousMode,
    setChatWorkspaceWorkMode,
    removeChatWorkspace,
    addSecondaryPath,
    removeSecondaryPath,
    openChatWorkspaceDir,
    saveChatWorkspacePicker,
  };
}
