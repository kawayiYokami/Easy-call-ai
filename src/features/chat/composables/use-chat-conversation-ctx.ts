import { computed, type Ref } from "vue";
import type { ChatConversationOverviewItem, ChatMentionTarget, ChatTodoItem } from "../../../types/app";
import type { TerminalApprovalConversationItem } from "../../shell/composables/use-terminal-approval";

export function useChatConversationCtx(
  props: {
    currentTheme: string;
    activeConversationId: string;
    conversationItems?: ChatConversationOverviewItem[];
    unarchivedConversationItems: ChatConversationOverviewItem[];
    currentTodos: ChatTodoItem[];
    compactingConversation: boolean;
    compactingConversationId?: string;
    trimming: boolean;
    trimmingConversationId?: string;
    terminalApprovals?: TerminalApprovalConversationItem[];
    terminalApprovalResolving?: boolean;
    supervisionActive: boolean;
    supervisionTitle: string;
    toolStatusState: "running" | "done" | "failed" | "";
    toolStatusText: string;
    streamToolCalls: Array<{ name: string; argsText: string; status?: "doing" | "done" }>;
    chatErrorText: string;
    selectedMentions: ChatMentionTarget[];
    messageBlocks: Array<{ isExtraTextBlock?: boolean; planCard?: { action?: string }; sourceMessageId?: string; id?: string; providerMeta?: Record<string, unknown> }>;
  },
  isDarkAppTheme: (theme: string) => boolean,
  t: (key: string, params?: Record<string, unknown>) => string,
) {
  const markdownIsDark = computed(() => isDarkAppTheme(props.currentTheme));

  function isOrganizeContextToolCall(call: { name: string; status?: string }): boolean {
    const name = String(call.name || "").trim().toLowerCase();
    return name === "organize_context" || name === "archive";
  }

  function isOrganizeContextStatusText(text: string): boolean {
    const value = String(text || "").trim();
    return !!value && (
      value.includes(t("chat.statusCompactingContext"))
      || value.includes("压缩上下文")
      || value.includes("壓縮上下文")
      || value.includes("整理上下文")
      || value.includes("整理後繼續")
      || value.includes("整理后继续")
      || value.toLowerCase().includes("compacting context")
    );
  }

  const visibleStreamToolCalls = computed(() =>
    props.streamToolCalls.filter((call) => !isOrganizeContextToolCall(call)),
  );

  const VALID_TODO_STATUSES: ReadonlySet<string> = new Set(["pending", "in_progress", "completed"]);

  function isValidTodoStatus(value: string): value is "pending" | "in_progress" | "completed" {
    return VALID_TODO_STATUSES.has(value);
  }

  const normalizedConversationTodos = computed<Array<{ content: string; status: "pending" | "in_progress" | "completed" }>>(() => {
    const todos = Array.isArray(props.currentTodos) ? props.currentTodos : [];
    return todos
      .map((item) => {
        const status = String(item?.status || "").trim();
        return {
          content: String(item?.content || "").trim(),
          status: isValidTodoStatus(status) ? status : ("pending" as const),
        };
      })
      .filter((item) => item.content);
  });

  const activeConversationSummary = computed(() => {
    const id = String(props.activeConversationId || "").trim();
    if (!id) return null;
    return (props.conversationItems || props.unarchivedConversationItems).find(
      (item) => String(item.conversationId || "").trim() === id,
    ) || null;
  });

  const isCurrentConversationCompacting = computed(() =>
    !!String(props.activeConversationId || "").trim() &&
    String(props.activeConversationId || "").trim() === String(props.compactingConversationId || "").trim(),
  );

  const activeConversationTerminalApprovals = computed(() => {
    const conversationId = String(props.activeConversationId || "").trim();
    if (!conversationId) return [];
    return (Array.isArray(props.terminalApprovals) ? props.terminalApprovals : [])
      .filter((item) => String(item.conversationId || item.sessionId || "").trim() === conversationId);
  });

  const supervisionButtonTitle = computed(() => {
    const base = props.supervisionActive ? t("chat.supervision.activeButtonTitle") : t("chat.supervision.buttonTitle");
    const detail = String(props.supervisionTitle || "").trim();
    return detail ? `${base}\n${detail}` : base;
  });

  const activeRunningToolCall = computed(() => {
    if (props.toolStatusState !== "running") return null;
    const calls = Array.isArray(props.streamToolCalls) ? props.streamToolCalls : [];
    for (let idx = calls.length - 1; idx >= 0; idx -= 1) {
      const call = calls[idx];
      if (String(call?.status || "").trim() === "done") continue;
      if (!String(call?.name || "").trim()) continue;
      return call;
    }
    return null;
  });

  const isOrganizingContextBusy = computed(() => {
    if (props.compactingConversation && isCurrentConversationCompacting.value) return true;
    const runtimeState = String(activeConversationSummary.value?.runtimeState || "").trim();
    if (runtimeState === "organizing_context" || runtimeState === "compacting") return true;
    const runningTool = activeRunningToolCall.value;
    if (runningTool && isOrganizeContextToolCall(runningTool)) return true;
    const statusState = String(props.toolStatusState || "").trim();
    if (statusState !== "running") return false;
    const actualText = String(props.toolStatusText || "").trim();
    return isOrganizeContextStatusText(actualText);
  });

  const chatStatusBanner = computed<null | { text: string; tone: "default" | "error" }>(() => {
    const errorText = String(props.chatErrorText || "").trim();
    if (errorText) return { text: errorText, tone: "error" };
    if (props.trimming) {
      const currentId = String(props.activeConversationId || "").trim();
      const archivingId = String(props.trimmingConversationId || "").trim();
      if (!currentId || currentId !== archivingId) return null;
      return { text: t("chat.statusArchivingConversation"), tone: "default" };
    }
    if (isOrganizingContextBusy.value) {
      return { text: t("chat.statusCompactingContext"), tone: "default" };
    }
    return null;
  });

  const selectedMentionKeys = computed(() =>
    (Array.isArray(props.selectedMentions) ? props.selectedMentions : [])
      .map((item) => {
        const agentId = String(item?.agentId || "").trim();
        const departmentId = String(item?.departmentId || "").trim();
        return agentId && departmentId ? `${agentId}:${departmentId}` : "";
      })
      .filter((value, index, list) => !!value && list.indexOf(value) === index),
  );

  const latestPendingPlanMessageId = computed(() => {
    for (let idx = props.messageBlocks.length - 1; idx >= 0; idx -= 1) {
      const block = props.messageBlocks[idx];
      if (block.isExtraTextBlock) continue;
      const providerMeta = (block.providerMeta || {}) as Record<string, unknown>;
      const messageMeta = ((providerMeta.message_meta || providerMeta.messageMeta || {}) as Record<string, unknown>);
      const messageKind = String(messageMeta.kind || providerMeta.messageKind || "").trim();
      if (messageKind === "plan_complete" || block.planCard?.action === "complete") return "";
      if (messageKind === "plan_present" || block.planCard?.action === "present") {
        return String(block.sourceMessageId || block.id || "").trim();
      }
    }
    return "";
  });

  return {
    markdownIsDark,
    visibleStreamToolCalls,
    normalizedConversationTodos,
    activeConversationSummary,
    isCurrentConversationCompacting,
    activeConversationTerminalApprovals,
    supervisionButtonTitle,
    isOrganizingContextBusy,
    chatStatusBanner,
    selectedMentionKeys,
    latestPendingPlanMessageId,
  };
}
