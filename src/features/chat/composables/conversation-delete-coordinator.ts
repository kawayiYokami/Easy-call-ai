export type ConversationDeleteResult<TConversation = unknown> = {
  deletedConversationId?: string;
  activeConversationId?: string | null;
  unarchivedConversations?: TConversation[];
};

type CoordinateConversationDeleteOptions<TConversation, TResult extends ConversationDeleteResult<TConversation>> = {
  conversationId: string;
  currentConversationId: () => string;
  deleteConversation: (conversationId: string) => Promise<TResult>;
  applyConversationList?: (items: TConversation[]) => void;
  readConversationList?: () => TConversation[];
  refreshConversationList?: () => Promise<void>;
  syncConversationList?: () => Promise<void>;
  conversationIds: () => string[];
  clearCurrentConversation: (reason: string) => void;
  openConversation: (conversationId: string) => Promise<void>;
  onNoReplacement?: () => Promise<void> | void;
};

/**
 * 删除会话后的前台选择只有这一套顺序：后端删除成功 → 应用列表 → 清理当前
 * 投影 → 打开后端指定会话或列表中的第一个剩余会话。App 与 Web 不再各自
 * 决定 activeConversationId 的回退规则。
 */
export async function coordinateConversationDelete<
  TConversation,
  TResult extends ConversationDeleteResult<TConversation>,
>(options: CoordinateConversationDeleteOptions<TConversation, TResult>): Promise<TResult | null> {
  const conversationId = String(options.conversationId || "").trim();
  if (!conversationId) return null;
  const deletingCurrent = String(options.currentConversationId() || "").trim() === conversationId;
  const result = await options.deleteConversation(conversationId);

  if (Array.isArray(result.unarchivedConversations) && result.unarchivedConversations.length > 0) {
    options.applyConversationList?.(result.unarchivedConversations);
  } else if (options.applyConversationList && options.readConversationList) {
    // 后端删除后不再返回全量列表：本地过滤被删项，再差量同步收敛。
    options.applyConversationList(
      options.readConversationList().filter(
        (item) => String((item as any)?.conversationId || "").trim() !== conversationId,
      ),
    );
    if (options.syncConversationList) {
      await options.syncConversationList();
    } else {
      // 差量接口不可用时回退全量刷新，避免停留在本地过滤后的陈旧状态。
      await options.refreshConversationList?.();
    }
  } else {
    await options.refreshConversationList?.();
  }

  if (!deletingCurrent || String(options.currentConversationId() || "").trim() !== conversationId) {
    return result;
  }

  options.clearCurrentConversation("conversation_delete_current");
  const backendActiveConversationId = String(result.activeConversationId || "").trim();
  const fallbackConversationId = options.conversationIds()
    .map((id) => String(id || "").trim())
    .find((id) => !!id && id !== conversationId) || "";
  const nextConversationId = backendActiveConversationId || fallbackConversationId;
  if (nextConversationId) {
    await options.openConversation(nextConversationId);
  } else {
    await options.onNoReplacement?.();
  }
  return result;
}
