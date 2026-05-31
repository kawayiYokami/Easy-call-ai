import { describe, expect, it } from "vitest";
import {
  contactCommunicationToggleClass,
  contactCommunicationToggleEnabled,
  parseKeywordList,
} from "../src/features/config/views/config-tabs/remote-im/helpers";

describe("remote im helpers", () => {
  it("normalizes fullwidth and halfwidth comma separated keyword lists", () => {
    expect(parseKeywordList("闭嘴， 张嘴, 闭嘴\n继续说\r\n张嘴")).toEqual([
      "闭嘴",
      "张嘴",
      "继续说",
    ]);
  });

  it("treats receive-only legacy contacts as enabled in the merged communication toggle", () => {
    expect(contactCommunicationToggleEnabled({ allowReceive: true, allowSend: false })).toBe(true);
    expect(contactCommunicationToggleClass({ allowReceive: true, allowSend: false })).toBe("toggle-success");
  });

  it("marks the merged contact communication toggle enabled when either direction is on", () => {
    expect(contactCommunicationToggleEnabled({ allowReceive: true, allowSend: true })).toBe(true);
    expect(contactCommunicationToggleClass({ allowReceive: true, allowSend: true })).toBe("toggle-success");
    expect(contactCommunicationToggleEnabled({ allowReceive: false, allowSend: true })).toBe(true);
    expect(contactCommunicationToggleClass({ allowReceive: false, allowSend: true })).toBe("toggle-success");
    expect(contactCommunicationToggleEnabled({ allowReceive: false, allowSend: false })).toBe(false);
    expect(contactCommunicationToggleClass({ allowReceive: false, allowSend: false })).toBe("");
  });
});
