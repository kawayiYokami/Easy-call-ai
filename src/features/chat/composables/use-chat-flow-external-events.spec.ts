import { describe, expect, it } from "vitest";
import {
  externalTerminalTargetsRound,
} from "./use-chat-flow-external-events";

describe("external chat terminal identity", () => {
  it("rejects a terminal from another activation", () => {
    expect(externalTerminalTargetsRound(
      { phase: "streaming", gen: 2, messageId: "assistant-new" },
      "activation-new",
      { activationId: "activation-old" },
    )).toBe(false);
  });

  it("rejects a formal completion for another assistant message", () => {
    expect(externalTerminalTargetsRound(
      { phase: "streaming", gen: 2, messageId: "assistant-new" },
      "",
      { assistantMessageId: "assistant-old" },
    )).toBe(false);
  });

  it("keeps legacy terminal payloads without identity usable", () => {
    expect(externalTerminalTargetsRound(
      { phase: "queued", gen: 1, messageId: "assistant-1" },
      "activation-1",
      {},
    )).toBe(true);
  });
});
