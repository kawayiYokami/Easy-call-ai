import type { RemoteImContact } from "../../../../../types/app";

export function platformLabelOf(platform: string): string {
  const value = String(platform || "").trim().toLowerCase();
  if (value === "feishu") return "Feishu";
  if (value === "dingtalk") return "DingTalk";
  if (value === "weixin_oc") return "个人微信";
  return "OneBot v11";
}

export function normalizeActivationMode(value: string): RemoteImContact["activationMode"] {
  const mode = String(value || "").trim().toLowerCase();
  if (mode === "always" || mode === "keyword") return mode;
  return "never";
}

export function normalizeResponseStrategy(
  value?: string,
): NonNullable<RemoteImContact["responseStrategy"]> {
  return value === "smart_judge" ? "smart_judge" : "always_reply";
}

export function normalizeProcessingMode(value?: string): "qa" | "continuous" {
  return value === "qa" ? "qa" : "continuous";
}

export function contactRouteLabel(_item: RemoteImContact): string {
  return "联系人独立会话";
}

export function contactRoutingHint(_item: RemoteImContact): string {
  return "部门将在该联系人的独立会话中处理消息";
}

export function processingModeHint(item: RemoteImContact): string {
  return normalizeProcessingMode(item.processingMode) === "qa"
    ? "当前为无上下文模式（问答模式）"
    : "当前为有上下文模式（会话模式）";
}

export function contactActivationHint(item: RemoteImContact): string {
  const mode = normalizeActivationMode(item.activationMode);
  if (mode === "always") return "始终入场：满足接待条件时，总是允许进入激活流程。";
  if (mode === "keyword") return "关键词入场：消息命中关键词时，才允许进入激活流程。";
  return "不入场：只记录消息，不进入激活流程。";
}

export function contactResponseStrategyHint(item: RemoteImContact): string {
  const mode = normalizeResponseStrategy(item.responseStrategy);
  if (mode === "smart_judge") {
    return "智能判断：先用快速模型检查这批消息是否真的需要回复。";
  }
  return "始终回复：一旦允许入场，就直接交给处理部门回复。";
}

export function contactCommunicationToggleEnabled(item: Pick<RemoteImContact, "allowReceive" | "allowSend">): boolean {
  return !!item.allowSend;
}

export function contactCommunicationToggleClass(item: Pick<RemoteImContact, "allowReceive" | "allowSend">): string {
  return contactCommunicationToggleEnabled(item) ? "toggle-success" : "";
}

export function parseKeywordList(raw: string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const item of String(raw || "").split(/[,\n，]/)) {
    const keyword = item.trim();
    if (!keyword || seen.has(keyword)) continue;
    seen.add(keyword);
    out.push(keyword);
  }
  return out;
}

export function parseActivationKeywords(raw: string): string[] {
  return parseKeywordList(raw);
}

export function formatLogTime(timestamp: string): string {
  const d = new Date(timestamp);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}:${String(d.getSeconds()).padStart(2, "0")}`;
}
