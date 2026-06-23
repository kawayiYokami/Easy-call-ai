import { describe, expect, it } from "vitest";
import { formatI18nError, toErrorMessage } from "../src/utils/error";

function createTranslator(messages: Record<string, string>) {
  return (key: string, params?: Record<string, unknown>) => {
    const template = messages[key];
    if (!template) return key;
    return template.replace(/\{(\w+)\}/g, (_, name: string) => String(params?.[name] ?? ""));
  };
}

describe("error formatting", () => {
  it("extracts detailed reasons from structured backend errors", () => {
    expect(toErrorMessage({
      error: "request_failed",
      detail: "Selected API config 'role:expert' not found.",
    })).toBe("Selected API config 'role:expert' not found.");

    expect(toErrorMessage(JSON.stringify({
      error: {
        message: "模型请求重试后仍失败: upstream 500",
      },
    }))).toBe("模型请求重试后仍失败: upstream 500");
  });

  it("does not leak i18n keys when request friendly keys are missing", () => {
    const t = createTranslator({
      "status.requestFailed": "请求失败: {err}",
      "status.requestUnknownReason": "未返回具体失败原因",
    });

    expect(formatI18nError(t, "status.requestFailed", "HTTP 429 rate limit")).toBe(
      "请求失败: HTTP 429 rate limit",
    );
    expect(formatI18nError(t, "status.requestFailed", "status.requestFailed")).toBe(
      "请求失败: 未返回具体失败原因",
    );
  });

  it("renders known request failures with friendly text and raw details", () => {
    const t = createTranslator({
      "status.requestFailed": "请求失败: {err}",
      "status.requestFriendlyWithRaw": "{reason}: {err}",
      "status.requestRateLimited": "请求过于频繁或额度受限",
      "status.requestUnknownReason": "未返回具体失败原因",
    });

    expect(formatI18nError(t, "status.requestFailed", "HTTP 429 rate limit")).toBe(
      "请求过于频繁或额度受限: HTTP 429 rate limit",
    );
  });

  it("summarizes html error pages instead of leaking full page content", () => {
    const htmlError = `Web stream error for model: <!DOCTYPE html><html><head><meta name="robots" content="noindex"><title>暂时无法访问 · evomap.ai</title></head><body><p>Cloudflare block page</p><p>REGION / NETWORK unavailable</p><p>Cloudflare WAF / IP-Access block</p></body></html>`;

    expect(toErrorMessage(htmlError)).toBe(
      "接口返回了网页错误页（暂时无法访问 · evomap.ai）：Cloudflare / WAF / 地区限制 / 网络不可用 / IP 访问受限",
    );
  });
});
