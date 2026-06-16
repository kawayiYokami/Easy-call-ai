<template>
  <div class="space-y-5">
    <div class="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
      <div>
        <div class="text-xl font-semibold">用量概览</div>
        <div class="mt-1 text-sm opacity-70">统计所有会话累计的缓存写入、输出、缓存读写，以及按会话、人格、配置、类型拆分的用量。</div>
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
      <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        <div v-for="card in summaryCards" :key="card.label" class="card border border-base-300 bg-base-100 shadow-sm">
          <div class="card-body p-4">
            <div class="text-xs opacity-60">{{ card.label }}</div>
            <div class="mt-1 text-2xl font-semibold">{{ card.value }}</div>
            <div v-if="card.hint" class="mt-1 text-xs opacity-60">{{ card.hint }}</div>
          </div>
        </div>
      </div>

      <div class="grid gap-4 xl:grid-cols-2">
        <section class="card border border-base-300 bg-base-100 shadow-sm">
          <div class="card-body p-4">
            <div class="card-title text-base">分人格用量</div>
            <div class="mt-2 overflow-x-auto">
              <table class="table table-sm">
                <thead>
                  <tr>
                    <th>人格</th>
                    <th class="text-right">总量</th>
                    <th class="text-right">缓存写入</th>
                    <th class="text-right">输出</th>
                    <th class="text-right">缓存读</th>
                    <th class="text-right">会话</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="item in topAgents" :key="`agent-${item.key}`">
                    <td class="min-w-36">{{ item.label }}</td>
                    <td class="text-right">{{ formatTokens(item.weightedTokens) }}</td>
                    <td class="text-right">{{ formatTokens(deriveCacheWriteTokens(item.inputTokens, item.cacheReadTokens)) }}</td>
                    <td class="text-right">{{ formatTokens(item.outputTokens) }}</td>
                    <td class="text-right">{{ formatTokens(item.cacheReadTokens) }}</td>
                    <td class="text-right">{{ item.conversationCount }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </section>

        <section class="card border border-base-300 bg-base-100 shadow-sm">
          <div class="card-body p-4">
            <div class="card-title text-base">分配置用量</div>
            <div class="mt-1 text-xs opacity-60">如果会话期间切换模型，统计会归最后使用的配置。</div>
            <div class="mt-2 overflow-x-auto">
              <table class="table table-sm">
                <thead>
                  <tr>
                    <th>配置</th>
                    <th class="text-right">总量</th>
                    <th class="text-right">缓存写入</th>
                    <th class="text-right">输出</th>
                    <th class="text-right">缓存读</th>
                    <th class="text-right">会话</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="item in topApiConfigs" :key="`api-${item.key}`">
                    <td class="min-w-36">{{ item.label }}</td>
                    <td class="text-right">{{ formatTokens(item.weightedTokens) }}</td>
                    <td class="text-right">{{ formatTokens(deriveCacheWriteTokens(item.inputTokens, item.cacheReadTokens)) }}</td>
                    <td class="text-right">{{ formatTokens(item.outputTokens) }}</td>
                    <td class="text-right">{{ formatTokens(item.cacheReadTokens) }}</td>
                    <td class="text-right">{{ item.conversationCount }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </section>

        <section class="card border border-base-300 bg-base-100 shadow-sm">
          <div class="card-body p-4">
            <div class="card-title text-base">分会话类型用量</div>
            <div class="mt-2 overflow-x-auto">
              <table class="table table-sm">
                <thead>
                  <tr>
                    <th>类型</th>
                    <th class="text-right">总量</th>
                    <th class="text-right">缓存写入</th>
                    <th class="text-right">输出</th>
                    <th class="text-right">缓存读</th>
                    <th class="text-right">会话</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="item in overview.byKind" :key="`kind-${item.key}`">
                    <td>{{ item.label }}</td>
                    <td class="text-right">{{ formatTokens(item.weightedTokens) }}</td>
                    <td class="text-right">{{ formatTokens(deriveCacheWriteTokens(item.inputTokens, item.cacheReadTokens)) }}</td>
                    <td class="text-right">{{ formatTokens(item.outputTokens) }}</td>
                    <td class="text-right">{{ formatTokens(item.cacheReadTokens) }}</td>
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
          <div class="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
            <div>
              <div class="card-title text-base">会话明细</div>
              <div class="text-xs opacity-60">按综合总量排序，方便直接定位最耗的会话。</div>
            </div>
            <div class="flex items-center gap-3 text-xs opacity-60">
              <div>共 {{ overview.conversations.length }} 条</div>
              <div>生成时间：{{ formatDateTime(overview.generatedAt) }}</div>
            </div>
          </div>
          <div class="mt-3 overflow-x-auto rounded-box border border-base-300">
            <table class="table table-sm">
              <thead>
                <tr>
                  <th>会话</th>
                  <th>人格</th>
                  <th>类型</th>
                  <th class="text-right">总量</th>
                  <th class="text-right">缓存写入</th>
                  <th class="text-right">输出</th>
                  <th class="text-right">缓存读</th>
                  <th class="text-right">缓存写</th>
                  <th class="text-right">消息</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="item in pagedConversations" :key="item.conversationId">
                  <td class="min-w-64">
                    <div class="font-medium">{{ item.title || item.conversationId }}</div>
                    <div class="text-[11px] opacity-60">{{ item.conversationId }}</div>
                    <div class="text-[11px] opacity-60">{{ formatConversationMeta(item) }}</div>
                  </td>
                  <td class="min-w-28">{{ item.agentName }}</td>
                  <td>{{ conversationKindLabel(item) }}</td>
                  <td class="text-right font-medium">{{ formatTokens(item.weightedTokens) }}</td>
                  <td class="text-right">{{ formatTokens(deriveCacheWriteTokens(item.inputTokens, item.cacheReadTokens)) }}</td>
                  <td class="text-right">{{ formatTokens(item.outputTokens) }}</td>
                  <td class="text-right">{{ formatTokens(item.cacheReadTokens) }}</td>
                  <td class="text-right">{{ formatTokens(item.cacheWriteTokens) }}</td>
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
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invokeTauri } from "../../../../services/tauri-api";
import type { UsageOverview, UsageConversationItem } from "../../../../types/app";
import { toErrorMessage } from "../../../../utils/error";

const { t } = useI18n();

const loading = ref(false);
const errorText = ref("");
const overview = ref<UsageOverview | null>(null);
const conversationPage = ref(1);
const conversationPageSize = 20;

const summaryCards = computed(() => {
  if (!overview.value) return [];
  const totals = overview.value.totals;
  return [
    { label: "综合总量", value: formatTokens(totals.weightedTokens), hint: `缓存写入 ${formatTokens(deriveCacheWriteTokens(totals.inputTokens, totals.cacheReadTokens))} / 输出 ${formatTokens(totals.outputTokens)}` },
    { label: "缓存命中", value: formatTokens(totals.cacheReadTokens), hint: `缓存写入 ${formatTokens(totals.cacheWriteTokens)}` },
  ];
});
const topAgents = computed(() => (overview.value?.byAgent || []).slice(0, 12));
const topApiConfigs = computed(() => (overview.value?.byApiConfig || []).slice(0, 12));
const conversationPageCount = computed(() => Math.max(1, Math.ceil((overview.value?.conversations.length || 0) / conversationPageSize)));
const pagedConversations = computed(() => {
  const items = overview.value?.conversations || [];
  const start = (conversationPage.value - 1) * conversationPageSize;
  return items.slice(start, start + conversationPageSize);
});
const conversationPageStart = computed(() => {
  const total = overview.value?.conversations.length || 0;
  if (total === 0) return 0;
  return (conversationPage.value - 1) * conversationPageSize + 1;
});
const conversationPageEnd = computed(() => {
  const total = overview.value?.conversations.length || 0;
  return Math.min(total, conversationPage.value * conversationPageSize);
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

function deriveCacheWriteTokens(inputTokens: number, cacheReadTokens: number): number {
  return Math.max(0, Number(inputTokens || 0) - Number(cacheReadTokens || 0));
}

function formatDateTime(value?: string | null): string {
  const text = String(value || "").trim();
  if (!text) return "-";
  const date = new Date(text);
  if (Number.isNaN(date.getTime())) return text;
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")} ${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
}

function conversationKindLabel(item: UsageConversationItem): string {
  if (item.isSystemNotificationConversation) return "系统通知";
  if (item.isDelegate) return "委托";
  if (item.conversationKind === "remote_im_contact") return "远程联系人";
  if (item.archivedAt) return "已归档";
  return "普通";
}

function formatConversationMeta(item: UsageConversationItem): string {
  const parts = [
    item.departmentName ? `部门: ${item.departmentName}` : "",
    item.updatedAt ? `更新: ${formatDateTime(item.updatedAt)}` : "",
    item.archivedAt ? `归档: ${formatDateTime(item.archivedAt)}` : "",
  ].filter(Boolean);
  return parts.join(" | ");
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
