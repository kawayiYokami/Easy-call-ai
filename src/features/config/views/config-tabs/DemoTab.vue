<template>
  <div class="grid gap-3">
    <div class="card border border-base-300 bg-base-100">
      <div class="card-body gap-3 p-4">
        <div class="space-y-1">
          <h3 class="card-title text-base">{{ t("config.demo.nativeNotificationTitle") }}</h3>
          <p class="text-sm text-base-content/70">
            {{ t("config.demo.nativeNotificationSummary") }}
          </p>
          <p class="text-xs text-base-content/60">
            {{ t("config.demo.nativeNotificationDevHint") }}
          </p>
        </div>

        <div class="flex flex-wrap items-center gap-3">
          <button
            type="button"
            class="btn btn-primary"
            :disabled="sending"
            @click="sendNativeNotification"
          >
            {{ sending ? t("config.demo.sending") : t("config.demo.sendNativeNotification") }}
          </button>
          <span class="text-xs text-base-content/60">{{ t("config.demo.backgroundHint") }}</span>
        </div>

        <div v-if="errorText" class="alert alert-error text-sm">
          <span>{{ errorText }}</span>
        </div>

        <div v-else-if="resultText" class="alert alert-success text-sm whitespace-pre-wrap">
          <span>{{ resultText }}</span>
        </div>
      </div>
    </div>

    <div class="card border border-base-300 bg-base-100">
      <div class="card-body gap-3 p-4">
        <div class="space-y-1">
          <h3 class="card-title text-base">{{ t("config.demo.restartTitle") }}</h3>
          <p class="text-sm text-base-content/70">
            {{ t("config.demo.restartSummary") }}
          </p>
        </div>

        <div class="flex flex-wrap items-center gap-3">
          <button
            type="button"
            class="btn btn-warning"
            :disabled="restarting"
            @click="restartApp"
          >
            <RotateCcw class="size-4" aria-hidden="true" />
            {{ restarting ? t("config.demo.restarting") : t("config.demo.restartApp") }}
          </button>
        </div>
      </div>
    </div>

    <div class="card border border-base-300 bg-base-100">
      <div class="card-body gap-3 p-4">
        <div class="space-y-1">
          <h3 class="card-title text-base">DelegateProgressLine 预览</h3>
          <p class="text-sm text-base-content/70">折叠卡片第二行的实时进度组件样本。</p>
        </div>
        <div class="flex flex-col gap-3 py-2">
          <details class="collapse collapse-arrow w-full rounded-box border border-base-300 bg-base-200">
            <summary class="collapse-title min-h-0 px-3 py-3 pr-10">
              <div class="flex items-center justify-between gap-3">
                <div class="min-w-0 flex items-center gap-2">
                  <div class="truncate text-sm">示例：代码审查（pending）</div>
                </div>
                <div class="badge badge-sm min-w-14 shrink-0 justify-center whitespace-nowrap badge-warning">生成中</div>
              </div>
              <DelegateProgressLine :running="true" :elapsed-ms="45000" :request-count="12" :token-count="15600" last-tool-name="apply_patch" />
            </summary>
            <div class="collapse-content flex flex-col gap-3 px-3 pb-3">
              <div class="whitespace-pre-wrap wrap-break-word text-sm leading-7 text-base-content/75">
                展开后的详情内容…
              </div>
            </div>
          </details>

          <details class="collapse collapse-arrow w-full rounded-box border border-base-300 bg-base-200">
            <summary class="collapse-title min-h-0 px-3 py-3 pr-10">
              <div class="flex items-center justify-between gap-3">
                <div class="min-w-0 flex items-center gap-2">
                  <div class="truncate text-sm">示例：委托任务（运行中）</div>
                </div>
                <div class="badge badge-sm min-w-14 shrink-0 justify-center whitespace-nowrap badge-warning">执行中</div>
              </div>
              <DelegateProgressLine :running="true" :elapsed-ms="120000" :request-count="34" :token-count="52800" last-tool-name="shell_exec" />
            </summary>
            <div class="collapse-content flex flex-col gap-3 px-3 pb-3">
              <div class="whitespace-pre-wrap wrap-break-word text-sm leading-7 text-base-content/75">
                展开后的详情内容…
              </div>
            </div>
          </details>

          <details class="collapse collapse-arrow w-full rounded-box border border-base-300 bg-base-200">
            <summary class="collapse-title min-h-0 px-3 py-3 pr-10">
              <div class="flex items-center justify-between gap-3">
                <div class="min-w-0 flex items-center gap-2">
                  <div class="truncate text-sm">示例：审查报告（完成）</div>
                </div>
                <div class="badge badge-sm min-w-14 shrink-0 justify-center whitespace-nowrap badge-primary">成功</div>
              </div>
              <DelegateProgressLine text="整体判定：正确，置信度 0.92" />
            </summary>
            <div class="collapse-content flex flex-col gap-3 px-3 pb-3">
              <div class="whitespace-pre-wrap wrap-break-word text-sm leading-7 text-base-content/75">
                展开后的详情内容…
              </div>
            </div>
          </details>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { RotateCcw } from "@lucide/vue";
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { invokeTauri } from "../../../../services/tauri-api";
import DelegateProgressLine from "../../../chat/components/DelegateProgressLine.vue";

type NativeNotificationDemoResult = {
  permissionBefore: string;
  permissionAfter: string;
  title: string;
  body: string;
  sentAt: string;
};

const sending = ref(false);
const restarting = ref(false);
const errorText = ref("");
const resultText = ref("");
const { t } = useI18n();

async function sendNativeNotification() {
  sending.value = true;
  errorText.value = "";
  resultText.value = "";

  try {
    const result = await invokeTauri<NativeNotificationDemoResult>("demo_send_native_notification");
    resultText.value = [
      t("config.demo.nativeNotificationSent"),
      `title: ${result.title}`,
      `permissionBefore: ${result.permissionBefore}`,
      `permissionAfter: ${result.permissionAfter}`,
      `sentAt: ${result.sentAt}`,
    ].join("\n");
  } catch (error) {
    errorText.value = error instanceof Error ? error.message : String(error);
  } finally {
    sending.value = false;
  }
}

async function restartApp() {
  restarting.value = true;
  errorText.value = "";
  resultText.value = "";

  try {
    await invokeTauri<void>("demo_restart_app");
    resultText.value = t("config.demo.restartRequested");
  } catch (error) {
    errorText.value = error instanceof Error ? error.message : String(error);
    restarting.value = false;
  }
}
</script>
