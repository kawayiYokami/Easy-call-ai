import type { ChatMessage } from "../../../types/app";
import { providerMetaWithoutStableRenderId } from "./stable-render-id";

const SYNTHETIC_MESSAGE_ID_PREFIX = "__synthetic_message__:";

function sanitizeProviderMeta(providerMeta: ChatMessage["providerMeta"] | undefined): Record<string, unknown> {
  const nextMeta = providerMetaWithoutStableRenderId(providerMeta);
  delete nextMeta._streaming;
  return nextMeta;
}

function fnv1aHash(input: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < input.length; index += 1) {
    hash ^= input.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(36);
}

function syntheticMessageSeed(message: ChatMessage): string {
  return JSON.stringify([
    String(message.role || "").trim(),
    String(message.createdAt || "").trim(),
    String(message.speakerAgentId || "").trim(),
    sanitizeProviderMeta(message.providerMeta),
    Array.isArray(message.parts) ? message.parts : [],
    Array.isArray(message.extraTextBlocks) ? message.extraTextBlocks : [],
    Array.isArray(message.toolCall) ? message.toolCall : [],
  ]);
}

export function ensureConversationMessageIds(messages: ChatMessage[]): ChatMessage[] {
  const nextMessages: ChatMessage[] = [];
  const seenIds = new Set<string>();
  const syntheticSeedCounts = new Map<string, number>();

  for (const message of Array.isArray(messages) ? messages : []) {
    const rawId = String(message?.id || "").trim();
    let nextId = rawId;

    if (!nextId) {
      const seed = syntheticMessageSeed(message);
      const occurrence = syntheticSeedCounts.get(seed) ?? 0;
      syntheticSeedCounts.set(seed, occurrence + 1);
      nextId = `${SYNTHETIC_MESSAGE_ID_PREFIX}${fnv1aHash(seed)}:${occurrence}`;
    }

    if (seenIds.has(nextId)) {
      let suffix = 1;
      let dedupedId = `${nextId}::dup:${suffix}`;
      while (seenIds.has(dedupedId)) {
        suffix += 1;
        dedupedId = `${nextId}::dup:${suffix}`;
      }
      nextId = dedupedId;
    }

    seenIds.add(nextId);
    nextMessages.push(nextId === rawId ? message : { ...message, id: nextId });
  }

  return nextMessages;
}

