const STRUCTURED_ERROR_KEYS = [
  "message",
  "detail",
  "details",
  "reason",
  "error_description",
  "errorMessage",
  "description",
  "error",
  "cause",
  "body",
] as const;

const HTML_ENTITY_MAP: Record<string, string> = {
  "&quot;": "\"",
  "&#39;": "'",
  "&apos;": "'",
  "&lt;": "<",
  "&gt;": ">",
  "&amp;": "&",
  "&nbsp;": " ",
};

function decodeHtmlEntities(value: string): string {
  return value.replace(/&(quot|#39|apos|lt|gt|amp|nbsp);/g, (entity) => HTML_ENTITY_MAP[entity] || entity);
}

function cleanErrorText(value: unknown): string {
  return String(value ?? "").trim();
}

function collapseWhitespace(value: string): string {
  return cleanErrorText(value).replace(/\s+/g, " ");
}

function stripHtmlTags(value: string): string {
  return value.replace(/<[^>]*>/g, " ");
}

function takePreview(value: string, maxChars: number): string {
  const normalized = collapseWhitespace(value);
  if (!normalized) return "";
  const chars = Array.from(normalized);
  if (chars.length <= maxChars) return normalized;
  return `${chars.slice(0, maxChars).join("")}...`;
}

function looksLikeHtmlErrorPage(value: string): boolean {
  const text = cleanErrorText(value);
  if (!text) return false;
  if (/<\s*html[\s>]/i.test(text) || /<!doctype\s+html/i.test(text)) return true;
  if (/<\s*(head|body|title|meta|script|style|p|div)[\s>]/i.test(text) && /<\/\s*(html|body|head|title|p|div)\s*>/i.test(text)) {
    return true;
  }
  return false;
}

function summarizeHtmlErrorPage(value: string): string {
  const text = cleanErrorText(value);
  if (!text) return "";

  const titleMatch = text.match(/<title[^>]*>([\s\S]*?)<\/title>/i);
  const title = takePreview(decodeHtmlEntities(stripHtmlTags(titleMatch?.[1] || "")), 80);

  const bodyText = collapseWhitespace(decodeHtmlEntities(stripHtmlTags(text)));
  const hints: string[] = [];
  const lower = bodyText.toLowerCase();

  if (lower.includes("cloudflare")) hints.push("Cloudflare");
  if (lower.includes("waf")) hints.push("WAF");
  if (lower.includes("region") || lower.includes("country block")) hints.push("地区限制");
  if (lower.includes("network unavailable")) hints.push("网络不可用");
  if (lower.includes("ip-access block") || lower.includes("ip access block") || lower.includes("forbidden")) {
    hints.push("IP 访问受限");
  }

  const uniqueHints = hints.filter((item, index) => hints.indexOf(item) === index);
  const hintText = uniqueHints.length > 0 ? `：${uniqueHints.join(" / ")}` : "";
  if (title) {
    return `接口返回了网页错误页（${title}）${hintText}`;
  }

  const preview = takePreview(bodyText, 120);
  if (preview) {
    return `接口返回了网页错误页：${preview}`;
  }
  return "接口返回了网页错误页";
}

function normalizeErrorText(value: string): string {
  const text = cleanErrorText(value);
  if (!text) return text;
  if (looksLikeHtmlErrorPage(text)) {
    return summarizeHtmlErrorPage(text);
  }
  return text;
}

function isGenericErrorText(value: string): boolean {
  const text = cleanErrorText(value);
  if (!text) return true;
  const normalized = text.toLowerCase();
  return (
    normalized === "unknown"
    || normalized === "error"
    || normalized === "failed"
    || normalized === "failure"
    || normalized === "request_failed"
    || normalized === "status.requestfailed"
    || /^status\.[a-z0-9_.-]+$/i.test(text)
  );
}

function tryParseStructuredErrorString(value: string, depth: number): string {
  const text = cleanErrorText(value);
  if (!text || depth > 4) return text;
  if (looksLikeHtmlErrorPage(text)) return summarizeHtmlErrorPage(text);
  const first = text[0];
  if (first !== "{" && first !== "[") return text;
  try {
    const parsed = JSON.parse(text) as unknown;
    const parsedText = toErrorMessageInner(parsed, depth + 1);
    return cleanErrorText(parsedText) || text;
  } catch {
    return text;
  }
}

function stringifyUnknownError(error: unknown): string {
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

function toErrorMessageInner(error: unknown, depth: number): string {
  if (depth > 6) return stringifyUnknownError(error);
  if (error instanceof Error) {
    const message = cleanErrorText(error.message || String(error));
    const cause = (error as Error & { cause?: unknown }).cause;
    if (cause !== undefined) {
      const causeText = toErrorMessageInner(cause, depth + 1);
      if (causeText && (isGenericErrorText(message) || !message.includes(causeText))) {
        return message && !isGenericErrorText(message) ? `${message}: ${causeText}` : causeText;
      }
    }
    return message || "unknown";
  }
  if (typeof error === "string") return tryParseStructuredErrorString(error, depth);
  if (error == null) return "unknown";
  if (typeof error !== "object") return cleanErrorText(error);

  if (Array.isArray(error)) {
    const parts = error
      .map((item) => toErrorMessageInner(item, depth + 1))
      .map(cleanErrorText)
      .filter((item) => !!item && !isGenericErrorText(item));
    if (parts.length > 0) return parts.join("; ");
    return stringifyUnknownError(error);
  }

  const record = error as Record<string, unknown>;
  let fallback = "";
  for (const key of STRUCTURED_ERROR_KEYS) {
    if (!(key in record)) continue;
    const text = cleanErrorText(toErrorMessageInner(record[key], depth + 1));
    if (!text) continue;
    if (!fallback) fallback = text;
    if (!isGenericErrorText(text)) return text;
  }
  if (fallback) return fallback;
  return stringifyUnknownError(error);
}

export function toErrorMessage(error: unknown): string {
  return normalizeErrorText(cleanErrorText(toErrorMessageInner(error, 0))) || "unknown";
}

function resolveKnownI18nErrorKey(errorMessage: string): string {
  const normalized = String(errorMessage || "").trim();
  const lower = normalized.toLowerCase();
  const hasStatusCode = (code: string) => new RegExp(`(^|\\D)${code}(\\D|$)`).test(lower);

  if (normalized === "CHAT_ABORTED_BY_USER") {
    return "status.requestAbortedByUser";
  }
  if (
    hasStatusCode("429")
    || lower.includes("rate limit")
    || lower.includes("too many requests")
    || lower.includes("quota exceeded")
  ) {
    return "status.requestRateLimited";
  }
  if (
    hasStatusCode("503")
    || lower.includes("service unavailable")
    || lower.includes("server overloaded")
    || lower.includes("overloaded")
  ) {
    return "status.requestServiceUnavailable";
  }
  if (
    hasStatusCode("401")
    || lower.includes("unauthorized")
    || lower.includes("invalid api key")
    || lower.includes("incorrect api key")
    || lower.includes("authentication failed")
  ) {
    return "status.requestUnauthorized";
  }
  if (
    hasStatusCode("403")
    || lower.includes("forbidden")
    || lower.includes("permission denied")
  ) {
    return "status.requestForbidden";
  }
  if (
    lower.includes("timed out")
    || lower.includes("timeout")
    || lower.includes("etimedout")
    || lower.includes("deadline exceeded")
  ) {
    return "status.requestTimedOut";
  }
  if (
    lower.includes("failed to fetch")
    || lower.includes("network error")
    || lower.includes("connection reset")
    || lower.includes("connection refused")
    || lower.includes("connection aborted")
    || lower.includes("dns")
    || lower.includes("unreachable")
    || lower.includes("econnreset")
    || lower.includes("econnrefused")
    || lower.includes("eai_again")
  ) {
    return "status.requestNetworkError";
  }
  if (
    hasStatusCode("404")
    || lower.includes("model not found")
    || lower.includes("no such model")
    || lower.includes("does not exist")
  ) {
    return "status.requestModelUnavailable";
  }
  if (
    lower.includes("context length")
    || lower.includes("maximum context length")
    || lower.includes("context window")
    || lower.includes("prompt is too long")
    || lower.includes("token limit")
    || lower.includes("too many tokens")
  ) {
    return "status.requestContextTooLong";
  }
  if (
    lower.includes("insufficient_quota")
    || lower.includes("quota exceeded")
    || lower.includes("billing")
    || lower.includes("credit")
    || lower.includes("balance")
    || lower.includes("payment required")
  ) {
    return "status.requestInsufficientBalance";
  }
  if (
    lower.includes("maintenance")
    || lower.includes("temporarily unavailable")
    || lower.includes("service under maintenance")
    || lower.includes("try again later")
  ) {
    return "status.requestServiceMaintenance";
  }

  return "";
}

export function formatI18nError(
  translate: (key: string, params?: Record<string, unknown>) => string,
  key: string,
  error: unknown,
): string {
  const translateWithFallback = (
    i18nKey: string,
    params: Record<string, unknown> | undefined,
    fallback: string,
  ): string => {
    const text = cleanErrorText(translate(i18nKey, params));
    return text && text !== i18nKey ? text : fallback;
  };
  const rawErr = toErrorMessage(error);
  const err = isGenericErrorText(rawErr)
    ? translateWithFallback("status.requestUnknownReason", undefined, "unknown")
    : rawErr;
  const knownKey = resolveKnownI18nErrorKey(err);
  if (knownKey) {
    const friendly = translateWithFallback(knownKey, undefined, "");
    if (err === "CHAT_ABORTED_BY_USER") {
      return friendly || err;
    }
    if (friendly) {
      return translateWithFallback(
        "status.requestFriendlyWithRaw",
        {
          reason: friendly,
          err,
        },
        `${friendly}: ${err}`,
      );
    }
  }
  return translateWithFallback(key, { err }, err);
}
