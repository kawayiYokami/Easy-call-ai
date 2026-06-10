<template>
  <ConfigWindowApp v-if="ready" />
  <div v-else class="flex min-h-screen items-center justify-center bg-base-200 px-4 text-base-content">
    <div class="w-full max-w-sm rounded-box border border-base-300 bg-base-100 p-5 shadow-xl">
      <div class="text-base font-semibold">P-ai 设置</div>
      <div class="mt-2 text-sm text-base-content/70">{{ statusText }}</div>
      <form v-if="authRequired" class="mt-4 flex flex-col gap-3" @submit.prevent="submitPassword">
        <input
          v-model.trim="password"
          class="input input-bordered input-sm w-full"
          type="password"
          autocomplete="current-password"
          placeholder="远程访问密码"
          :disabled="submitting"
        />
        <button class="btn btn-sm btn-primary w-full" type="submit" :disabled="submitting || !password">
          <span v-if="submitting" class="loading loading-spinner loading-xs"></span>
          进入设置
        </button>
      </form>
      <button v-else class="btn btn-sm btn-primary mt-4 w-full" type="button" :disabled="connecting" @click="initialize">
        <span v-if="connecting" class="loading loading-spinner loading-xs"></span>
        重试连接
      </button>
      <div v-if="errorText" class="mt-3 text-xs text-error">{{ errorText }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import ConfigWindowApp from "../../ConfigWindowApp.vue";
import { connectWebBridge, getWebBridgeState, isTauriRuntimeAvailable, loginWebBridge } from "../../services/tauri-api";

const ready = ref(isTauriRuntimeAvailable());
const connecting = ref(false);
const submitting = ref(false);
const authRequired = ref(false);
const password = ref("");
const errorText = ref("");
const statusText = ref("正在连接 PAI...");

function applyBridgeState() {
  const state = getWebBridgeState();
  authRequired.value = state.authRequired && !state.authenticated;
  if (state.connected && authRequired.value) {
    statusText.value = "请输入远程访问密码。";
  } else if (state.connected) {
    statusText.value = "连接成功，正在加载设置...";
  } else {
    statusText.value = state.errorText || "PAI 未运行。";
  }
}

async function initialize() {
  if (ready.value || isTauriRuntimeAvailable()) {
    ready.value = true;
    return;
  }
  connecting.value = true;
  errorText.value = "";
  statusText.value = "正在连接 PAI...";
  try {
    const state = await connectWebBridge();
    applyBridgeState();
    ready.value = !state.authRequired || state.authenticated;
  } catch (error) {
    applyBridgeState();
    errorText.value = String(error || "连接失败");
  } finally {
    connecting.value = false;
  }
}

async function submitPassword() {
  if (!password.value || submitting.value) return;
  submitting.value = true;
  errorText.value = "";
  try {
    const state = await loginWebBridge(password.value);
    applyBridgeState();
    ready.value = !state.authRequired || state.authenticated;
    password.value = "";
  } catch (error) {
    errorText.value = String(error || "认证失败");
  } finally {
    submitting.value = false;
  }
}

onMounted(() => {
  void initialize();
});
</script>
