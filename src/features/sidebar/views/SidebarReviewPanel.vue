<template>
  <div
    v-if="open"
    class="absolute inset-0 z-50 flex"
  >
    <div class="flex-1 bg-base-content/20" @click="emit('close')"></div>
    <div class="flex h-full w-80 min-w-0 flex-col border-l border-base-300 bg-base-100 shadow-lg">
      <div class="flex h-10 shrink-0 items-center justify-between border-b border-base-300 px-3">
        <span class="text-sm font-semibold">审查</span>
        <button
          type="button"
          class="btn btn-ghost btn-sm h-7 min-h-7 w-7 px-0"
          @click="emit('close')"
        >
          <X class="h-4 w-4" />
        </button>
      </div>

      <div class="border-b border-base-300 px-3 py-3">
        <button
          type="button"
          class="btn btn-sm w-full"
          :disabled="submitting"
          @click="emit('openCodeReview')"
        >
          <span v-if="submitting" class="loading loading-spinner loading-xs"></span>
          {{ t("chat.toolReview.generateReviewReport") }}
        </button>
      </div>

      <div v-if="errorText" class="mx-3 my-3 rounded-box border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
        {{ errorText }}
      </div>

      <div v-if="loading && reports.length === 0" class="flex flex-1 items-center justify-center text-sm text-base-content/65">
        <span class="loading loading-spinner loading-sm mr-2"></span>
        加载中
      </div>

      <div v-else-if="reports.length === 0" class="flex flex-1 items-center justify-center px-4 text-center text-sm text-base-content/65">
        暂无审查报告
      </div>

      <div v-else class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto py-2 px-3">
        <section v-for="report in reports" :key="report.id">
          <details class="collapse collapse-arrow w-full rounded-box border border-base-300 bg-base-200">
            <summary class="collapse-title min-h-0 px-3 py-3 pr-10">
              <div class="flex items-center justify-between gap-3">
                <div class="min-w-0 flex items-center gap-2">
                  <div class="truncate text-sm">{{ report.title || report.target || formatReportScope(report.scope) }}</div>
                </div>
                <div class="badge badge-sm min-w-14 shrink-0 justify-center whitespace-nowrap" :class="reportStatusBadgeClass(report.status)">
                  {{ formatReportStatus(report.status) }}
                </div>
              </div>
              <div class="mt-1 text-xs text-base-content/65">
                {{ reportJudgementSummary(report) }}
              </div>
            </summary>
            <div class="collapse-content flex flex-col gap-3 px-3 pb-3">
              <div class="whitespace-pre-wrap wrap-break-word text-sm leading-7 text-base-content/75">
                {{ reportExpandedText(report) }}
              </div>
              <div class="flex items-center justify-between gap-3">
                <button
                  type="button"
                  class="btn btn-sm gap-1.5 border-base-300 bg-base-100 font-normal hover:bg-base-100"
                  :disabled="deleting"
                  @click.prevent.stop="emit('deleteReport', report)"
                >删除</button>
                <button
                  v-if="canRetryReport(report)"
                  type="button"
                  class="btn btn-sm gap-1.5 border-base-300 bg-base-100 font-normal hover:bg-base-100"
                  :disabled="submitting"
                  @click.prevent.stop="emit('retryReport', report)"
                >重新生成</button>
              </div>
            </div>
          </details>
        </section>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { X } from "@lucide/vue";
import type { ToolReviewReportRecord } from "../../chat/composables/use-chat-tool-review";

defineProps<{
  open: boolean;
  loading: boolean;
  submitting: boolean;
  deleting: boolean;
  errorText: string;
  reports: ToolReviewReportRecord[];
}>();

const emit = defineEmits<{
  close: [];
  openCodeReview: [];
  deleteReport: [report: ToolReviewReportRecord];
  retryReport: [report: ToolReviewReportRecord];
}>();

const { t } = useI18n();

function canRetryReport(report: ToolReviewReportRecord) {
  return report.status === "failed";
}

function formatReportStatus(status: string) {
  if (status === "success") return "已完成";
  if (status === "failed") return "生成失败";
  return "生成中";
}

function reportStatusBadgeClass(status: string) {
  if (status === "success") return "badge-primary";
  if (status === "failed") return "badge-error";
  return "badge-warning";
}

function formatReportScope(scope: string) {
  if (scope === "commit") return "commit";
  if (scope === "main") return "主分支差异";
  if (scope === "uncommitted") return "未提交改动";
  if (scope === "custom") return "自定义";
  return scope || "未知范围";
}

function reportJudgementSummary(report: ToolReviewReportRecord) {
  if (report.status === "pending") return "生成中...";
  if (report.status === "failed") return report.errorText || "生成失败";
  const parsed = parseToolReviewJson(report.reportText);
  if (parsed) {
    const judgement = formatJsonCorrectness(stringField(parsed.raw.overall_correctness) || "未知判定");
    const confidence = numberField(parsed.raw.overall_confidence_score);
    return confidence === null ? judgement : `${judgement} · 置信度 ${confidence.toFixed(2)}`;
  }
  const judgement = reportMarkdownField(report, "整体判定") || "未知判定";
  const confidence = reportMarkdownField(report, "整体置信度");
  return confidence ? `${judgement} · 置信度 ${confidence}` : judgement;
}

function reportExpandedText(report: ToolReviewReportRecord) {
  if (report.status === "pending") return "生成中";
  if (report.status === "failed") return report.errorText || "生成失败";
  const parsed = parseToolReviewJson(report.reportText);
  if (parsed) return stringField(parsed.raw.overall_explanation) || report.reportText || t("chat.toolReview.noReportContent");
  return reportMarkdownField(report, "判定说明") || report.reportText || t("chat.toolReview.noReportContent");
}

function reportMarkdownField(report: ToolReviewReportRecord, label: string) {
  const text = String(report.reportText || "");
  const escapedLabel = label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const matched = new RegExp(`^-\\s*${escapedLabel}：\\s*(.+)$`, "m").exec(text);
  return String(matched?.[1] || "").trim();
}

type ParsedToolReviewJson = {
  raw: Record<string, unknown>;
  findings?: Array<Record<string, unknown>>;
};

function parseToolReviewJson(text: string): ParsedToolReviewJson | null {
  const trimmed = String(text || "").trim();
  if (!trimmed) return null;
  try {
    const raw = JSON.parse(trimmed) as Record<string, unknown>;
    if (typeof raw !== "object" || !raw) return null;
    const findings = Array.isArray(raw.findings) ? raw.findings as Array<Record<string, unknown>> : undefined;
    return { raw, findings };
  } catch {
    return null;
  }
}

function stringField(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function numberField(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function formatJsonCorrectness(raw: string) {
  const lower = String(raw || "").toLowerCase().trim();
  if (lower === "correct" || lower === "yes") return "代码正确";
  if (lower === "incorrect" || lower === "no") return "代码有误";
  if (lower === "partially_correct" || lower === "partial") return "部分正确";
  return raw || "未知判定";
}
</script>
