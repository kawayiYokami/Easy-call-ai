import type { ApiRequestFormat } from "../../../types/app";
import { normalizeApiRequestFormat } from "./api-request-format";

export type MultimodalProtocolFamily = "openai" | "gemini" | "anthropic";

function normalizeModelName(modelName: unknown): string {
  return String(modelName ?? "").trim().toLowerCase();
}

export function detectMultimodalModelFamily(modelName: unknown): MultimodalProtocolFamily | null {
  const normalized = normalizeModelName(modelName);
  if (!normalized) return null;
  if (normalized.includes("gemini") || normalized.includes("gemma")) return "gemini";
  if (normalized.includes("minimax")) return "anthropic";
  if (normalized.includes("qwen") || normalized.includes("mimo") || normalized.includes("gpt")) return "openai";
  return null;
}

export function resolveMultimodalProtocolFamily(
  requestFormat: unknown,
  modelName: unknown,
): MultimodalProtocolFamily | null {
  const normalized = normalizeApiRequestFormat(requestFormat);
  const modelFamily = detectMultimodalModelFamily(modelName);
  if (!modelFamily) return null;
  if (normalized === "auto") {
    return modelFamily;
  }
  if (normalized === "gemini") {
    return "gemini";
  }
  if (normalized === "anthropic") {
    return "anthropic";
  }
  if (normalized === "openai" || normalized === "openai_responses") {
    return "openai";
  }
  return null;
}

export function supportsMultimodalRouting(
  requestFormat: unknown,
  modelName: unknown,
): boolean {
  const modelFamily = detectMultimodalModelFamily(modelName);
  if (!modelFamily) return false;
  return resolveMultimodalProtocolFamily(requestFormat, modelName) === modelFamily;
}

export function supportsMultimodalCapabilityToggles(
  _requestFormat: ApiRequestFormat | string | undefined,
  modelName: unknown,
): boolean {
  return detectMultimodalModelFamily(modelName) !== null;
}
