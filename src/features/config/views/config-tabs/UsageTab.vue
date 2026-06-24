<template>
  <div class="space-y-5">
    <div class="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
      <div>
        <div class="text-xl font-semibold">用量概览</div>
        <div class="mt-1 text-sm opacity-70">综合总量按 输出 x 2 + 缓存写入 + 缓存命中 x 0.02 估算，仅供参考。</div>
      </div>
      <button class="btn btn-primary btn-sm shrink-0" :disabled="loading" @click="loadOverview">
        <span v-if="loading" class="loading loading-spinner loading-xs"></span>
        <span>{{ t("common.refresh") }}</span>
      </button>
    </div>

    <div v-if="loading && !overview" class="rounded-box border border-base-300 bg-base-100 p-4">
      <div class="mb-3 text-sm opacity-70">{{ t("common.loading") }}</div>
      <progress class="progress progress-primary w-full"></progress>
    </div>

    <template v-else-if="overview">
      <section class="rounded-box border border-base-300 bg-base-100 shadow-sm">
        <div class="stats stats-vertical w-full lg:stats-horizontal">
          <div v-for="item in summaryStats" :key="item.label" class="stat">
            <div class="stat-title text-xs">{{ item.label }}</div>
            <div class="stat-value text-2xl">{{ item.value }}</div>
            <div v-if="item.desc" class="stat-desc">{{ item.desc }}</div>
          </div>
        </div>
      </section>

      <div class="grid gap-4 xl:grid-cols-2">
        <section class="card border border-base-300 bg-base-100 shadow-sm">
          <div class="card-body p-4">
            <div class="card-title text-base">模型用量</div>
            <div class="mt-1 text-xs opacity-60">新累计按供应商和模型细分；历史未细分残留会并入当前推断的供应商/模型。</div>
            <div class="mt-2 overflow-x-auto">
              <table class="table table-sm">
                <thead>
                  <tr>
                    <th><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleProviderModelSort('providerLabel')">供应商{{ sortIndicator(providerModelSort, 'providerLabel') }}</button></th>
                    <th><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleProviderModelSort('modelName')">模型{{ sortIndicator(providerModelSort, 'modelName') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleProviderModelSort('weightedTokens')">综合总量{{ sortIndicator(providerModelSort, 'weightedTokens') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleProviderModelSort('totalTokens')">总输入{{ sortIndicator(providerModelSort, 'totalTokens') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleProviderModelSort('cacheHitRate')">缓存命中率{{ sortIndicator(providerModelSort, 'cacheHitRate') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleProviderModelSort('outputTokens')">输出{{ sortIndicator(providerModelSort, 'outputTokens') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleProviderModelSort('cacheReadTokens')">缓存读{{ sortIndicator(providerModelSort, 'cacheReadTokens') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleProviderModelSort('cacheWrite')">缓存写入{{ sortIndicator(providerModelSort, 'cacheWrite') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleProviderModelSort('conversationCount')">会话{{ sortIndicator(providerModelSort, 'conversationCount') }}</button></th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="item in sortedProviderModels" :key="`provider-model-${item.key}`">
                    <td class="min-w-28">{{ item.providerLabel }}</td>
                    <td class="min-w-36">{{ item.modelName }}</td>
                    <td class="text-right">{{ formatTokens(item.weightedTokens) }}</td>
                    <td class="text-right">{{ formatTokens(totalInputTokens(item)) }}</td>
                    <td class="text-right">{{ formatPercent(cacheHitRate(item)) }}</td>
                    <td class="text-right">{{ formatTokens(item.outputTokens) }}</td>
                    <td class="text-right">{{ formatTokens(item.cacheReadTokens) }}</td>
                    <td class="text-right">{{ formatTokens(cacheWriteAmount(item)) }}</td>
                    <td class="text-right">{{ item.conversationCount }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </section>

        <section class="card border border-base-300 bg-base-100 shadow-sm">
          <div class="card-body p-4">
            <div class="card-title text-base">人格用量</div>
            <div class="mt-2 overflow-x-auto">
              <table class="table table-sm">
                <thead>
                  <tr>
                    <th><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleAgentSort('label')">人格{{ sortIndicator(agentSort, 'label') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleAgentSort('weightedTokens')">综合总量{{ sortIndicator(agentSort, 'weightedTokens') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleAgentSort('totalTokens')">总输入{{ sortIndicator(agentSort, 'totalTokens') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleAgentSort('cacheHitRate')">缓存命中率{{ sortIndicator(agentSort, 'cacheHitRate') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleAgentSort('outputTokens')">输出{{ sortIndicator(agentSort, 'outputTokens') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleAgentSort('cacheReadTokens')">缓存读{{ sortIndicator(agentSort, 'cacheReadTokens') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleAgentSort('cacheWrite')">缓存写入{{ sortIndicator(agentSort, 'cacheWrite') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleAgentSort('conversationCount')">会话{{ sortIndicator(agentSort, 'conversationCount') }}</button></th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="item in sortedAgents" :key="`agent-${item.key}`">
                    <td class="min-w-36">{{ item.label }}</td>
                    <td class="text-right">{{ formatTokens(item.weightedTokens) }}</td>
                    <td class="text-right">{{ formatTokens(totalInputTokens(item)) }}</td>
                    <td class="text-right">{{ formatPercent(cacheHitRate(item)) }}</td>
                    <td class="text-right">{{ formatTokens(item.outputTokens) }}</td>
                    <td class="text-right">{{ formatTokens(item.cacheReadTokens) }}</td>
                    <td class="text-right">{{ formatTokens(cacheWriteAmount(item)) }}</td>
                    <td class="text-right">{{ item.conversationCount }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </section>

        <section class="card border border-base-300 bg-base-100 shadow-sm">
          <div class="card-body p-4">
            <div class="card-title text-base">会话类型用量</div>
            <div class="mt-2 overflow-x-auto">
              <table class="table table-sm">
                <thead>
                  <tr>
                    <th><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleKindSort('label')">类型{{ sortIndicator(kindSort, 'label') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleKindSort('weightedTokens')">综合总量{{ sortIndicator(kindSort, 'weightedTokens') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleKindSort('totalTokens')">总输入{{ sortIndicator(kindSort, 'totalTokens') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleKindSort('cacheHitRate')">缓存命中率{{ sortIndicator(kindSort, 'cacheHitRate') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleKindSort('outputTokens')">输出{{ sortIndicator(kindSort, 'outputTokens') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleKindSort('cacheReadTokens')">缓存读{{ sortIndicator(kindSort, 'cacheReadTokens') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleKindSort('cacheWrite')">缓存写入{{ sortIndicator(kindSort, 'cacheWrite') }}</button></th>
                    <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleKindSort('conversationCount')">会话{{ sortIndicator(kindSort, 'conversationCount') }}</button></th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="item in sortedKinds" :key="`kind-${item.key}`">
                    <td>{{ item.label }}</td>
                    <td class="text-right">{{ formatTokens(item.weightedTokens) }}</td>
                    <td class="text-right">{{ formatTokens(totalInputTokens(item)) }}</td>
                    <td class="text-right">{{ formatPercent(cacheHitRate(item)) }}</td>
                    <td class="text-right">{{ formatTokens(item.outputTokens) }}</td>
                    <td class="text-right">{{ formatTokens(item.cacheReadTokens) }}</td>
                    <td class="text-right">{{ formatTokens(cacheWriteAmount(item)) }}</td>
                    <td class="text-right">{{ item.conversationCount }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </section>
      </div>

      <section class="card border border-base-300 bg-base-100 shadow-sm">
        <div class="card-body p-4">
          <div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
            <div>
              <div class="card-title text-base">会话用量</div>
              <div class="text-xs opacity-60">按综合总量排序，方便直接定位最耗的会话。</div>
            </div>
            <div class="flex flex-col gap-2 md:items-end">
              <div class="join">
                <label
                  v-for="option in conversationFilterOptions"
                  :key="option.value"
                  class="btn btn-sm join-item border-base-300"
                  :class="conversationFilter === option.value ? 'btn-primary' : 'bg-base-200 text-base-content hover:bg-base-300'"
                >
                  <input
                    class="sr-only"
                    type="radio"
                    name="usage-conversation-filter"
                    :checked="conversationFilter === option.value"
                    @change="setConversationFilter(option.value)"
                  />
                  {{ option.label }}
                </label>
              </div>
            </div>
          </div>
          <div class="mt-3 overflow-x-auto">
            <table class="table table-sm">
              <thead>
                <tr>
                  <th><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleConversationSort('activityAt')">会话{{ sortIndicator(conversationSort, 'activityAt') }}</button></th>
                  <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleConversationSort('weightedTokens')">综合总量{{ sortIndicator(conversationSort, 'weightedTokens') }}</button></th>
                  <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleConversationSort('totalTokens')">总输入{{ sortIndicator(conversationSort, 'totalTokens') }}</button></th>
                  <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleConversationSort('cacheHitRate')">缓存命中率{{ sortIndicator(conversationSort, 'cacheHitRate') }}</button></th>
                  <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleConversationSort('outputTokens')">输出{{ sortIndicator(conversationSort, 'outputTokens') }}</button></th>
                  <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleConversationSort('cacheReadTokens')">缓存读{{ sortIndicator(conversationSort, 'cacheReadTokens') }}</button></th>
                  <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleConversationSort('cacheWrite')">缓存写入{{ sortIndicator(conversationSort, 'cacheWrite') }}</button></th>
                  <th class="text-right"><button class="btn btn-ghost btn-xs px-1 font-semibold" @click="toggleConversationSort('messageCount')">消息{{ sortIndicator(conversationSort, 'messageCount') }}</button></th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="item in pagedConversations" :key="item.conversationId">
                  <td class="min-w-64">
                    <div class="flex items-start gap-3">
                      <div class="avatar shrink-0">
                        <div class="flex h-10 w-10 items-center justify-center rounded-full bg-neutral text-neutral-content">
                          <img
                            v-if="conversationAvatarUrl(item)"
                            :src="conversationAvatarUrl(item)"
                            :alt="conversationAvatarLabel(item)"
                            class="h-10 w-10 rounded-full object-cover"
                          />
                          <span v-else class="text-sm font-bold">{{ conversationAvatarInitial(item) }}</span>
                        </div>
                      </div>
                      <div class="min-w-0">
                        <div class="truncate font-medium">{{ displayConversationTitle(item) }}</div>
                        <div class="text-[11px] opacity-60">{{ item.conversationId }}</div>
                        <div class="text-[11px] opacity-60">{{ formatConversationMeta(item) }}</div>
                      </div>
                    </div>
                  </td>
                  <td class="text-right font-medium">{{ formatTokens(item.weightedTokens) }}</td>
                  <td class="text-right">{{ formatTokens(totalInputTokens(item)) }}</td>
                  <td class="text-right">{{ formatPercent(cacheHitRate(item)) }}</td>
                  <td class="text-right">{{ formatTokens(item.outputTokens) }}</td>
                  <td class="text-right">{{ formatTokens(item.cacheReadTokens) }}</td>
                  <td class="text-right">{{ formatTokens(cacheWriteAmount(item)) }}</td>
                  <td class="text-right">{{ item.messageCount }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div class="mt-3 flex flex-col gap-2 text-sm md:flex-row md:items-center md:justify-between">
            <div class="opacity-70">
              第 {{ conversationPage }} / {{ conversationPageCount }} 页
              <span class="mx-1">·</span>
              显示 {{ conversationPageStart }}-{{ conversationPageEnd }}
            </div>
            <div class="join">
              <button
                class="btn btn-sm join-item"
                type="button"
                :disabled="conversationPage <= 1"
                @click="conversationPage -= 1"
              >
                上一页
              </button>
              <button
                class="btn btn-sm join-item"
                type="button"
                :disabled="conversationPage >= conversationPageCount"
                @click="conversationPage += 1"
              >
                下一页
              </button>
            </div>
          </div>
        </div>
      </section>
    </template>

    <div v-else class="rounded-box border border-base-300 bg-base-100 p-4 text-sm opacity-70">
      暂无用量数据。
    </div>

    <div v-if="errorText" class="alert alert-error">
      <span>{{ errorText }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watchEffect } from "vue";
import { useI18n } from "vue-i18n";
import { invokeTauri } from "../../../../services/tauri-api";
import type {
  PersonaProfile,
  UsageAggregateItem,
  UsageConversationItem,
  UsageOverview,
  UsageProviderModelAggregateItem,
} from "../../../../types/app";
import { useAvatarCache } from "../../../chat/composables/use-avatar-cache";
import { resolveConversationDisplayTitle } from "../../../chat/utils/conversation-title";
import { toErrorMessage } from "../../../../utils/error";

const { t } = useI18n();

const loading = ref(false);
const errorText = ref("");
const overview = ref<UsageOverview | null>(null);
const conversationPage = ref(1);
const conversationPageSize = 20;
const conversationFilter = ref<"all" | "normal" | "delegate" | "contact" | "system" | "archived">("all");
const { resolveAvatarUrl, ensureAvatarCached } = useAvatarCache({ personas: ref<PersonaProfile[]>([]) });
type SortDirection = "asc" | "desc";
type SortState<T extends string> = {
  key: T;
  direction: SortDirection;
};
type ProviderModelSortKey =
  | "providerLabel"
  | "modelName"
  | "weightedTokens"
  | "totalTokens"
  | "cacheHitRate"
  | "outputTokens"
  | "cacheReadTokens"
  | "cacheWrite"
  | "conversationCount";
type AggregateSortKey =
  | "label"
  | "weightedTokens"
  | "totalTokens"
  | "cacheHitRate"
  | "outputTokens"
  | "cacheReadTokens"
  | "cacheWrite"
  | "conversationCount";
type ConversationSortKey =
  | "activityAt"
  | "weightedTokens"
  | "totalTokens"
  | "cacheHitRate"
  | "outputTokens"
  | "cacheReadTokens"
  | "cacheWrite"
  | "messageCount";

const providerModelSort = ref<SortState<ProviderModelSortKey>>({ key: "weightedTokens", direction: "desc" });
const agentSort = ref<SortState<AggregateSortKey>>({ key: "weightedTokens", direction: "desc" });
const kindSort = ref<SortState<AggregateSortKey>>({ key: "weightedTokens", direction: "desc" });
const conversationSort = ref<SortState<ConversationSortKey>>({ key: "weightedTokens", direction: "desc" });

const conversationFilterOptions = [
  { value: "all" as const, label: "全部" },
  { value: "normal" as const, label: "普通" },
  { value: "delegate" as const, label: "委托" },
  { value: "contact" as const, label: "远程" },
  { value: "system" as const, label: "通知" },
  { value: "archived" as const, label: "归档" },
];

const summaryStats = computed(() => {
  if (!overview.value) return [];
  const totals = overview.value.totals;
  const overallCacheHitRate = cacheHitRate(totals);
  return [
    {
      label: "综合总量",
      value: formatTokens(totals.weightedTokens),
      desc: "仅供参考",
    },
    {
      label: "总输出",
      value: formatTokens(totals.outputTokens),
      desc: " ",
    },
    {
      label: "思维链",
      value: formatTokens(totals.reasoningTokens),
      desc: " ",
    },
    {
      label: "总缓存读取",
      value: formatTokens(totals.cacheReadTokens),
      desc: " ",
    },
    {
      label: "缓存写入",
      value: formatTokens(cacheWriteAmount(totals)),
      desc: " ",
    },
    {
      label: "平均缓存命中率",
      value: formatPercent(overallCacheHitRate),
      desc: "未排除无法缓存的",
    },
  ];
});
const sortedProviderModels = computed(() =>
  sortProviderModelItems(overview.value?.byProviderModel || [], providerModelSort.value).slice(0, 12),
);
const sortedAgents = computed(() =>
  sortAggregateItems(overview.value?.byAgent || [], agentSort.value).slice(0, 12),
);
const sortedKinds = computed(() =>
  sortAggregateItems(overview.value?.byKind || [], kindSort.value),
);
const sortedFilteredConversations = computed(() => {
  const items = overview.value?.conversations || [];
  const filtered = items.filter((item) => conversationMatchesFilter(item, conversationFilter.value));
  return sortConversationItems(filtered, conversationSort.value);
});
const filteredConversationCount = computed(() => sortedFilteredConversations.value.length);
const conversationPageCount = computed(() => Math.max(1, Math.ceil(filteredConversationCount.value / conversationPageSize)));
const pagedConversations = computed(() => {
  const items = sortedFilteredConversations.value;
  const start = (conversationPage.value - 1) * conversationPageSize;
  return items.slice(start, start + conversationPageSize);
});
const conversationPageStart = computed(() => {
  const total = filteredConversationCount.value;
  if (total === 0) return 0;
  return (conversationPage.value - 1) * conversationPageSize + 1;
});
const conversationPageEnd = computed(() => {
  const total = filteredConversationCount.value;
  return Math.min(total, conversationPage.value * conversationPageSize);
});

watchEffect(() => {
  for (const item of pagedConversations.value) {
    if (!item.avatarPath) continue;
    void ensureAvatarCached(item.avatarPath, item.avatarUpdatedAt);
  }
});

function formatTokens(value: number): string {
  const numeric = Number(value || 0);
  if (!Number.isFinite(numeric)) return "0";
  const abs = Math.abs(numeric);
  const units = [
    { threshold: 1_000_000_000_000, suffix: "T" },
    { threshold: 1_000_000_000, suffix: "B" },
    { threshold: 1_000_000, suffix: "M" },
    { threshold: 1_000, suffix: "K" },
  ];
  for (const unit of units) {
    if (abs >= unit.threshold) {
      const scaled = numeric / unit.threshold;
      const digits = Math.abs(scaled) >= 100 ? 0 : Math.abs(scaled) >= 10 ? 1 : 2;
      return `${scaled.toFixed(digits).replace(/\.0+$|(\.\d*[1-9])0+$/, "$1")}${unit.suffix}`;
    }
  }
  return new Intl.NumberFormat("zh-CN").format(Math.round(numeric));
}

function totalInputTokens(item: { totalTokens: number }): number {
  return Math.max(0, Number(item.totalTokens || 0));
}

function cacheWriteAmount(item: { inputTokens: number; cacheReadTokens: number }): number {
  return Math.max(0, Number(item.inputTokens || 0) - Number(item.cacheReadTokens || 0));
}

function cacheHitRate(item: { totalTokens: number; cacheReadTokens: number }): number {
  const total = totalInputTokens(item);
  if (total <= 0) return 0;
  return Number(item.cacheReadTokens || 0) / total;
}

function formatPercent(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "0%";
  return `${(value * 100).toFixed(value >= 0.1 ? 0 : 1).replace(/\.0$/, "")}%`;
}

function formatDateTime(value?: string | null): string {
  const text = String(value || "").trim();
  if (!text) return "-";
  const date = new Date(text);
  if (Number.isNaN(date.getTime())) return text;
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")} ${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
}

function timeValue(value?: string | null): number {
  const text = String(value || "").trim();
  if (!text) return 0;
  const timestamp = new Date(text).getTime();
  return Number.isFinite(timestamp) ? timestamp : 0;
}

function compareText(left: string, right: string): number {
  return left.localeCompare(right, "zh-CN");
}

function compareNumber(left: number, right: number): number {
  return left - right;
}

function applyDirection(result: number, direction: SortDirection): number {
  return direction === "asc" ? result : -result;
}

function cacheWriteSortValue(item: { inputTokens: number; cacheReadTokens: number }): number {
  return cacheWriteAmount(item);
}

function providerModelSortValue(item: UsageProviderModelAggregateItem, key: ProviderModelSortKey): number | string {
  if (key === "providerLabel") return item.providerLabel || "";
  if (key === "modelName") return item.modelName || "";
  if (key === "weightedTokens") return item.weightedTokens || 0;
  if (key === "totalTokens") return item.totalTokens || 0;
  if (key === "cacheHitRate") return cacheHitRate(item);
  if (key === "outputTokens") return item.outputTokens || 0;
  if (key === "cacheReadTokens") return item.cacheReadTokens || 0;
  if (key === "cacheWrite") return cacheWriteSortValue(item);
  return item.conversationCount || 0;
}

function aggregateSortValue(item: UsageAggregateItem, key: AggregateSortKey): number | string {
  if (key === "label") return item.label || "";
  if (key === "weightedTokens") return item.weightedTokens || 0;
  if (key === "totalTokens") return item.totalTokens || 0;
  if (key === "cacheHitRate") return cacheHitRate(item);
  if (key === "outputTokens") return item.outputTokens || 0;
  if (key === "cacheReadTokens") return item.cacheReadTokens || 0;
  if (key === "cacheWrite") return cacheWriteSortValue(item);
  return item.conversationCount || 0;
}

function conversationSortValue(item: UsageConversationItem, key: ConversationSortKey): number {
  if (key === "activityAt") {
    return item.archivedAt ? timeValue(item.archivedAt) : timeValue(item.updatedAt);
  }
  if (key === "weightedTokens") return item.weightedTokens || 0;
  if (key === "totalTokens") return item.totalTokens || 0;
  if (key === "cacheHitRate") return cacheHitRate(item);
  if (key === "outputTokens") return item.outputTokens || 0;
  if (key === "cacheReadTokens") return item.cacheReadTokens || 0;
  if (key === "cacheWrite") return cacheWriteSortValue(item);
  return item.messageCount || 0;
}

function sortByValue<T>(
  items: T[],
  getter: (item: T) => number | string,
  direction: SortDirection,
  fallback: (left: T, right: T) => number,
): T[] {
  return [...items].sort((left, right) => {
    const leftValue = getter(left);
    const rightValue = getter(right);
    const result = typeof leftValue === "string" && typeof rightValue === "string"
      ? compareText(leftValue, rightValue)
      : compareNumber(Number(leftValue || 0), Number(rightValue || 0));
    if (result !== 0) return applyDirection(result, direction);
    return fallback(left, right);
  });
}

function sortProviderModelItems(
  items: UsageProviderModelAggregateItem[],
  sort: SortState<ProviderModelSortKey>,
): UsageProviderModelAggregateItem[] {
  return sortByValue(
    items,
    (item) => providerModelSortValue(item, sort.key),
    sort.direction,
    (left, right) => compareText(left.key, right.key),
  );
}

function sortAggregateItems(
  items: UsageAggregateItem[],
  sort: SortState<AggregateSortKey>,
): UsageAggregateItem[] {
  return sortByValue(
    items,
    (item) => aggregateSortValue(item, sort.key),
    sort.direction,
    (left, right) => compareText(left.key, right.key),
  );
}

function sortConversationItems(
  items: UsageConversationItem[],
  sort: SortState<ConversationSortKey>,
): UsageConversationItem[] {
  return sortByValue(
    items,
    (item) => conversationSortValue(item, sort.key),
    sort.direction,
    (left, right) => compareText(left.conversationId, right.conversationId),
  );
}

function nextSortState<T extends string>(current: SortState<T>, key: T): SortState<T> {
  if (current.key === key) {
    return {
      key,
      direction: current.direction === "desc" ? "asc" : "desc",
    };
  }
  return { key, direction: "desc" };
}

function toggleProviderModelSort(key: ProviderModelSortKey) {
  providerModelSort.value = nextSortState(providerModelSort.value, key);
}

function toggleAgentSort(key: AggregateSortKey) {
  agentSort.value = nextSortState(agentSort.value, key);
}

function toggleKindSort(key: AggregateSortKey) {
  kindSort.value = nextSortState(kindSort.value, key);
}

function toggleConversationSort(key: ConversationSortKey) {
  conversationSort.value = nextSortState(conversationSort.value, key);
  conversationPage.value = 1;
}

function sortIndicator<T extends string>(sort: { key: T; direction: SortDirection }, key: T): string {
  if (sort.key !== key) return "";
  return sort.direction === "desc" ? " ↓" : " ↑";
}

function conversationMatchesFilter(
  item: UsageConversationItem,
  filter: "all" | "normal" | "delegate" | "contact" | "system" | "archived",
): boolean {
  if (filter === "all") return true;
  if (filter === "system") return item.isSystemNotificationConversation;
  if (filter === "delegate") return item.isDelegate;
  if (filter === "contact") return item.conversationKind === "remote_im_contact";
  if (filter === "archived") return !!item.archivedAt;
  return !item.isSystemNotificationConversation
    && !item.isDelegate
    && item.conversationKind !== "remote_im_contact"
    && !item.archivedAt;
}

function setConversationFilter(value: "all" | "normal" | "delegate" | "contact" | "system" | "archived") {
  conversationFilter.value = value;
  conversationPage.value = 1;
}

function formatConversationMeta(item: UsageConversationItem): string {
  if (item.archivedAt) {
    return `归档: ${formatDateTime(item.archivedAt)}`;
  }
  return `更新: ${formatDateTime(item.updatedAt)}`;
}

function displayConversationTitle(item: UsageConversationItem): string {
  return resolveConversationDisplayTitle(
    {
      conversationId: item.conversationId,
      kind: item.conversationKind === "remote_im_contact" ? "remote_im_contact" : "local_unarchived",
      title: item.title,
      summaryTitle: item.summaryTitle,
      remoteContactDisplayName: "",
      updatedAt: item.updatedAt,
      lastMessageAt: item.updatedAt,
      isSystemNotificationConversation: item.isSystemNotificationConversation,
    },
    {
      locale: "zh-CN",
      untitledLabel: "未命名会话",
    },
  );
}

function conversationAvatarUrl(item: UsageConversationItem): string {
  return resolveAvatarUrl(item.avatarPath, item.avatarUpdatedAt);
}

function conversationAvatarLabel(item: UsageConversationItem): string {
  return String(item.agentName || displayConversationTitle(item) || "会话头像").trim() || "会话头像";
}

function conversationAvatarInitial(item: UsageConversationItem): string {
  const text = conversationAvatarLabel(item);
  return text.charAt(0).toUpperCase() || "?";
}

async function loadOverview() {
  loading.value = true;
  errorText.value = "";
  try {
    overview.value = await invokeTauri<UsageOverview>("get_usage_overview");
    conversationPage.value = 1;
  } catch (error) {
    errorText.value = toErrorMessage(error);
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  void loadOverview();
});
</script>
