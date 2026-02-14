<template>
  <div v-if="selectedApiConfig.requestFormat === 'gemini'" class="form-control mt-1 rounded-box border border-base-300 p-2">
    <div class="label py-0">
      <span class="label-text text-xs font-medium">Gemini 思考控制</span>
    </div>

    <label v-if="modelKind === 'v3_pro' || modelKind === 'v3_flash'" class="form-control mt-1">
      <div class="label py-0"><span class="label-text text-xs opacity-75">thinkingLevel</span></div>
      <select v-model="selectedApiConfig.geminiThinkingLevel" class="select select-bordered select-xs">
        <option v-if="modelKind === 'v3_flash'" value="minimal">minimal</option>
        <option value="high">high</option>
        <option v-if="modelKind === 'v3_flash'" value="medium">medium</option>
        <option value="low">low</option>
      </select>
    </label>

    <label v-if="modelKind === 'v2_5'" class="form-control mt-1">
      <div class="label py-0"><span class="label-text text-xs opacity-75">thinkingBudget</span></div>
      <input
        v-model.number="selectedApiConfig.geminiThinkingBudget"
        type="number"
        min="-1"
        step="1"
        class="input input-bordered input-xs"
      />
      <div class="mt-1 text-[10px] opacity-65">`-1` 表示动态预算（由模型决定）。</div>
    </label>

    <div v-if="autoHint" class="mt-1 text-[10px] opacity-70">
      {{ autoHint }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, watchEffect } from "vue";
import type { ApiConfigItem } from "../../../../types/app";

const props = defineProps<{
  selectedApiConfig: ApiConfigItem;
}>();

function hasAlphaUnit(text: string, unit: string): boolean {
  return new RegExp(`(^|[^a-z0-9])${unit}([^a-z0-9]|$)`, "i").test(text);
}

function hasNumberUnit(text: string, unit: "2.5" | "3"): boolean {
  const escaped = unit === "2.5" ? "2\\.5" : "3";
  return new RegExp(`(^|[^0-9])${escaped}([^0-9]|$)`).test(text);
}

const autoHint = computed(() => {
  const model = String(props.selectedApiConfig.model || "").trim().toLowerCase();
  if (!hasAlphaUnit(model, "gemini")) return "未命中 gemini 关键字，不显示思考参数。";
  if (hasNumberUnit(model, "2.5")) return "当前模型匹配 2.5：使用 thinkingBudget。";
  if (hasNumberUnit(model, "3") && hasAlphaUnit(model, "flash")) return "当前模型匹配 3 Flash：支持 minimal/low/medium/high。";
  if (hasNumberUnit(model, "3") && hasAlphaUnit(model, "pro")) return "当前模型匹配 3 Pro：支持 low/high。";
  if (hasNumberUnit(model, "3")) return "当前模型匹配 3：默认按 Pro 规则（low/high）。";
  return "未命中 2.5/3 规则，不追加 thinkingConfig。";
});

const modelKind = computed<"v2_5" | "v3_pro" | "v3_flash" | "other">(() => {
  const model = String(props.selectedApiConfig.model || "").trim().toLowerCase();
  if (!hasAlphaUnit(model, "gemini")) return "other";
  if (hasNumberUnit(model, "2.5")) return "v2_5";
  if (hasNumberUnit(model, "3") && hasAlphaUnit(model, "flash")) return "v3_flash";
  if (hasNumberUnit(model, "3")) return "v3_pro";
  return "other";
});

watchEffect(() => {
  if (props.selectedApiConfig.requestFormat !== "gemini") return;
  if (!props.selectedApiConfig.geminiThinkingLevel) props.selectedApiConfig.geminiThinkingLevel = "high";
  if (modelKind.value === "v3_pro") {
    if (props.selectedApiConfig.geminiThinkingLevel !== "low" && props.selectedApiConfig.geminiThinkingLevel !== "high") {
      props.selectedApiConfig.geminiThinkingLevel = "high";
    }
  }
  if (modelKind.value === "v3_flash") {
    if (
      props.selectedApiConfig.geminiThinkingLevel !== "minimal"
      && props.selectedApiConfig.geminiThinkingLevel !== "low"
      && props.selectedApiConfig.geminiThinkingLevel !== "medium"
      && props.selectedApiConfig.geminiThinkingLevel !== "high"
    ) {
      props.selectedApiConfig.geminiThinkingLevel = "high";
    }
  }
  if (!Number.isFinite(props.selectedApiConfig.geminiThinkingBudget)) {
    props.selectedApiConfig.geminiThinkingBudget = -1;
  }
});
</script>
