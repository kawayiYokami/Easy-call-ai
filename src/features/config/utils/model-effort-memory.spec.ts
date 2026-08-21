import { describe, expect, it } from "vitest";
import type { ApiConfigItem } from "../../../types/app";
import { apiConfigModelGroupKey } from "./api-config-selection-tree";
import { buildModelGroupIndex, resolveModelEffortSelection } from "./model-effort-memory";

function item(id: string, effort: string): ApiConfigItem {
  return {
    id: `prov::${id}`,
    name: `prov/model`,
    model: "model-a",
    reasoningEffort: effort,
  } as ApiConfigItem;
}

function groupKeyOf(options: ApiConfigItem[]): string {
  return apiConfigModelGroupKey(options[0]);
}

function entryOf(options: ApiConfigItem[], id: string) {
  return buildModelGroupIndex(options).get(`prov::${id}`);
}

describe("resolveModelEffortSelection 思维等级记忆与降级", () => {
  it("记忆命中：直接落到记忆等级的叶子", () => {
    const options = [item("d", "default"), item("h", "high")];
    const memory = { [groupKeyOf(options)]: "high" };
    const entry = entryOf(options, "d");
    const result = entry && resolveModelEffortSelection(entry.items, entry.groupKey, "prov::d", memory);
    expect(result?.leafId).toBe("prov::h");
    expect(result?.effort).toBe("high");
  });

  it("无记忆：落 default，不继承当前模型档位", () => {
    const options = [item("d", "default"), item("h", "high")];
    const entry = entryOf(options, "h");
    const result = entry && resolveModelEffortSelection(entry.items, entry.groupKey, "prov::h", {});
    expect(result?.leafId).toBe("prov::d");
    expect(result?.effort).toBe("default");
  });

  it("记忆等级不可用：优先 default", () => {
    const options = [item("d", "default"), item("x", "xhigh")];
    const memory = { [groupKeyOf(options)]: "high" };
    const entry = entryOf(options, "x");
    const result = entry && resolveModelEffortSelection(entry.items, entry.groupKey, "prov::x", memory);
    expect(result?.leafId).toBe("prov::d");
    expect(result?.effort).toBe("default");
  });

  it("无 default：按全局档位刻度取距离 default 最近者", () => {
    const lowHigh = [item("l", "low"), item("h", "high")];
    const entry = entryOf(lowHigh, "l");
    // low(3) 比 high(5) 距 default(0) 更近
    expect(resolveModelEffortSelection(entry!.items, entry!.groupKey, "prov::l", {})?.effort).toBe("low");
    // none(1) 是三者中距 default 最近的
    const noneMinimal = [item("n", "none"), item("m", "minimal"), item("h", "high")];
    const entry2 = entryOf(noneMinimal, "n");
    expect(resolveModelEffortSelection(entry2!.items, entry2!.groupKey, "prov::n", {})?.effort).toBe("none");
  });

  it("单档位模型组：直接返回请求的叶子", () => {
    const options = [item("only", "high")];
    const entry = entryOf(options, "only");
    const result = entry && resolveModelEffortSelection(entry.items, entry.groupKey, "prov::only", {});
    expect(result?.leafId).toBe("prov::only");
  });

  it("同模型不同配置分组互不干扰", () => {
    const sameModelOtherConfig = { ...item("t1", "default"), customTemperatureEnabled: true } as ApiConfigItem;
    const options = [item("d", "default"), item("h", "high"), sameModelOtherConfig];
    // t1 配置不同 → 单档位组，直接生效
    const entry = entryOf(options, "t1");
    expect(resolveModelEffortSelection(entry!.items, entry!.groupKey, "prov::t1", {})?.leafId).toBe("prov::t1");
    // d/h 组的记忆不影响 t1 组
    const other = { [apiConfigModelGroupKey(sameModelOtherConfig)]: "high" };
    expect(resolveModelEffortSelection(entry!.items, entry!.groupKey, "prov::t1", other)?.leafId).toBe("prov::t1");
  });

  it("未知档位值：记忆命中未知档时仍能解析出叶子", () => {
    const options = [item("u", "turbo"), item("d", "default")];
    const memory = { [groupKeyOf(options)]: "turbo" };
    const entry = entryOf(options, "d");
    expect(resolveModelEffortSelection(entry!.items, entry!.groupKey, "prov::d", memory)?.effort).toBe("turbo");
  });
});
