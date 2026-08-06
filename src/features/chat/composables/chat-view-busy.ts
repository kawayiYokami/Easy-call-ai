/**
 * 视图层「忙碌」统一语义：主会话与追问会话共用同一份判定，
 * 避免两个外壳各自拼装 conversation-busy 导致语义分叉。
 *
 * 约定：
 * - 结构性操作（修剪/压缩/组织上下文）期间锁定交互，停止按钮禁用；
 * - 流式态（chatting / assistant_streaming）**不**算忙碌——流式时停止按钮必须可用；
 * - submitPending 由 ChatView 的 stop-chat-disabled 单独处理（含在 :stop-chat-disabled
 *   的 isOrganizingContextBusy || submitPending 中），不重复计入视图层忙碌。
 */
export type ViewLayerBusyInput = {
  trimming: boolean;
  trimmingConversationId?: string;
  compactingConversation: boolean;
  compactingConversationId?: string;
  activeConversationId: string;
  /** 组织上下文进行中（追问视图用本地 runtimeState 判定，主会话由 isOrganizingContextBusy 覆盖） */
  organizingContext: boolean;
};

export function isViewLayerBusy(input: ViewLayerBusyInput): boolean {
  const currentId = String(input.activeConversationId || "").trim();
  const trimmingId = String(input.trimmingConversationId || "").trim();
  const compactingId = String(input.compactingConversationId || "").trim();
  const trimmingCurrent =
    input.trimming && (!trimmingId || trimmingId === currentId);
  const compactingCurrent =
    input.compactingConversation && (!compactingId || compactingId === currentId);
  return trimmingCurrent || compactingCurrent || input.organizingContext;
}
